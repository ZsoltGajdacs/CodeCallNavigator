//! Portable graph artifacts. Runtime observations belong to the process protocol.
mod canonicalization;
mod compare;
pub mod difference;
mod identity;
mod paths;
mod portable;
pub mod provenance;
mod publish;
pub mod revision;
mod transform;
mod validation;
pub mod verification;
use canonicalization::sort_graph;
use transform::transform;
#[cfg(test)]
mod tests;

use anyhow::{Context, Result, ensure};
pub use compare::{Comparison, compare, compare_git};
pub use publish::{AnalysisLock, load, publish};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};

pub type Roots = [(String, PathBuf)];
pub const SCHEMA: &str = "codecallnavigator.graph.v1";
pub const GRAPH_PATH: &str = ".codecallnavigator/graph.json";

pub fn validate_roots(roots: &Roots) -> Result<()> {
    paths::Paths::new(roots).map(|_| ())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub schema: String,
    pub graph: Value,
    #[serde(flatten)]
    pub extensions: std::collections::BTreeMap<String, Value>,
}

/// Canonical bytes validated and produced only by the artifact serializer.
/// This buffer is shared by digest computation and atomic publication.
pub struct EncodedArtifact(Vec<u8>);
impl AsRef<[u8]> for EncodedArtifact {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Canonicalize named declarations; anonymous adapter keys can be supplied below.
pub fn canonical(graph: &crate::model::GraphData, roots: &Roots) -> Result<Artifact> {
    canonical_with_identities(graph, roots, &HashMap::new())
}

/// Explicit keys are compiler-owned semantic keys, never source positions.
pub fn canonical_with_identities(
    graph: &crate::model::GraphData,
    roots: &Roots,
    semantic_keys: &HashMap<String, String>,
) -> Result<Artifact> {
    graph.validate()?;
    let paths = paths::Paths::new(roots)?;
    let identities = identity::identities(graph, &paths, semantic_keys)?;
    let mut portable = portable::PortableGraph::from_runtime(graph, &identities)?;
    transform(&mut portable, &paths, Some(&identities), false)?;
    let mut value = serde_json::to_value(portable)?;
    sort_graph(&mut value)?;
    let artifact = Artifact {
        schema: SCHEMA.into(),
        graph: value,
        extensions: Default::default(),
    };
    artifact.validate()?;
    Ok(artifact)
}

impl Artifact {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let artifact: Self =
            serde_json::from_slice(bytes).context("Invalid graph artifact JSON")?;
        artifact.validate()?;
        Ok(artifact)
    }

    pub fn encode(&self) -> Result<EncodedArtifact> {
        Ok(EncodedArtifact(self.bytes()?))
    }

    pub fn bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut value = serde_json::to_value(self)?;
        sort_graph(&mut value["graph"])?;
        let mut bytes = serde_json::to_vec(&value)?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn validate(&self) -> Result<()> {
        self.validated_portable()?;
        Ok(())
    }

    pub fn hydrate(&self, roots: &Roots) -> Result<crate::model::GraphData> {
        let (mut portable, ids) = self.validated_portable()?;
        transform(&mut portable, &paths::Paths::new(roots)?, None, true)?;
        portable.into_runtime(&ids)
    }

    fn validated_portable(&self) -> Result<(portable::PortableGraph, HashMap<String, u32>)> {
        ensure!(
            self.schema == SCHEMA,
            "Unsupported graph artifact schema: {}",
            self.schema
        );
        ensure!(
            !self.extensions.contains_key("schema") && !self.extensions.contains_key("graph"),
            "Reserved artifact extension property"
        );
        provenance::validate_analysis(&self.graph)?;
        let mut graph = portable::PortableGraph::deserialize(&self.graph)?;
        let ids = validation::validate(&mut graph)?;
        Ok((graph, ids))
    }
}
