use super::{identity, paths::Paths};
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
    }
    .visit(value)
}

struct Transform<'a> {
    paths: &'a Paths,
    identities: Option<&'a HashMap<String, String>>,
    hydrate: bool,
}

impl Transform<'_> {
    fn visit(&self, value: &mut Value) -> Result<()> {
        match value {
            Value::Object(fields) => {
                for (key, value) in fields {
                    self.field(key, value)?;
                }
            }
            Value::Array(values) => {
                for value in values {
                    self.visit(value)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn field(&self, key: &str, value: &mut Value) -> Result<()> {
        match key {
            "id" | "symbol" => self.symbol(value),
            "file" | "build" => self.paths.rewrite(value, self.hydrate),
            "project" => {
                self.paths.project(value, self.hydrate);
                Ok(())
            }
            "roots" => self.roots(value),
            "candidates" | "application_starts" | "framework_entries" | "supporting_symbols" => {
                self.entries(value)
            }
            "diagnostics" if !self.hydrate => self.diagnostics(value),
            _ => self.visit(value),
        }
    }

    fn symbol(&self, value: &mut Value) -> Result<()> {
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
            | "roots"
            | "diagnostics"
            | "functions"
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
