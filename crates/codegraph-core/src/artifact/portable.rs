//! Explicit persisted contracts. Runtime observations are deliberately absent.
use crate::{
    architecture::ArchitectureFacts,
    boundary::BoundaryInventory,
    flow::FlowInventory,
    model::{Edge, EntryPoints, GraphData, Node, RelationshipEvidence},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub(super) struct PortableGraph {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identity_roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folder_project_builds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundaries: Option<BoundaryInventory>,
    #[serde(default)]
    pub flow_inventory: Option<FlowInventory>,
    #[serde(default)]
    pub architecture: Option<ArchitectureFacts>,
    pub nodes: Vec<Node>,
    pub edges: Vec<PortableEdge>,
    pub diagnostics: Vec<String>,
    pub roots: Vec<String>,
    #[serde(default)]
    pub entry_points: Option<EntryPoints>,
}
#[derive(Serialize, Deserialize)]
pub(super) struct PortableEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub certainty: String,
    pub count: u32,
    #[serde(default)]
    pub provenance: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<RelationshipEvidence>,
}
impl PortableGraph {
    pub fn from_runtime(graph: &GraphData, ids: &HashMap<String, String>) -> Result<Self> {
        let edges = graph
            .edges
            .iter()
            .map(|edge| {
                Ok(PortableEdge {
                    from: ids
                        .get(&graph.nodes[edge.from as usize].id)
                        .context("Missing caller identity")?
                        .clone(),
                    to: ids
                        .get(&graph.nodes[edge.to as usize].id)
                        .context("Missing target identity")?
                        .clone(),
                    kind: edge.kind.clone(),
                    certainty: edge.certainty.clone(),
                    count: edge.count,
                    provenance: edge.provenance.clone(),
                    evidence: edge.evidence.clone(),
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            identity_roots: graph.identity_roots.clone(),
            folder_project_builds: graph.folder_project_builds.clone(),
            boundaries: graph.boundaries.clone(),
            flow_inventory: graph.flow_inventory.clone(),
            architecture: graph.architecture.clone(),
            nodes: graph.nodes.clone(),
            edges,
            diagnostics: graph.diagnostics.clone(),
            roots: graph.roots.clone(),
            entry_points: graph.entry_points.clone(),
        })
    }
    pub fn into_runtime(self, ids: &HashMap<String, u32>) -> Result<GraphData> {
        let edges = self
            .edges
            .into_iter()
            .map(|edge| {
                Ok(Edge {
                    from: *ids
                        .get(edge.from.as_str())
                        .with_context(|| format!("Missing artifact endpoint: {}", edge.from))?,
                    to: *ids
                        .get(edge.to.as_str())
                        .with_context(|| format!("Missing artifact endpoint: {}", edge.to))?,
                    kind: edge.kind,
                    certainty: edge.certainty,
                    count: edge.count,
                    provenance: edge.provenance,
                    evidence: edge.evidence,
                })
            })
            .collect::<Result<_>>()?;
        let graph = GraphData {
            nodes: self.nodes,
            edges,
            identity_roots: self.identity_roots,
            folder_project_builds: self.folder_project_builds,
            boundaries: self.boundaries,
            flow_inventory: self.flow_inventory,
            architecture: self.architecture,
            diagnostics: self.diagnostics,
            roots: self.roots,
            entry_points: self.entry_points,
            ..Default::default()
        };
        Ok(graph)
    }
}
