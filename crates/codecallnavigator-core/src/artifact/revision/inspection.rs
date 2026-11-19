//! Bound metadata survives strict artifact unavailability.
use super::{Artifact, Endpoint, EndpointKind, ReadSnapshot, Repository, Snapshot, failure};
use crate::artifact::{
    GRAPH_PATH,
    local_snapshots::{SnapshotRecord, Store},
};
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ObservedSchema {
    Known(String),
    Unknown,
}

/// Field order and known-schema serialization match the legacy Endpoint binding.
#[derive(Clone, Debug, Serialize)]
pub struct InspectedEndpoint {
    pub kind: EndpointKind,
    pub requested_ref: String,
    pub commit: Option<String>,
    pub path: String,
    pub schema: ObservedSchema,
    pub digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<SnapshotRecord>,
}
#[derive(Debug, Serialize)]
pub struct ArtifactFailure {
    pub code: &'static str,
    pub message: String,
}
#[derive(Debug)]
pub enum ArtifactResult {
    Available(Artifact),
    Unavailable(ArtifactFailure),
}
#[derive(Debug)]
pub struct Inspection {
    pub endpoint: InspectedEndpoint,
    pub artifact: ArtifactResult,
}
impl Inspection {
    /// Legacy consumers retain the original all-or-error contract.
    pub fn into_snapshot(self) -> Result<Snapshot> {
        match self.artifact {
            ArtifactResult::Available(artifact) => {
                let endpoint = Endpoint {
                    kind: self.endpoint.kind,
                    requested_ref: self.endpoint.requested_ref,
                    commit: self.endpoint.commit,
                    path: self.endpoint.path,
                    schema: artifact.schema.clone(),
                    digest: self.endpoint.digest,
                    snapshot: self.endpoint.snapshot,
                };
                Ok(Snapshot { artifact, endpoint })
            }
            ArtifactResult::Unavailable(error) => Err(super::failure(error.code, error.message)),
        }
    }
}

/// Read once, validate strictly, and retain provenance even when saved facts are incompatible.
pub fn inspect(project: &Path, revision: &str) -> Result<Inspection> {
    Ok(inspect_read(
        super::read_snapshot(project, revision)?,
        revision,
    ))
}

/// Prefer a committed graph unless the caller explicitly selected a cached variant.
pub fn inspect_cached(
    project: &Path,
    revision: &str,
    selected: Option<&str>,
) -> Result<Inspection> {
    if revision == "WORKTREE" {
        if selected.is_some() {
            return Err(failure(
                "invalid_request",
                "WORKTREE does not take a snapshot ID",
            ));
        }
        return inspect(project, revision);
    }
    let commit = super::resolve_commit(project, revision)?;
    if selected.is_none()
        && let Some(read) = committed_if_present(project, &commit)?
    {
        return Ok(inspect_read(read, revision));
    }
    Ok(inspect_read(
        read_cached(project, commit, selected)?,
        revision,
    ))
}

fn committed_if_present(project: &Path, commit: &str) -> Result<Option<ReadSnapshot>> {
    match super::read_commit(project, commit.into()) {
        Ok(snapshot) => Ok(Some(snapshot)),
        Err(error) if super::error_code(&error) == "graph_missing" => Ok(None),
        Err(error) => Err(error),
    }
}

fn read_cached(project: &Path, commit: String, selected: Option<&str>) -> Result<ReadSnapshot> {
    let repository = Repository::open(project)?;
    let store = Store::system().map_err(cache_error)?;
    let records = store
        .list(&repository.common_dir, &commit, &repository.project)
        .map_err(cache_error)?;
    let record = select_record(records, selected, &commit)?;
    let captured = store
        .capture(&repository.common_dir, &record)
        .map_err(cache_error)?;
    let path = if repository.project.is_empty() {
        GRAPH_PATH.into()
    } else {
        format!("{}/{GRAPH_PATH}", repository.project)
    };
    Ok(ReadSnapshot {
        bytes: captured.manifest_bytes().to_vec(),
        decoded: captured.complete(),
        commit: Some(commit),
        kind: EndpointKind::Commit,
        path,
        snapshot: Some(record),
    })
}

fn select_record(
    records: Vec<SnapshotRecord>,
    selected: Option<&str>,
    commit: &str,
) -> Result<SnapshotRecord> {
    if let Some(id) = selected {
        return records
            .into_iter()
            .find(|record| record.id == id)
            .ok_or_else(|| {
                failure(
                    "snapshot_missing",
                    format!("Snapshot {id} is unavailable for {commit}"),
                )
            });
    }
    let mut records = records.into_iter();
    let first = records.next().ok_or_else(|| {
        failure(
            "graph_missing",
            format!(
                "No committed or local graph for {commit}. Prepare a snapshot for this revision."
            ),
        )
    })?;
    if records.next().is_some() {
        return Err(failure(
            "snapshot_ambiguous",
            format!("Multiple local snapshots exist for {commit}. Select a snapshot ID."),
        ));
    }
    Ok(first)
}

fn cache_error(error: anyhow::Error) -> anyhow::Error {
    failure("snapshot_unavailable", format!("{error:#}"))
}

fn inspect_read(read: ReadSnapshot, revision: &str) -> Inspection {
    let decoded = read.decoded;
    let schema = match &decoded {
        Ok(artifact) => ObservedSchema::Known(artifact.schema.clone()),
        Err(_) => observed_schema(&read.bytes),
    };
    let endpoint = InspectedEndpoint {
        kind: read.kind,
        requested_ref: revision.into(),
        commit: read.commit,
        path: read.path,
        schema,
        digest: blake3::hash(&read.bytes).to_hex().to_string(),
        snapshot: read.snapshot,
    };
    let artifact = match decoded {
        Ok(artifact) => ArtifactResult::Available(artifact),
        Err(error) => ArtifactResult::Unavailable(ArtifactFailure {
            code: "invalid_graph",
            message: format!("{error:#}"),
        }),
    };
    Inspection { endpoint, artifact }
}

fn observed_schema(bytes: &[u8]) -> ObservedSchema {
    // Observation after failure does not turn a relaxed JSON projection into a usable artifact.
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| value.get("schema")?.as_str().map(str::to_owned))
        .map_or(ObservedSchema::Unknown, ObservedSchema::Known)
}
