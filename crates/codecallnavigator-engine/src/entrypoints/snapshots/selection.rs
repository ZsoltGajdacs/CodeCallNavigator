//! Resolve snapshot selections within an owned checkout before any analyzer runs.
use crate::entrypoints::analysis;
use anyhow::{Context, Result, ensure};
use codecallnavigator_core::{
    discovery::{self, BuildTarget},
    protocol::SnapshotTarget,
};
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

// Keep the existing lease key encoding for legacy path requests.
#[derive(Serialize)]
#[serde(untagged)]
pub(in crate::entrypoints) enum Selection {
    Paths(Vec<PathBuf>),
    Exact(Vec<SnapshotTarget>),
}

impl Selection {
    pub(in crate::entrypoints) fn from_request(
        paths: Vec<PathBuf>,
        exact: Option<Vec<SnapshotTarget>>,
    ) -> Result<Self> {
        let Some(exact) = exact else {
            return Ok(Self::Paths(paths));
        };
        ensure!(
            paths.is_empty(),
            "Cannot combine snapshot paths and exact targets"
        );
        Ok(Self::Exact(exact))
    }

    pub(super) fn normalize(self) -> Result<Self> {
        match self {
            Self::Paths(mut paths) => {
                for path in &paths {
                    require_relative(path)?;
                }
                paths.sort();
                paths.dedup();
                Ok(Self::Paths(paths))
            }
            Self::Exact(mut targets) => {
                ensure!(
                    !targets.is_empty(),
                    "Select at least one exact snapshot target"
                );
                for target in &mut targets {
                    target.path = normalize_exact_path(&target.path)?;
                }
                targets.sort();
                targets.dedup();
                Ok(Self::Exact(targets))
            }
        }
    }

    pub(super) fn resolve(self, checkout: &Path, revision: &str) -> Result<Vec<BuildTarget>> {
        match self {
            Self::Paths(paths) => {
                for path in &paths {
                    ensure!(
                        checkout.join(path).canonicalize()?.starts_with(checkout),
                        "Snapshot target resolves outside its project"
                    );
                }
                let selected = analysis::select_targets(checkout, paths)?;
                ensure!(!selected.is_empty(), "No analysis targets in this revision");
                Ok(selected)
            }
            Self::Exact(targets) => targets
                .into_iter()
                .map(|target| {
                    resolve_exact(checkout, &target).with_context(|| {
                        format!(
                            "Cannot resolve selected {} target {} in revision {revision}",
                            target.kind,
                            target.path.display()
                        )
                    })
                })
                .collect(),
        }
    }
}

fn require_relative(path: &Path) -> Result<()> {
    ensure!(
        path.components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir)),
        "Snapshot targets must be project-relative paths without parent traversal"
    );
    Ok(())
}

fn normalize_exact_path(path: &Path) -> Result<PathBuf> {
    require_relative(path).with_context(|| format!("Invalid exact target {}", path.display()))?;
    ensure!(
        !path.as_os_str().is_empty() && !path.as_os_str().as_encoded_bytes().contains(&0),
        "Exact snapshot paths must be nonempty and contain no NUL bytes"
    );
    let relative = path
        .components()
        .filter(|part| *part != Component::CurDir)
        .collect::<PathBuf>();
    Ok(if relative.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        relative
    })
}

fn resolve_exact(checkout: &Path, target: &SnapshotTarget) -> Result<BuildTarget> {
    let path = checkout.join(&target.path);
    let canonical = path.canonicalize().context("Target path is unavailable")?;
    ensure!(
        canonical == path,
        "Target path is redirected in the checkout"
    );
    discovery::rediscover_exact(&BuildTarget {
        path: canonical,
        kind: target.kind.clone(),
        language: target.language.clone(),
        label: String::new(),
    })
}
