use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
mod evidence;
pub use evidence::{RelationshipEvidence, SourceSite};

/// Declared access; absence on a node means unknown or not applicable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Package,
    Internal,
    ProtectedInternal,
    PrivateProtected,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Node {
    pub id: String,
    pub project: String,
    pub build: String,
    pub language: String,
    pub kind: String,
    pub label: String,
    pub signature: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_visibility: Option<Visibility>,
    pub namespace: String,
    pub container: String,
    pub file: String,
    pub line: u32,
    pub is_test: bool,
    pub is_generated: bool,
    pub body_unavailable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Edge {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<RelationshipEvidence>,
    #[serde(default)]
    pub provenance: String,
    pub from: u32,
    pub to: u32,
    pub kind: String,
    pub certainty: String,
    pub count: u32,
}

/// Shared by graph filtering and canvas rendering, including projected edges.
pub fn relationship_is_reference(kind: &str) -> bool {
    matches!(
        kind,
        "methodgroup" | "reference" | "callback" | "_LM" | "_BSM" | "react-event" | "react-prop"
    )
}

pub fn relationship_is_uncertain(certainty: &str) -> bool {
    !matches!(certainty, "direct" | "proven-direct")
}

impl Edge {
    pub fn is_reference(&self) -> bool {
        relationship_is_reference(&self.kind)
    }

    pub fn is_uncertain(&self) -> bool {
        relationship_is_uncertain(&self.certainty)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FileFingerprint {
    pub path: String,
    pub size: u64,
    pub modified_ns: u64,
    pub hash: String,
    /// Build-owned child names omitted from this directory's listing hash.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded_children: Vec<String>,
    /// An explicit input that did not exist when the snapshot was captured.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub missing: bool,
}

/// Analyzer-owned entry candidates and compiler/bytecode-verified executable starts.
/// Identities are stable across graph composition and independent of node offsets.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct EntryPoints {
    #[serde(default)]
    pub framework_entries: Vec<String>,
    pub candidates: Vec<String>,
    pub application_starts: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GraphData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundaries: Option<crate::boundary::BoundaryInventory>,
    /// Adapter-owned export keys; native bindings and graph identity stay intact.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub export_identities: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub flow_inventory: Option<crate::flow::FlowInventory>,
    #[serde(default)]
    pub architecture: Option<crate::architecture::ArchitectureFacts>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub diagnostics: Vec<String>,
    pub roots: Vec<String>,
    #[serde(default)]
    pub fingerprints: Vec<FileFingerprint>,
    #[serde(default)]
    pub engine_versions: String,
    #[serde(default)]
    pub entry_points: Option<EntryPoints>,
}

impl GraphData {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.nodes.len() <= u32::MAX as usize,
            "Too many graph nodes"
        );
        ensure!(
            self.edges.len() <= u32::MAX as usize,
            "Too many graph edges"
        );
        let mut identities = HashSet::with_capacity(self.nodes.len());
        for node in &self.nodes {
            ensure!(!node.id.is_empty(), "A graph symbol has no identity");
            ensure!(
                identities.insert(node.id.as_str()),
                "Duplicate graph identity: {}",
                node.id
            );
        }
        if let Some(inventory) = &self.boundaries {
            inventory.validate(&identities)?;
        }
        if let Some(entries) = &self.entry_points {
            validate_entries(&entries.candidates, &identities)?;
            validate_entries(&entries.framework_entries, &identities)?;
            validate_entries(&entries.application_starts, &identities)?;
        }
        for edge in &self.edges {
            ensure!(
                (edge.from as usize) < self.nodes.len() && (edge.to as usize) < self.nodes.len(),
                "Relationship references a missing graph node"
            );
            ensure!(edge.count > 0, "Relationship count must be positive");
            for evidence in &edge.evidence {
                evidence.validate(&identities)?;
            }
        }
        Ok(())
    }
}

fn validate_entries(entries: &[String], identities: &HashSet<&str>) -> Result<()> {
    let mut seen = HashSet::new();
    for entry in entries {
        ensure!(
            identities.contains(entry.as_str()),
            "Entry point references a missing graph symbol: {entry}"
        );
        ensure!(
            seen.insert(entry),
            "Duplicate entry point identity: {entry}"
        );
    }
    Ok(())
}
