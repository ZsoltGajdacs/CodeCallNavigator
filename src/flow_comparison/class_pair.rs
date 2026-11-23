//! One class layout projected into two independently framed revision panes.
use super::{SideId, Workspace};
use crate::{
    camera::Camera,
    projection::{self, GraphView, NodeOrder, ViewEdge, ViewNode},
};
use anyhow::{Context, Result, ensure};
use codecallnavigator::artifact::comparison_changes::{ClassGraph, ClassNode, GraphChange};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) struct ClassPair {
    pub graph: ClassGraph,
    pub layout: GraphView,
    pub root: usize,
    pub cameras: [Camera; 2],
    fit_pending: [bool; 2],
}

impl ClassPair {
    pub(super) fn load(graph: ClassGraph, dot: &Path, cancel: &AtomicBool) -> Result<Self> {
        ensure!(!cancel.load(Ordering::Acquire), "Comparison cancelled");
        let indices: BTreeMap<_, _> = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect();
        ensure!(
            indices.len() == graph.nodes.len(),
            "Duplicate class in graph"
        );
        let root = index_of(&indices, &graph.root)?;
        let edges = edges(&graph, &indices, root)?;
        let layout = projection::geometry::layout_cancellable(
            GraphView {
                nodes: graph.nodes.iter().map(|_| ViewNode::default()).collect(),
                ordering: ordering(graph.nodes.len(), root, &edges),
                edges,
                ..GraphView::default()
            },
            dot,
            cancel,
        )?;
        ensure!(!cancel.load(Ordering::Acquire), "Comparison cancelled");
        Ok(Self {
            graph,
            layout,
            root,
            cameras: [Camera::default(), Camera::default()],
            fit_pending: [true, true],
        })
    }

    pub fn has_root(&self, side: SideId) -> bool {
        present(&self.graph.nodes[self.root], side)
    }

    pub fn has_changes(&self) -> bool {
        self.graph
            .nodes
            .iter()
            .any(|node| node.status != GraphChange::Unchanged)
            || self
                .graph
                .links
                .iter()
                .any(|link| link.status != GraphChange::Unchanged)
    }

    pub fn fit(&mut self, side: SideId, viewport: [f32; 2]) {
        self.cameras[side.index()].fit(self.layout.size, viewport);
    }
}

impl Workspace {
    pub(crate) fn measure_class_pane(&mut self, side: SideId, id: &str, viewport: [f32; 2]) {
        if self.class_selection() != Some(id) {
            return;
        }
        self.layout(side, viewport);
        if let Some(pair) = &mut self.class_pair
            && std::mem::take(&mut pair.fit_pending[side.index()])
        {
            pair.fit(side, viewport);
        }
    }

    pub(crate) fn class_camera_mut(&mut self, side: SideId) -> Option<&mut Camera> {
        self.class_pair
            .as_mut()
            .map(|pair| &mut pair.cameras[side.index()])
    }
}

pub(crate) fn present(node: &ClassNode, side: SideId) -> bool {
    match side {
        SideId::Before => node.before,
        SideId::After => node.after,
    }
}

fn index_of(indices: &BTreeMap<&str, usize>, id: &str) -> Result<usize> {
    indices
        .get(id)
        .copied()
        .context("Class graph references an unknown class")
}

fn edges(
    graph: &ClassGraph,
    indices: &BTreeMap<&str, usize>,
    root: usize,
) -> Result<Vec<ViewEdge>> {
    let mut seen = BTreeSet::new();
    graph
        .links
        .iter()
        .map(|link| {
            let (from, to) = (index_of(indices, &link.from)?, index_of(indices, &link.to)?);
            ensure!(
                from != to && (from == root || to == root) && seen.insert((from, to)),
                "Invalid class relationship"
            );
            Ok(ViewEdge {
                from,
                to,
                ..ViewEdge::default()
            })
        })
        .collect()
}

fn ordering(count: usize, root: usize, edges: &[ViewEdge]) -> Vec<NodeOrder> {
    let mut order = vec![
        NodeOrder {
            rank: 1,
            parent: Some(root)
        };
        count
    ];
    order[root] = NodeOrder::default();
    for edge in edges.iter().filter(|edge| edge.to == root) {
        order[edge.from].rank = -1;
    }
    order
}
