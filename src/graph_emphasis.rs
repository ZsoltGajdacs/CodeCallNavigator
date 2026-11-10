//! Selection emphasis over the visible graph, independent of depth and geometry.
use crate::projection::GraphView;

const DIM_CARD_OPACITY: f32 = 0.35;
const DIM_EDGE_OPACITY: f32 = 0.08;

pub struct GraphEmphasis {
    anchors: Vec<bool>,
    bright_nodes: Vec<bool>,
    incident_edges: Vec<bool>,
    active: bool,
}

impl GraphEmphasis {
    pub fn new(view: &GraphView, selection: Option<&str>) -> Self {
        let selected = selection.and_then(|key| view.nodes.iter().position(|node| node.key == key));
        let anchors: Vec<_> = view
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| match selected {
                Some(selected) => index == selected,
                None => view.focus_keys.contains(&node.key),
            })
            .collect();
        let active = anchors.iter().any(|&anchor| anchor);
        let mut bright_nodes = anchors.clone();
        let incident_edges = view
            .edges
            .iter()
            .map(|edge| {
                let incident = anchors[edge.from] || anchors[edge.to];
                if incident {
                    bright_nodes[edge.from] = true;
                    bright_nodes[edge.to] = true;
                }
                incident
            })
            .collect();
        Self {
            anchors,
            bright_nodes,
            incident_edges,
            active,
        }
    }

    pub fn is_anchor(&self, node: usize) -> bool {
        self.anchors[node]
    }

    pub fn card_opacity(&self, node: usize) -> f32 {
        if !self.active || self.bright_nodes[node] {
            1.0
        } else {
            DIM_CARD_OPACITY
        }
    }

    pub fn edge_emphasized(&self, edge: usize) -> bool {
        !self.active || self.incident_edges[edge]
    }

    pub fn edge_opacity(&self, edge: usize) -> f32 {
        if self.edge_emphasized(edge) {
            1.0
        } else {
            DIM_EDGE_OPACITY
        }
    }
}

#[cfg(test)]
#[path = "graph_emphasis/tests.rs"]
mod tests;
