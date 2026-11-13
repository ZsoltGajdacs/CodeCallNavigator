//! Portable input provenance. Runtime timestamps and machine paths never enter this contract.
use super::{Roots, paths::Paths};
use crate::{fingerprint::InputKind, model::FileFingerprint};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};

/// Complete evaluated C# import observations, independent of MSBuild input mtimes.
pub const CSHARP_INPUT_POLICY: &str = "msbuild-preprocess.v2";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Analysis {
    pub version: u32,
    pub targets: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "selection_value"
    )]
    pub selection: Option<Value>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "selection_value"
    )]
    pub refresh_inputs: Option<Value>,
    pub roots: Vec<String>,
    pub engine_versions: String,
    pub inputs: Vec<Input>,
    pub limitations: Vec<Limitation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub path: String,
    pub kind: InputKind,
    pub hash: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_children: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Limitation {
    pub code: String,
    pub count: usize,
}

/// Use hashes captured during analysis, never substitute newer source contents.
/// The coordinator must still reject changed runtime fingerprints before publication.
pub fn build(
    fingerprints: &[FileFingerprint],
    targets: &[String],
    engine_versions: &str,
    roots: &Roots,
) -> Result<Analysis> {
    let paths = Paths::new(roots)?;
    let mut analysis = Analysis {
        version: 1,
        targets: Vec::new(),
        selection: None,
        refresh_inputs: None,
        roots: roots.iter().map(|(name, _)| name.clone()).collect(),
        engine_versions: portable_versions(engine_versions, &paths)?,
        inputs: Vec::new(),
        limitations: Vec::new(),
    };
    analysis.targets = targets
        .iter()
        .filter_map(|target| paths.portable(target).ok())
        .collect();
    analysis.inputs = fingerprints
        .iter()
        .filter_map(|input| portable_input(input, &paths))
        .collect();
    let unmapped =
        targets.len() - analysis.targets.len() + fingerprints.len() - analysis.inputs.len();
    if unmapped > 0 {
        analysis.limitations.push(Limitation {
            code: "unmapped_input".into(),
            count: unmapped,
        });
    }
    if fingerprints.is_empty() {
        analysis.limitations.push(Limitation {
            code: "no_observed_inputs".into(),
            count: 1,
        });
    }
    analysis.targets.sort();
    analysis.targets.dedup();
    analysis.roots.sort();
    analysis.inputs.sort_by(|a, b| a.path.cmp(&b.path));
    // Overlapping build selections may observe an identical input more than once.
    analysis.inputs.dedup();
    analysis.validate()?;
    Ok(analysis)
}

fn selection_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

/// Preserve exact identities before analysis consumes its selected build targets.
pub fn build_selected(
    fingerprints: &[FileFingerprint],
    targets: &[crate::discovery::BuildTarget],
    engine_versions: &str,
    roots: &Roots,
) -> Result<Analysis> {
    let legacy = targets
        .iter()
        .map(|target| target.path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let mut analysis = build(fingerprints, &legacy, engine_versions, roots)?;
    analysis.selection = Some(serde_json::to_value(super::selection::capture(
        targets, roots,
    )?)?);
    analysis.validate()?;
    Ok(analysis)
}

fn portable_input(fingerprint: &FileFingerprint, paths: &Paths) -> Option<Input> {
    let path = paths.portable(&fingerprint.path).ok()?;
    let kind = if fingerprint.missing {
        InputKind::Missing
    } else {
        let metadata = std::fs::metadata(&fingerprint.path).ok()?;
        if metadata.is_dir() {
            InputKind::Directory
        } else {
            InputKind::File
        }
    };
    let mut excluded_children = fingerprint.excluded_children.clone();
    excluded_children.sort();
    excluded_children.dedup();
    Some(Input {
        path,
        kind,
        hash: fingerprint.hash.clone(),
        excluded_children,
    })
}
impl Input {
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            valid_content_hash(self.kind, &self.hash),
            "Invalid analysis content hash"
        );
        for child in &self.excluded_children {
            ensure!(
                !child.is_empty()
                    && Path::new(child).components().count() == 1
                    && !matches!(child.as_str(), "." | "..")
                    && !child.contains(['/', '\\']),
                "Invalid analysis exclusion"
            );
        }
        Ok(())
    }
}

/// Missing observations have no digest; other observations use lowercase BLAKE3.
fn valid_content_hash(kind: InputKind, hash: &str) -> bool {
    match kind {
        InputKind::Missing => hash.is_empty(),
        _ => {
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        }
    }
}

pub fn read(graph: &Value) -> Result<Option<Analysis>> {
    graph
        .get("analysis")
        .filter(|value| !value.is_null())
        .map(|value| {
            let analysis = Analysis::deserialize(value).context("Invalid analysis provenance")?;
            analysis.validate()?;
            Ok(analysis)
        })
        .transpose()
}

pub fn validate_analysis(graph: &Value) -> Result<()> {
    read(graph).map(|_| ())
}

