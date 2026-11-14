//! Bound metadata survives strict artifact unavailability.
use super::{Artifact, Endpoint, EndpointKind, Snapshot};
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
                };
                Ok(Snapshot { artifact, endpoint })
            }
            ArtifactResult::Unavailable(error) => Err(super::failure(error.code, error.message)),
        }
    }
}

/// Read once, validate strictly, and retain provenance even when saved facts are incompatible.
pub fn inspect(project: &Path, revision: &str) -> Result<Inspection> {
    let read = super::read_snapshot(project, revision)?;
    let decoded = Artifact::from_bytes(&read.bytes);
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
    };
    let artifact = match decoded {
        Ok(artifact) => ArtifactResult::Available(artifact),
        Err(error) => ArtifactResult::Unavailable(ArtifactFailure {
            code: "invalid_graph",
            message: format!("{error:#}"),
        }),
    };
    Ok(Inspection { endpoint, artifact })
}

fn observed_schema(bytes: &[u8]) -> ObservedSchema {
    // Observation after failure does not turn a relaxed JSON projection into a usable artifact.
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| value.get("schema")?.as_str().map(str::to_owned))
        .map_or(ObservedSchema::Unknown, ObservedSchema::Known)
}
