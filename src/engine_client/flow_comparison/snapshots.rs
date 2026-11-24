//! Commit snapshot discovery and preparation use the public engine wire contract.
use super::Header;
use crate::engine_client::{Analysis, QueryContext};
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator::{
    artifact::local_snapshots::SnapshotRecord,
    protocol::{self, Event, Request, SnapshotTarget},
    query::Query,
};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComparisonRefs {
    pub base: String,
    pub head: String,
    pub base_snapshot: Option<String>,
    pub head_snapshot: Option<String>,
}

impl ComparisonRefs {
    pub(super) fn validate(&self, header: &Header) -> Result<()> {
        header.validate(&self.base, &self.head)?;
        for (requested, endpoint) in [
            (&self.base_snapshot, &header.base),
            (&self.head_snapshot, &header.head),
        ] {
            if let Some(id) = requested {
                ensure!(
                    endpoint
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| &snapshot.id == id),
                    "CLI returned another selected snapshot"
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct SnapshotStatus {
    pub cache_path: PathBuf,
    pub commit: String,
    pub snapshots: Vec<SnapshotRecord>,
}

pub struct SnapshotPreparation {
    pub revision: String,
    pub analysis: Analysis,
}

impl SnapshotPreparation {
    fn request(self, comparison_project: &Path) -> Result<Request> {
        let project = comparison_project
            .canonicalize()
            .context("Cannot open the comparison project")?;
        let selected_project = self
            .analysis
            .project
            .canonicalize()
            .context("Cannot open the main graph project")?;
        ensure!(
            project == selected_project,
            "The main graph build selection belongs to a different project"
        );
        let targets = self
            .analysis
            .targets
            .into_iter()
            .map(|target| {
                let path = target.path.strip_prefix(&project).with_context(|| {
                    format!(
                        "Selected build {} is outside the comparison project",
                        target.path.display()
                    )
                })?;
                Ok(SnapshotTarget {
                    path: if path.as_os_str().is_empty() {
                        ".".into()
                    } else {
                        path.into()
                    },
                    kind: target.kind,
                    language: target.language,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Request::PrepareSnapshot {
            version: protocol::VERSION,
            project,
            revision: self.revision,
            targets: Vec::new(),
            exact_targets: Some(targets),
        })
    }
}

impl QueryContext {
    pub fn require_local_snapshots(&self, cancel: &AtomicBool) -> Result<()> {
        ensure!(
            self.client.local_snapshots_supported(cancel)?,
            "unsupported_capability: update the CLI to use local snapshots"
        );
        Ok(())
    }
    pub fn snapshot_status(&self, revision: String, cancel: &AtomicBool) -> Result<SnapshotStatus> {
        self.require_local_snapshots(cancel)?;
        let result: SnapshotStatus = self.request(Query::SnapshotStatus { revision }, cancel)?;
        ensure!(
            result.cache_path.is_absolute(),
            "CLI returned an invalid cache path"
        );
        ensure!(
            matches!(result.commit.len(), 40 | 64)
                && result.commit.bytes().all(|byte| byte.is_ascii_hexdigit())
                && result
                    .snapshots
                    .iter()
                    .all(|snapshot| snapshot.identity.commit == result.commit),
            "CLI returned inconsistent snapshot status"
        );
        Ok(result)
    }

    pub fn prepare_local_snapshot(
        &self,
        selection: SnapshotPreparation,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
    ) -> Result<SnapshotRecord> {
        ensure!(
            self.client.exact_snapshot_targets_supported(cancel)?,
            "unsupported_capability: update the CLI to prepare snapshots with the main graph's build selection"
        );
        let request = selection.request(&self.descriptor.project)?;
        match super::super::exchange(self.client.runtime(), &request, cancel, progress)? {
            Event::SnapshotPrepared { snapshot } => Ok(snapshot),
            _ => bail!("Engine returned an unexpected result for snapshot preparation"),
        }
    }
}
