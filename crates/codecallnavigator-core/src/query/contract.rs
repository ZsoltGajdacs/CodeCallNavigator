use crate::graph::{Direction, Filter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub project: PathBuf,
    #[serde(default)]
    pub roots: Vec<(String, PathBuf)>,
    #[serde(default)]
    pub expected_generation: Option<String>,
    pub query: Query,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    Status,
    /// Session-owned flow residency, bound to an explicit graph generation.
    FlowResources {
        action: FlowResourceAction,
    },
    Freshness {
        observation_version: u32,
    },
    SnapshotStatus {
        revision: String,
    },
    ImpactPreview {
        base: String,
        #[serde(default = "comparison_head")]
        head: String,
        #[serde(default)]
        base_snapshot: Option<String>,
    },
    Compare {
        base: String,
        #[serde(default = "comparison_head")]
        head: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_snapshot: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        head_snapshot: Option<String>,
        #[serde(default)]
        offset: usize,
        #[serde(default = "comparison_limit")]
        limit: usize,
        #[serde(default)]
        expected_comparison: Option<String>,
    },
    ComparisonChanges {
        base: String,
        #[serde(default = "comparison_head")]
        head: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_snapshot: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        head_snapshot: Option<String>,
        #[serde(default)]
        expected_comparison: Option<String>,
        selection: ComparisonChangesSelection,
    },
    ClassFlowComparison {
        base: String,
        #[serde(default = "comparison_head")]
        head: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_snapshot: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        head_snapshot: Option<String>,
        expected_comparison: String,
        id: String,
    },
    ClassComparisonSource {
        base: String,
        #[serde(default = "comparison_head")]
        head: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_snapshot: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        head_snapshot: Option<String>,
        expected_comparison: String,
        id: String,
        node: String,
        side: super::ComparisonSide,
    },
    FlowComparison {
        base: String,
        #[serde(default = "comparison_head")]
        head: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_snapshot: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        head_snapshot: Option<String>,
        #[serde(default)]
        expected_comparison: Option<String>,
        selection: FlowComparisonSelection,
    },
    /// Explicit full render snapshot; index-based edges are local to this generation only.
    Snapshot,
    Search {
        text: String,
        #[serde(default)]
        category: Category,
        #[serde(default)]
        offset: usize,
        #[serde(default = "search_limit")]
        limit: usize,
    },
    Symbol {
        symbol: String,
    },
    Source {
        symbol: String,
        #[serde(default)]
        origin: SourceOrigin,
    },
    LiveSource {
        path: PathBuf,
    },
    Architecture {
        #[serde(default)]
        selection: Option<crate::architecture::Selection>,
        #[serde(default)]
        previous_signature: Option<String>,
        #[serde(default)]
        configuration_root: Option<PathBuf>,
    },
    Explorer {
        /// Absolute live directory path, restricted to configured roots by the engine.
        path: PathBuf,
        #[serde(default)]
        offset: usize,
        #[serde(default = "directory_limit")]
        limit: usize,
    },
    Selection {
        selection: SelectionKind,
        #[serde(default, deserialize_with = "filter")]
        filter: Filter,
        #[serde(default)]
        limits: Limits,
    },
    /// Complete semantic membership for generation-bound desktop presentation.
    /// Traversal retains the graph engine's intrinsic safety limits.
    RenderSelection {
        selection: SelectionKind,
        #[serde(default, deserialize_with = "filter")]
        filter: Filter,
        #[serde(default)]
        layer: Option<RenderLayer>,
    },
    /// Complete root-incident relationships for a generation-bound inspector.
    Inspect {
        symbol: String,
        #[serde(default, deserialize_with = "filter")]
        filter: Filter,
    },
    Neighbors {
        symbol: String,
        #[serde(default, deserialize_with = "filter")]
        filter: Filter,
        #[serde(default)]
        limits: Limits,
    },
    Flow {
        symbol: String,
    },
    /// Explicit addressed expansion over exact saved source; generation binding is mandatory.
    FlowExpansion(FlowExpansionRequest),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowExpansionRequest {
    pub expansion_version: u32,
    pub symbol: String,
    #[serde(default)]
    pub selections: Vec<FlowExpansionSelection>,
    #[serde(default)]
    pub limits: crate::flow::ExpansionLimits,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowExpansionSelection {
    pub caller: crate::flow::OccurrencePath,
    pub choice: crate::flow::CallChoice,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FlowComparisonSelection {
    Catalog {
        #[serde(default)]
        text: String,
        #[serde(default)]
        offset: usize,
        #[serde(default = "flow_catalog_limit")]
        limit: usize,
    },
    Detail {
        symbol: String,
        #[serde(default)]
        head_symbol: Option<String>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComparisonChangesSelection {
    List {
        #[serde(default)]
        offset: usize,
        #[serde(default = "comparison_limit")]
        limit: usize,
    },
    SourceDiff {
        id: String,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderLayer {
    #[serde(default)]
    pub selection: Option<crate::architecture::Selection>,
    #[serde(default)]
    pub tab: Option<String>,
    #[serde(default)]
    pub expected_signature: Option<String>,
    #[serde(default)]
    pub configuration_root: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    #[default]
    All,
    Classes,
    Methods,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceOrigin {
    #[default]
    Analyzed,
    Live,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SelectionKind {
    Overview,
    EntryPoints {
        direction: Direction,
        depth: u32,
    },
    Neighborhood {
        symbols: Vec<String>,
        direction: Direction,
        depth: u32,
    },
    Path {
        from: String,
        to: String,
    },
    File {
        file: String,
    },
    Class {
        build: String,
        project: String,
        container: String,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub nodes: usize,
    pub edges: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            nodes: 1500,
            edges: 6000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
}
impl Error {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Response {
    pub version: u32,
    pub generation: Option<String>,
    pub freshness: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<Error>,
}

fn search_limit() -> usize {
    50
}
fn comparison_head() -> String {
    "WORKTREE".into()
}
fn comparison_limit() -> usize {
    100
}
fn directory_limit() -> usize {
    1000
}

// Persisted desktop filters support a legacy upgrade policy that intentionally
// resets old relationship switches. Public requests must never silently apply
// that policy to a caller's explicit relationship selection.
fn filter<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Filter, D::Error> {
    let value = Value::deserialize(deserializer)?;
    if value.get("version").and_then(Value::as_u64) != Some(2) {
        return Err(serde::de::Error::custom(
            "Explicit query filters require version 2 and both relationship selections",
        ));
    }
    serde_json::from_value(value).map_err(serde::de::Error::custom)
}

fn flow_catalog_limit() -> usize {
    50
}

/// The caller owns residency policy; the CLI owns the retained data.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlowResourceAction {
    Load,
    Unload,
    Status,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowResourceStatus {
    pub loaded: bool,
    pub functions: usize,
    /// Approximation of heap allocations retained by the typed inventory and its lookup index.
    pub retained_bytes: usize,
    pub load_milliseconds: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonSide {
    Before,
    After,
}
