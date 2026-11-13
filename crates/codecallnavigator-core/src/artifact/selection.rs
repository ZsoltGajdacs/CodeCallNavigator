//! Exact portable target selection; legacy path sets cannot reconstruct build kinds.
use super::{Roots, paths::Paths};
use crate::discovery::BuildTarget;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, path::PathBuf};

pub const VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Selection {
    pub version: u32,
    /// False when any selected target could not be mapped; never infer omissions.
    #[serde(default)]
    pub complete: bool,
    pub targets: Vec<Target>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Target {
    pub path: String,
    pub kind: String,
    pub language: String,
}

pub(super) fn capture(targets: &[BuildTarget], roots: &Roots) -> Result<Selection> {
    let paths = Paths::new(roots)?;
    let mut complete = true;
    let mut selected = BTreeSet::new();
    for target in targets {
        match paths.portable(&target.path.to_string_lossy()) {
            Ok(path) => {
                selected.insert(Target {
                    path,
                    kind: target.kind.clone(),
                    language: target.language.clone(),
                });
            }
            Err(_) => complete = false,
        }
    }
    Ok(Selection {
        version: VERSION,
        complete,
        targets: selected.into_iter().collect(),
    })
}

/// Decode only the owned version when a consumer requests repeatable analysis.
/// Basic graph loading keeps future selection payloads opaque.
pub fn resolve(value: Option<&Value>, roots: &Roots) -> Result<Vec<BuildTarget>> {
    let value =
        value.context("Exact target selection is unavailable; run an explicit manual refresh")?;
    ensure!(
        value["version"] == VERSION,
        "Unsupported automatic-refresh target selection version"
    );
    let selected: Selection =
        serde_json::from_value(value.clone()).context("Invalid exact target selection")?;
    ensure!(
        selected.complete && !selected.targets.is_empty(),
        "Exact target selection is incomplete or empty; run an explicit manual refresh"
    );
    let paths = Paths::new(roots)?;
    let mut identities = BTreeSet::new();
    selected
        .targets
        .into_iter()
        .map(|target| {
            ensure!(
                supported(&target),
                "Unsupported automatic-refresh target {}/{}",
                target.language,
                target.kind
            );
            ensure!(
                identities.insert(target.clone()),
                "Duplicate exact target selection"
            );
            super::paths::validate_field("file", &Value::String(target.path.clone()))?;
            let path = PathBuf::from(paths.local(&target.path)?);
            let label = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            Ok(BuildTarget {
                path,
                kind: target.kind,
                language: target.language,
                label,
            })
        })
        .collect()
}

fn supported(target: &Target) -> bool {
    matches!(
        (target.language.as_str(), target.kind.as_str()),
        ("csharp", "project" | "solution")
            | ("typescript", "typescript")
            | ("java", "maven" | "gradle")
    )
}

pub(super) fn validate_owned(value: &Value, roots: &[String]) -> Result<()> {
    if value["version"] != VERSION {
        return Ok(());
    }
    let selected: Selection =
        serde_json::from_value(value.clone()).context("Invalid selection v1")?;
    for target in selected.targets {
        super::paths::validate_field("file", &Value::String(target.path.clone()))?;
        let root = target
            .path
            .strip_prefix('@')
            .and_then(|path| path.split_once('/'))
            .context("Invalid selected target path")?
            .0;
        ensure!(
            roots.iter().any(|name| name == root),
            "Selected target references undeclared root"
        );
        ensure!(
            !target.kind.is_empty() && !target.language.is_empty(),
            "Selected target omitted its identity"
        );
    }
    Ok(())
}

pub(super) fn normalize(value: &mut Value) {
    if value["version"] != VERSION {
        return;
    }
    if let Some(targets) = value.get_mut("targets").and_then(Value::as_array_mut) {
        targets.sort_by_cached_key(Value::to_string);
    }
}
