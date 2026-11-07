use crate::model::{Edge, GraphData, Node};
use anyhow::Result;
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Filter {
    pub include_tests: bool,
    pub include_generated: bool,
    pub include_unavailable: bool,
    pub include_uncertain: bool,
    pub include_references: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            include_tests: false,
            include_generated: false,
            include_unavailable: false,
            include_uncertain: true,
            include_references: true,
        }
    }
}

impl Filter {
    pub fn accepts_node(&self, node: &Node) -> bool {
        (self.include_tests || !node.is_test)
            && (self.include_generated || !node.is_generated)
            && (self.include_unavailable || !node.body_unavailable)
    }

    pub fn accepts_edge(&self, edge: &Edge) -> bool {
        (self.include_uncertain || !edge.is_uncertain())
            && (self.include_references || !edge.is_reference())
    }
}

/// Bounded traversal centered on one method.
pub struct Neighborhood<'a> {
    pub node: u32,
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
    fn new(node_count: usize, start: u32) -> Self {
        let mut frontier = Self {
            visited: vec![false; node_count],
            queue: VecDeque::new(),
        };
        frontier.enqueue(start, 0);
        frontier
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
        Ok(Self {
            data,
            identities,
            incoming,
            outgoing,
        })
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
        edges.retain(|&id| {
            let edge = &self.data.edges[id as usize];
            filter.accepts_edge(edge)
                && self.visible(edge.from, filter)
                && self.visible(edge.to, filter)
        });
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
            .map(|&id| &self.data.edges[id as usize])
            .filter(|edge| filter.accepts_edge(edge))
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
        let Neighborhood {
            node,
            direction,
            depth,
            filter,
        } = *request;
        if !self.visible(node, filter) {
            return GraphSlice::default();
        }
        let mut frontier = Frontier::new(self.data.nodes.len(), node);
        let mut reached = Vec::new();
        while let Some((current, distance)) = frontier.queue.pop_front() {
            reached.push(current);
            if distance >= depth {
                continue;
            }
            for next in self.neighbors(current, direction, filter) {
                frontier.enqueue(next, distance + 1);
            }
        }
        self.slice_reached(reached, &frontier.visited, filter)
    }

    fn slice_reached(&self, reached: Vec<u32>, visited: &[bool], filter: &Filter) -> GraphSlice {
        let omitted_nodes = reached.len().saturating_sub(MAX_CANVAS_NODES);
        let mut nodes: Vec<u32> = reached.into_iter().take(MAX_CANVAS_NODES).collect();
        nodes.sort_unstable();
        let mut shown = vec![false; self.data.nodes.len()];
        for &id in &nodes {
            shown[id as usize] = true;
        }
        let mut edges = Vec::new();
        let mut matching_edges = 0;
        for (id, edge) in self.data.edges.iter().enumerate() {
            if !edge_in_nodes(edge, visited) || !filter.accepts_edge(edge) {
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

    pub fn cycles(&self, filter: &Filter) -> Vec<Vec<u32>> {
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
            .filter(|edge| filter.accepts_edge(edge))
            .filter_map(|edge| indices[edge.from as usize].zip(indices[edge.to as usize]));
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
