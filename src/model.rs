use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Node {
    pub id: String,
    pub project: String,
    pub build: String,
    pub language: String,
    pub kind: String,
    pub label: String,
    pub signature: String,
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
    pub from: u32,
    pub to: u32,
    pub kind: String,
    pub certainty: String,
    pub count: u32,
}

impl Edge {
    pub fn is_reference(&self) -> bool {
        matches!(
            self.kind.as_str(),
            "methodgroup" | "reference" | "callback" | "_LM" | "_BSM"
        )
    }

    pub fn is_uncertain(&self) -> bool {
        !matches!(self.certainty.as_str(), "direct" | "proven-direct")
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FileFingerprint {
    pub path: String,
    pub size: u64,
    pub modified_ns: u64,
    pub hash: String,
}

/// Analyzer-owned entry candidates and compiler/bytecode-verified executable starts.
/// Identities are stable across graph composition and independent of node offsets.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct EntryPoints {
    pub candidates: Vec<String>,
    pub application_starts: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GraphData {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub diagnostics: Vec<String>,
    pub roots: Vec<String>,
    pub fingerprints: Vec<FileFingerprint>,
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
        if let Some(entries) = &self.entry_points {
            validate_entries(&entries.candidates, &identities)?;
            validate_entries(&entries.application_starts, &identities)?;
        }
        for edge in &self.edges {
            ensure!(
                (edge.from as usize) < self.nodes.len() && (edge.to as usize) < self.nodes.len(),
                "Relationship references a missing graph node"
            );
            ensure!(edge.count > 0, "Relationship count must be positive");
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
