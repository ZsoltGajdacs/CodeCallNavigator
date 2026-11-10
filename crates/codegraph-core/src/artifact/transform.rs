use super::{
    identity,
    paths::{Paths, ProjectOwner, SourceDisplay},
};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;

pub(super) fn transform(
    value: &mut Value,
    paths: &Paths,
    identities: Option<&HashMap<String, String>>,
    hydrate: bool,
) -> Result<()> {
    Transform {
        paths,
        identities,
        hydrate,
        folder_builds: value
            .get("folder_project_builds")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
    }
    .visit(value)
}

struct Transform<'a> {
    paths: &'a Paths,
    identities: Option<&'a HashMap<String, String>>,
    hydrate: bool,
    folder_builds: std::collections::HashSet<String>,
}

struct ObjectPaths {
    build: String,
    directory: bool,
    typescript: bool,
    display: Option<SourceDisplay>,
}
impl ObjectPaths {
    fn project_field(&self, key: &str) -> bool {
        match key {
            "project" => self.directory || self.typescript,
            _ => false,
        }
    }
}

impl Transform<'_> {
    fn visit(&self, value: &mut Value) -> Result<()> {
        match value {
            Value::Object(fields) => self.object(fields)?,
            Value::Array(values) => {
                for value in values {
                    self.visit(value)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn object(&self, fields: &mut serde_json::Map<String, Value>) -> Result<()> {
        let context = self.object_context(fields)?;
        for (key, value) in fields {
            self.object_field(&context, key, value)?;
        }
        Ok(())
    }
    fn object_context(&self, fields: &serde_json::Map<String, Value>) -> Result<ObjectPaths> {
        let build = fields
            .get("build")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let directory = self.folder_builds.contains(&build);
        let typescript = fields.get("language").and_then(Value::as_str) == Some("typescript");
        let display = if typescript {
            self.paths.source_display(
                fields
                    .get("file")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                &build,
                self.hydrate,
            )?
        } else {
            None
        };
        Ok(ObjectPaths {
            build,
            directory,
            typescript,
            display,
        })
    }
    fn object_field(&self, context: &ObjectPaths, key: &str, value: &mut Value) -> Result<()> {
        if let Some(display) = &context.display {
            display.rewrite(key, value);
        }
        if context.project_field(key) {
            self.paths.project(
                value,
                ProjectOwner {
                    build: &context.build,
                    directory: context.directory,
                },
                self.hydrate,
            )
        } else {
            self.field(key, value)
        }
    }

    fn field(&self, key: &str, value: &mut Value) -> Result<()> {
        match key {
            "qualifiers" => Ok(()),
            "id" | "symbol" => self.symbol(value),
            "file" | "build" => self.paths.rewrite(value, self.hydrate),
            "roots" | "folder_project_builds" | "identity_roots" => self.roots(value),
            "candidates" | "application_starts" | "framework_entries" | "supporting_symbols" => {
                self.entries(value)
            }
            "diagnostics" if !self.hydrate => self.diagnostics(value),
            _ => self.visit(value),
        }
    }

    fn symbol(&self, value: &mut Value) -> Result<()> {
        if value.is_null() {
            return Ok(());
        }
        if let Some(identities) = self.identities {
            identity::rewrite(value, identities)?;
        }
        Ok(())
    }

    fn roots(&self, value: &mut Value) -> Result<()> {
        for root in value.as_array_mut().context("Roots must be an array")? {
            self.paths.rewrite(root, self.hydrate)?;
        }
        Ok(())
    }

    fn entries(&self, value: &mut Value) -> Result<()> {
        for entry in value.as_array_mut().context("Entries must be an array")? {
            self.symbol(entry)?;
        }
        Ok(())
    }

    fn diagnostics(&self, value: &mut Value) -> Result<()> {
        for diagnostic in value
            .as_array_mut()
            .context("Diagnostics must be an array")?
        {
            let text = diagnostic.as_str().context("Diagnostic must be a string")?;
            *diagnostic = self.paths.embedded(text).into();
        }
        Ok(())
    }
}

pub(super) fn sort_sets(value: &mut Value, field: &str) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                sort_sets(value, key);
            }
        }
        Value::Array(values) => {
            for value in values.iter_mut() {
                sort_sets(value, "");
            }
            if unordered(field) {
                values.sort_by_cached_key(Value::to_string);
            }
        }
        _ => {}
    }
}

fn unordered(field: &str) -> bool {
    matches!(
        field,
        "nodes"
            | "evidence"
            | "supporting_symbols"
            | "edges"
            | "folder_project_builds"
            | "identity_roots"
            | "roots"
            | "diagnostics"
            | "functions"
            | "routes"
            | "uses"
            | "methods"
            | "restrictions"
            | "files"
            | "candidates"
            | "application_starts"
            | "framework_entries"
            | "targets"
            | "namespaces"
            | "names"
            | "kinds"
            | "annotations"
            | "method_annotations"
            | "base_types"
            | "annotated_interfaces"
    )
}
