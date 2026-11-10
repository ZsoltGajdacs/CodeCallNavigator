//! Portable graph artifacts. Runtime observations belong to the process protocol.
mod compare;
mod identity;
mod paths;
mod publish;
mod transform;
use transform::{sort_sets, transform};
#[cfg(test)]
mod tests;

use anyhow::{Context, Result, ensure};
pub use compare::{Comparison, compare, compare_git};
pub use publish::{AnalysisLock, load, publish};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};

pub type Roots = [(String, PathBuf)];
pub const SCHEMA: &str = "codegraph.graph.v1";
pub const GRAPH_PATH: &str = ".codegraph/graph.json";

pub fn validate_roots(roots: &Roots) -> Result<()> {
    paths::Paths::new(roots).map(|_| ())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub schema: String,
    pub graph: Value,
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
    let mut value = serde_json::to_value(graph)?;
    let object = value.as_object_mut().context("Graph is not an object")?;
    object.remove("fingerprints");
    object.remove("engine_versions");
    object.remove("export_identities");
    for (index, edge) in object["edges"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        let native = &graph.edges[index];
        edge["from"] = identities[&graph.nodes[native.from as usize].id]
            .clone()
            .into();
        edge["to"] = identities[&graph.nodes[native.to as usize].id]
            .clone()
            .into();
    }
    transform(&mut value, &paths, Some(&identities), false)?;
    sort_sets(&mut value, "");
    let artifact = Artifact {
        schema: SCHEMA.into(),
        graph: value,
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

    pub fn bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut value = serde_json::to_value(self)?;
        sort_sets(&mut value, "");
        let mut bytes = serde_json::to_vec_pretty(&value)?;
        bytes.push(b'\n');
        Ok(bytes)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema == SCHEMA,
            "Unsupported graph artifact schema: {}",
            self.schema
        );
        self.portable_graph()?;
        Ok(())
    }

    pub fn hydrate(&self, roots: &Roots) -> Result<crate::model::GraphData> {
        self.validate()?;
        let paths = paths::Paths::new(roots)?;
        let mut graph = self.portable_graph()?;
        let mut value = serde_json::to_value(&graph)?;
        transform(&mut value, &paths, None, true)?;
        graph = serde_json::from_value(value)?;
        graph.validate()?;
        Ok(graph)
    }

    fn portable_graph(&self) -> Result<crate::model::GraphData> {
        let mut value = self.graph.clone();
        let nodes = value["nodes"]
            .as_array()
            .context("Artifact nodes must be an array")?;
        let mut ids = HashMap::new();
        for (index, node) in nodes.iter().enumerate() {
            let id = node["id"]
                .as_str()
                .context("Artifact node has no string identity")?;
            ensure!(
                ids.insert(id.to_owned(), index as u32).is_none(),
                "Duplicate artifact identity: {id}"
            );
        }
        for edge in value["edges"]
            .as_array_mut()
            .context("Artifact edges must be an array")?
        {
            for endpoint in ["from", "to"] {
                let id = edge[endpoint]
                    .as_str()
                    .context("Artifact edge endpoint must be an identity")?;
                edge[endpoint] = (*ids
                    .get(id)
                    .with_context(|| format!("Missing artifact endpoint: {id}"))?)
                .into();
            }
        }
        value["fingerprints"] = Value::Array(Vec::new());
        value["engine_versions"] = String::new().into();
        let graph: crate::model::GraphData = serde_json::from_value(value)?;
        graph.validate()?;
        validate_symbols(&self.graph, &ids)?;
        Ok(graph)
    }
}

fn validate_symbols(value: &Value, ids: &HashMap<String, u32>) -> Result<()> {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if key == "symbol" {
                    let symbol = value
                        .as_str()
                        .context("Flow target must have a string identity")?;
                    ensure!(
                        ids.contains_key(symbol),
                        "Flow references missing symbol: {symbol}"
                    );
                } else {
                    validate_symbols(value, ids)?;
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                validate_symbols(value, ids)?;
            }
        }
        _ => {}
    }
    Ok(())
}
