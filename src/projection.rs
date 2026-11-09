//! Bounded presentation of analyzed relationships, separate from graph layout.
use codegraph::graph::{Direction, Filter, GraphIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[path = "projection/aggregation.rs"]
mod aggregation;
#[path = "projection/geometry.rs"]
mod geometry;
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
    pub grouping: Grouping,
    pub expanded: HashSet<String>,
    pub scope: Scope,
    pub direction: FlowDirection,
    pub depth: u32,
    pub filter: Filter,
    pub page: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupKind {
    Project,
    Namespace,
    Folder,
    Class,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcePath {
    pub label: String,
    pub path: String,
    pub language: String,
}

#[derive(Clone, Debug, Default)]
pub struct ViewNode {
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
    pub rank: usize,
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct GraphView {
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
    let selection = selection::resolve(graph, request);
    aggregation::project_page(graph, request, selection)
}
