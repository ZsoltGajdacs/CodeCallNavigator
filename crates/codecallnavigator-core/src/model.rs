use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
mod evidence;
mod relationship;
pub use evidence::{RelationshipEvidence, SourceSite};
pub use relationship::{RelationshipClassification, RelationshipKind, TargetResolution};

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
    #[serde(default)]
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct FileFingerprint {
    /// Captured input kind distinguishes files from directories with equal hashes.
    pub kind: crate::fingerprint::InputKind,
    #[serde(default)]
    pub observation_policy: crate::fingerprint::InputPolicy,
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
#[serde(default)]
pub struct CompilerOutputs {
    pub bytecode_directories: Vec<String>,
    pub raw_bytecode_directories: Option<Vec<String>>,
    pub compile_classpath: Vec<String>,
    pub source_roots: Vec<String>,
    pub build_inputs: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GraphData {
    /// Runtime-only compiler ownership used to build portable refresh policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_outputs: Option<CompilerOutputs>,
    /// Compiler-owned context directories embedded in native identities.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identity_roots: Vec<String>,
    /// Build owners whose project display labels come from their directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folder_project_builds: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boundaries: Option<crate::boundary::BoundaryInventory>,
    /// Adapter-owned export keys; native bindings and graph identity stay intact.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub export_identities: std::collections::HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
        self.validate_inventory_versions()?;
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

    fn validate_inventory_versions(&self) -> Result<()> {
        if let Some(inventory) = &self.flow_inventory {
            ensure!(
                inventory.version == crate::flow::INVENTORY_VERSION,
                "Unsupported function flow inventory version"
            );
        }
        if let Some(facts) = &self.architecture {
            ensure!(
                facts.version == crate::architecture::FACTS_VERSION,
                "Unsupported architecture declaration facts version"
            );
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

/// Immutable navigation data. Construction removes all function bodies, so an
/// indexed structural generation cannot keep a separately unloaded inventory alive.
#[derive(Debug, Default, Serialize)]
#[serde(transparent)]
pub struct GraphStructure(GraphData);

impl GraphStructure {
    /// Explicitly copy structure when a caller needs an independently editable graph.
    pub fn to_graph_data(&self) -> GraphData {
        self.0.clone()
    }

    pub fn split(mut graph: GraphData) -> (Self, Option<crate::flow::FlowInventory>) {
        let flows = graph.flow_inventory.take();
        (Self(graph), flows)
    }
}

impl std::ops::Deref for GraphStructure {
    type Target = GraphData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for GraphStructure {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = GraphData::deserialize(deserializer)?;
        if data.flow_inventory.is_some() {
            return Err(serde::de::Error::custom(
                "Structural snapshots cannot contain function flows",
            ));
        }
        Ok(Self(data))
    }
}

#[cfg(test)]
mod structure_tests {
    use super::*;

    #[test]
    fn structural_snapshot_excludes_function_bodies() {
        let inventory = crate::flow::FlowInventory {
            version: crate::flow::INVENTORY_VERSION,
            functions: vec![crate::flow::FunctionFlowIR {
                symbol: "example".into(),
                ..Default::default()
            }],
        };
        let (structure, flows) = GraphStructure::split(GraphData {
            flow_inventory: Some(inventory.clone()),
            ..Default::default()
        });
        assert!(structure.flow_inventory.is_none());
        assert_eq!(flows, Some(inventory));
        let wire = serde_json::to_value(&structure).unwrap();
        assert!(wire.get("flow_inventory").is_none());
        assert!(serde_json::from_value::<GraphStructure>(wire).is_ok());
    }

    #[test]
    fn structural_snapshot_rejects_embedded_function_bodies() {
        let wire = serde_json::to_value(GraphData {
            flow_inventory: Some(crate::flow::FlowInventory {
                version: crate::flow::INVENTORY_VERSION,
                functions: Vec::new(),
            }),
            ..Default::default()
        })
        .unwrap();
        assert!(serde_json::from_value::<GraphStructure>(wire).is_err());
    }
}
