mod filter;
mod filter_summary;
use crate::model::{Edge, GraphData, RelationshipClassification};
use anyhow::Result;
pub use filter::Filter;
pub use filter_summary::FilterSummary;
use filter_summary::SummaryCache;
use petgraph::{algo::kosaraju_scc, graph::DiGraph};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

pub const MAX_CANVAS_NODES: usize = 1_500;
pub const MAX_CANVAS_EDGES: usize = 6_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Direction {
    Incoming,
    Outgoing,
    Both,
}

/// Bounded traversal centered on one method.
pub struct Neighborhood<'a> {
    pub node: u32,
    pub direction: Direction,
    pub depth: u32,
    pub filter: &'a Filter,
}

/// Bounded traversal rooted at one or more methods.
pub struct MultiNeighborhood<'a> {
    pub roots: &'a [u32],
    pub direction: Direction,
    pub depth: u32,
    pub filter: &'a Filter,
}

#[derive(Clone, Debug, Default)]
pub struct GraphSlice {
    pub nodes: Vec<u32>,
    pub edges: Vec<u32>,
    pub omitted_nodes: usize,
    pub omitted_edges: usize,
}

struct Adjacency {
    offsets: Vec<usize>,
    edges: Vec<u32>,
}

struct Frontier {
    visited: Vec<bool>,
    queue: VecDeque<(u32, u32)>,
}

impl Frontier {
    fn new(node_count: usize) -> Self {
        Self {
            visited: vec![false; node_count],
            queue: VecDeque::new(),
        }
    }

    fn enqueue(&mut self, node: u32, distance: u32) {
        if !self.visited[node as usize] {
            self.visited[node as usize] = true;
            self.queue.push_back((node, distance));
        }
    }
}

impl Adjacency {
    fn build(data: &GraphData, direction: Direction) -> Self {
        let owner = |edge: &Edge| {
            if direction == Direction::Incoming {
                edge.to as usize
            } else {
                edge.from as usize
            }
        };
        let mut offsets = vec![0; data.nodes.len() + 1];
        for edge in &data.edges {
            offsets[owner(edge) + 1] += 1;
        }
        for index in 1..offsets.len() {
            offsets[index] += offsets[index - 1];
        }
        let mut cursor = offsets.clone();
        let mut edges = vec![0; data.edges.len()];
        for (id, edge) in data.edges.iter().enumerate() {
            let node = owner(edge);
            edges[cursor[node]] = id as u32;
            cursor[node] += 1;
        }
        Self { offsets, edges }
    }

    fn get(&self, node: u32) -> &[u32] {
        let index = node as usize;
        &self.edges[self.offsets[index]..self.offsets[index + 1]]
    }
}

pub struct GraphIndex {
    pub data: Arc<GraphData>,
    identities: HashMap<String, u32>,
    incoming: Adjacency,
    outgoing: Adjacency,
    summaries: SummaryCache,
    classifications: Vec<RelationshipClassification>,
}

