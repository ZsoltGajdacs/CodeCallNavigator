//! Bounded presentation of analyzed relationships, separate from graph layout.
use codegraph::architecture::LayerAssignments;
use codegraph::graph::{Direction, Filter, GraphIndex};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

#[path = "projection/aggregation.rs"]
mod aggregation;
#[path = "projection/focus.rs"]
mod focus;
#[path = "projection/focus_level.rs"]
mod focus_level;
#[path = "projection/geometry.rs"]
mod geometry;
#[path = "projection/groups.rs"]
mod groups;
#[path = "projection/labels.rs"]
mod labels;
#[path = "projection/layer_geometry.rs"]
mod layer_geometry;
#[path = "projection/layers.rs"]
mod layers;
#[path = "projection/ordering.rs"]
mod ordering;
#[path = "projection/selection.rs"]
mod selection;

pub use geometry::layout;
const NODE_LIMIT: usize = 1_500;
const EDGE_LIMIT: usize = 6_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grouping {
    #[default]
    Project,
    Namespace,
    Folder,
    Layer,
    Method,
}

/// Presentation has exactly two directions; engine queries may still use Both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowDirection {
    #[serde(alias = "Incoming")]
    Callers,
    #[default]
    #[serde(alias = "Outgoing", alias = "Both")]
    Callees,
}

impl From<FlowDirection> for Direction {
    fn from(value: FlowDirection) -> Self {
        match value {
            FlowDirection::Callers => Self::Incoming,
            FlowDirection::Callees => Self::Outgoing,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scope {
    #[default]
    Overview,
    Focus(u32),
    Path(u32, u32),
}

#[derive(Clone)]
pub struct Request {
    pub focus_target: Option<FocusTarget>,
    pub grouping: Grouping,
    pub expanded: HashSet<String>,
    pub scope: Scope,
    pub direction: FlowDirection,
    pub depth: u32,
    pub filter: Filter,
    pub page: usize,
    pub architecture: Option<Arc<LayerAssignments>>,
    pub layer_tab: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusTarget {
    Method(String),
    Group(String),
    ClassMethods(String),
    GroupChildren(String),
}

#[derive(Clone, Debug)]
pub struct MethodRow {
    pub name: String,
    pub description: String,
    pub member: u32,
    pub id: String,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupKind {
    Project,
    Namespace,
    Folder,
    Class,
    Layer,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcePath {
    pub label: String,
    pub path: String,
    pub language: String,
}

#[derive(Clone, Debug, Default)]
pub struct ViewNode {
    pub description: String,
    pub methods: Vec<MethodRow>,
    pub key: String,
    pub label: String,
    pub detail: String,
    pub member: Option<u32>,
    pub group_kind: Option<GroupKind>,
    pub files: Vec<SourcePath>,
    pub count: usize,
    pub internal_edges: usize,
    pub cyclic: bool,
    pub application_start: bool,
    pub layer: Option<String>,
    pub position: [f32; 2],
    pub size: [f32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct ViewEdge {
    pub from: usize,
    pub to: usize,
    pub kind: String,
    pub certainty: String,
    pub count: u64,
    pub points: Vec<[f32; 2]>,
}

/// A node's traversal level and owning parent, independent of geometry.
#[derive(Clone, Debug, Default)]
pub struct NodeOrder {
    pub rank: i32,
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct GraphView {
    pub focus_keys: Vec<String>,
    pub focus_valid: bool,
    pub nodes: Vec<ViewNode>,
    pub edges: Vec<ViewEdge>,
    pub ordering: Vec<NodeOrder>,
    pub size: [f32; 2],
    pub omitted_nodes: usize,
    pub omitted_edges: usize,
    pub hidden_nodes: usize,
    pub hidden_edges: usize,
    pub collapsed_edges: usize,
    pub message: String,
    pub pages: usize,
    pub layer_bands: Vec<LayerBand>,
}

/// Presentation metadata only; relationships retain analyzer direction and certainty.
#[derive(Clone, Debug, Default)]
pub struct LayerBand {
    pub name: String,
    pub flow: Option<String>,
    pub support: bool,
    pub beside: Option<String>,
    pub position: [f32; 2],
    pub size: [f32; 2],
}

struct Selection {
    nodes: Vec<u32>,
    edges: Vec<u32>,
    roots: Vec<u32>,
    direction: FlowDirection,
    message: String,
    omitted_nodes: usize,
    omitted_edges: usize,
}

pub fn project(graph: &GraphIndex, request: &Request) -> GraphView {
    if let Some(target) = focus_target(graph, request) {
        return aggregation::focused(graph, request, &target);
    }
    let mut selection = selection::resolve(graph, request);
    layers::select(graph, request, &mut selection);
    aggregation::project_page(graph, request, selection)
}

#[cfg(test)]
#[path = "projection/focus_tests.rs"]
mod focus_tests;

pub fn focus_target(graph: &GraphIndex, request: &Request) -> Option<FocusTarget> {
    if matches!(request.scope, Scope::Path(..)) {
        return None;
    }
    request
        .focus_target
        .clone()
        .or_else(|| match request.scope {
            Scope::Focus(node) => graph
                .data
                .nodes
                .get(node as usize)
                .map(|n| FocusTarget::Method(n.id.clone())),
            _ => None,
        })
}