impl Analysis {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.version == 1, "Unsupported analysis provenance version");
        if let Some(selection) = &self.selection {
            super::selection::validate_owned(selection, &self.roots)?;
        }
        if let Some(refresh) = &self.refresh_inputs {
            super::refresh_inputs::validate(refresh, self)?;
        }
        let roots: BTreeSet<_> = self.roots.iter().collect();
        ensure!(roots.len() == self.roots.len(), "Duplicate analysis root");
        for root in &roots {
            ensure!(!root.contains('/'), "Invalid analysis root");
            super::paths::validate_field("file", &Value::String(format!("@{root}/")))?;
        }
        for path in self
            .targets
            .iter()
            .chain(self.inputs.iter().map(|input| &input.path))
        {
            super::paths::validate_field("file", &Value::String(path.clone()))?;
            let name = path
                .strip_prefix('@')
                .and_then(|s| s.split_once('/'))
                .context("Invalid analysis input path")?
                .0;
            ensure!(
                roots.contains(&name.to_string()),
                "Analysis input references undeclared root"
            );
        }
        for input in &self.inputs {
            input.validate()?;
        }
        for limitation in &self.limitations {
            ensure!(
                !limitation.code.is_empty() && limitation.count > 0,
                "Invalid analysis limitation"
            );
        }
        Ok(())
    }
}

/// Canonicalize only contract-owned arrays, leaving extension payloads untouched.
pub(super) fn sort_analysis(analysis: &mut Value) -> Result<()> {
    if analysis.is_null() {
        return Ok(());
    }
    if let Some(selection) = analysis.get_mut("selection") {
        super::selection::normalize(selection);
    }
    if let Some(refresh) = analysis.get_mut("refresh_inputs") {
        super::refresh_inputs::normalize(refresh);
    }
    if let Some(inputs) = analysis.get_mut("inputs").and_then(Value::as_array_mut) {
        for input in inputs {
            if let Some(children) = input
                .get_mut("excluded_children")
                .and_then(Value::as_array_mut)
            {
                children.sort_by_cached_key(Value::to_string);
            }
        }
    }
    for field in ["targets", "roots", "inputs", "limitations"] {
        let values = analysis
            .get_mut(field)
            .and_then(Value::as_array_mut)
            .context("Analysis collection must be an array")?;
        values.sort_by_cached_key(Value::to_string);
    }
    Ok(())
}

// Runtime version reports also contain compiler context paths and classpath hashes.
// Persist version identifiers, never those machine-specific observations.
fn portable_versions(text: &str, paths: &Paths) -> Result<String> {
    if text.is_empty() {
        return Ok(String::new());
    }
    let mut value: Value = serde_json::from_str(text).context("Invalid runtime version report")?;
    sanitize_versions(&mut value, paths);
    Ok(value.to_string())
}
fn sanitize_versions(value: &mut Value, paths: &Paths) {
    match value {
        Value::Object(fields) => {
            let old = std::mem::take(fields);
            for (key, mut value) in old {
                if matches!(key.as_str(), "compile_contexts" | "classpath_order_hash") {
                    continue;
                }
                let Some(key) = portable_version_key(&key, paths) else {
                    continue;
                };
                sanitize_versions(&mut value, paths);
                fields.insert(key, value);
            }
        }
        Value::Array(items) => {
            for item in items {
                sanitize_versions(item, paths);
            }
        }
        _ => {}
    }
}

/// Version reports key per-project values by either a build path or package:path.
fn portable_version_key(key: &str, paths: &Paths) -> Option<String> {
    let (prefix, path) = match key.split_once(':') {
        Some((prefix, suffix)) if Path::new(suffix).is_absolute() => (prefix, suffix),
        _ if Path::new(key).is_absolute() => ("", key),
        _ => return Some(key.into()),
    };
    let portable = paths.portable(path).ok()?;
    Some(if prefix.is_empty() {
        portable
    } else {
        format!("{prefix}:{portable}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn null_analysis_is_legacy_absence_and_stays_null_on_serialization() {
        let mut artifact = super::super::canonical(
            &crate::model::GraphData::default(),
            &[("repo".into(), "/tmp".into())],
        )
        .unwrap();
        artifact.graph["analysis"] = Value::Null;
        assert!(read(&artifact.graph).unwrap().is_none());
        let bytes = artifact.bytes().unwrap();
        let report =
            super::super::verification::verify_bytes(&bytes, &[("repo".into(), "/tmp".into())])
                .unwrap();
        assert_eq!(
            report.input_status,
            super::super::verification::InputStatus::Unverifiable
        );
        assert!(serde_json::from_slice::<Value>(&bytes).unwrap()["graph"]["analysis"].is_null());
    }
    #[test]
    fn runtime_version_paths_are_portable_and_compile_observations_are_omitted() {
        let versions = serde_json::json!({"callatlas":{"revision":"abc"},"projects":{"/checkout/pom.xml":{"compile_contexts":[{"classpath":["/home/cache/a.jar"]}],"classpath_order_hash":"unstable", "java_context_protocol":1},"/checkout/tsconfig.json":{"react:/checkout/tsconfig.json":"19"}}}).to_string();
        let analysis = build(
            &[],
            &["/checkout/pom.xml".into()],
            &versions,
            &[("repo".into(), "/checkout".into())],
        )
        .unwrap();
        assert!(!analysis.engine_versions.contains("/checkout"));
        assert!(!analysis.engine_versions.contains("/home"));
        assert!(!analysis.engine_versions.contains("unstable"));
        assert!(
            analysis
                .engine_versions
                .contains("react:@repo/tsconfig.json")
        );
    }
}

#[cfg(test)]
#[path = "provenance_selection_tests.rs"]
mod selection_tests;