impl GraphIndex {
    pub fn new(data: Arc<GraphData>) -> Result<Self> {
        data.validate()?;
        let identities = data
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.clone(), index as u32))
            .collect();
        let incoming = Adjacency::build(&data, Direction::Incoming);
        let outgoing = Adjacency::build(&data, Direction::Outgoing);
        let classifications = data.edges.iter().map(Edge::classification).collect();
        Ok(Self {
            classifications,
            data,
            identities,
            incoming,
            outgoing,
            summaries: SummaryCache::default(),
        })
    }

    /// The classification cache has the same lifetime and ordering as this graph generation.
    pub fn classification(&self, edge: u32) -> RelationshipClassification {
        self.classifications[edge as usize]
    }

    pub fn accepts_edge(&self, edge: u32, filter: &Filter) -> bool {
        let relation = &self.data.edges[edge as usize];
        filter.accepts_classification(self.classification(edge))
            && self.visible(relation.from, filter)
            && self.visible(relation.to, filter)
    }

    pub fn node_index(&self, identity: &str) -> Option<u32> {
        self.identities.get(identity).copied()
    }

    fn visible(&self, node: u32, filter: &Filter) -> bool {
        self.data
            .nodes
            .get(node as usize)
            .is_some_and(|node| filter.accepts_node(node))
    }

    pub fn neighbors(&self, node: u32, direction: Direction, filter: &Filter) -> Vec<u32> {
        if !self.visible(node, filter) {
            return Vec::new();
        }
        let mut neighbors = Vec::new();
        if direction != Direction::Incoming {
            neighbors.extend(self.adjacent_nodes(
                self.outgoing.get(node),
                Direction::Outgoing,
                filter,
            ));
        }
        if direction != Direction::Outgoing {
            neighbors.extend(self.adjacent_nodes(
                self.incoming.get(node),
                Direction::Incoming,
                filter,
            ));
        }
        neighbors.sort_unstable();
        neighbors.dedup();
        neighbors
    }

    pub fn incident_edges(&self, node: u32, direction: Direction, filter: &Filter) -> Vec<u32> {
        if !self.visible(node, filter) {
            return Vec::new();
        }
        let mut edges = Vec::new();
        if direction != Direction::Incoming {
            edges.extend_from_slice(self.outgoing.get(node));
        }
        if direction != Direction::Outgoing {
            edges.extend_from_slice(self.incoming.get(node));
        }
        edges.retain(|&id| self.accepts_edge(id, filter));
        edges.sort_unstable();
        edges.dedup();
        edges
    }

    fn adjacent_nodes<'a>(
        &'a self,
        edges: &'a [u32],
        direction: Direction,
        filter: &'a Filter,
    ) -> impl Iterator<Item = u32> + 'a {
        edges
            .iter()
            .filter(|&&id| self.accepts_edge(id, filter))
            .map(|&id| &self.data.edges[id as usize])
            .map(move |edge| {
                if direction == Direction::Incoming {
                    edge.from
                } else {
                    edge.to
                }
            })
            .filter(|&other| self.visible(other, filter))
    }

    pub fn shortest_path(&self, from: u32, to: u32, filter: &Filter) -> Option<Vec<u32>> {
        if !self.visible(from, filter) || !self.visible(to, filter) {
            return None;
        }
        let mut previous = vec![u32::MAX; self.data.nodes.len()];
        let mut queue = VecDeque::from([from]);
        previous[from as usize] = from;
        while previous[to as usize] == u32::MAX {
            let node = queue.pop_front()?;
            for next in self.neighbors(node, Direction::Outgoing, filter) {
                if previous[next as usize] == u32::MAX {
                    previous[next as usize] = node;
                    queue.push_back(next);
                }
            }
        }
        Some(reconstruct_path(&previous, from, to))
    }

    pub fn neighborhood(&self, request: &Neighborhood<'_>) -> GraphSlice {
        self.multi_neighborhood(&MultiNeighborhood {
            roots: &[request.node],
            direction: request.direction,
            depth: request.depth,
            filter: request.filter,
        })
    }

    /// Breadth-first union of neighborhoods, with roots and siblings ordered by identity.
    pub fn multi_neighborhood(&self, request: &MultiNeighborhood<'_>) -> GraphSlice {
        let MultiNeighborhood {
            roots,
            direction,
            depth,
            filter,
        } = *request;
        let mut frontier = Frontier::new(self.data.nodes.len());
        for root in self.identity_order(
            roots
                .iter()
                .copied()
                .filter(|&id| self.visible(id, filter))
                .collect(),
        ) {
            frontier.enqueue(root, 0);
        }
        let mut reached = Vec::new();
        while let Some((current, distance)) = frontier.queue.pop_front() {
            reached.push(current);
            if distance >= depth {
                continue;
            }
            for next in self.identity_order(self.neighbors(current, direction, filter)) {
                frontier.enqueue(next, distance + 1);
            }
        }
        self.slice_reached(reached, &frontier.visited, filter)
    }

    fn identity_order(&self, mut nodes: Vec<u32>) -> Vec<u32> {
        nodes.sort_unstable_by(|&a, &b| {
            self.data.nodes[a as usize]
                .id
                .cmp(&self.data.nodes[b as usize].id)
        });
        nodes.dedup();
        nodes
    }

    fn slice_reached(&self, reached: Vec<u32>, visited: &[bool], filter: &Filter) -> GraphSlice {
        let omitted_nodes = reached.len().saturating_sub(MAX_CANVAS_NODES);
        let nodes: Vec<u32> = reached.into_iter().take(MAX_CANVAS_NODES).collect();
        let mut shown = vec![false; self.data.nodes.len()];
        for &id in &nodes {
            shown[id as usize] = true;
        }
        let mut edges = Vec::new();
        let mut matching_edges = 0;
        for (id, edge) in self.data.edges.iter().enumerate() {
            if !edge_in_nodes(edge, visited) || !self.accepts_edge(id as u32, filter) {
                continue;
            }
            matching_edges += 1;
            if edges.len() < MAX_CANVAS_EDGES && edge_in_nodes(edge, &shown) {
                edges.push(id as u32);
            }
        }
        GraphSlice {
            nodes,
            omitted_nodes,
            omitted_edges: matching_edges - edges.len(),
            edges,
        }
    }

    pub fn filter_summary(&self, filter: &Filter) -> Arc<FilterSummary> {
        self.summaries.get(filter, || {
            FilterSummary::new(self, filter, self.compute_cycles(filter))
        })
    }

    pub fn cycles(&self, filter: &Filter) -> Vec<Vec<u32>> {
        self.filter_summary(filter).cycles.clone()
    }

    fn compute_cycles(&self, filter: &Filter) -> Vec<Vec<u32>> {
        let mut graph = DiGraph::<u32, ()>::new();
        let indices: Vec<_> = self
            .data
            .nodes
            .iter()
            .enumerate()
            .map(|(id, node)| filter.accepts_node(node).then(|| graph.add_node(id as u32)))
            .collect();
        let edges = self
            .data
            .edges
            .iter()
            .enumerate()
            .filter(|(id, _)| self.accepts_edge(*id as u32, filter))
            .filter_map(|(_, edge)| indices[edge.from as usize].zip(indices[edge.to as usize]));
        for (from, to) in edges {
            graph.add_edge(from, to, ());
        }
        let mut cycles: Vec<Vec<u32>> = kosaraju_scc(&graph)
            .into_iter()
            .filter(|component| {
                component.len() > 1 || graph.contains_edge(component[0], component[0])
            })
            .map(|component| {
                let mut nodes: Vec<u32> = component.into_iter().map(|index| graph[index]).collect();
                nodes.sort_unstable();
                nodes
            })
            .collect();
        cycles.sort();
        cycles
    }
}

fn edge_in_nodes(edge: &Edge, membership: &[bool]) -> bool {
    membership[edge.from as usize] && membership[edge.to as usize]
}

fn reconstruct_path(previous: &[u32], from: u32, to: u32) -> Vec<u32> {
    let mut path = vec![to];
    while *path.last().unwrap() != from {
        path.push(previous[*path.last().unwrap() as usize]);
    }
    path.reverse();
    path
}
