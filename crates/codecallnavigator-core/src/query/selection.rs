//! Semantic graph selection, independent of canvas grouping and layout.
use super::{Error, Limits, Result, SelectionKind};
use crate::graph::{Direction, Filter, GraphIndex, MultiNeighborhood};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Selection {
    pub nodes: Vec<u32>,
    pub edges: Vec<u32>,
    pub roots: Vec<u32>,
    pub omitted_nodes: usize,
    pub omitted_edges: usize,
}

pub fn select(
    graph: &GraphIndex,
    kind: &SelectionKind,
    filter: &Filter,
    limits: Limits,
) -> Result<Selection> {
    if !(1..=1500).contains(&limits.nodes) || !(1..=6000).contains(&limits.edges) {
        return Err(Error::new(
            "invalid_request",
            "Limits require 1..=1500 nodes and 1..=6000 edges",
        ));
    }
    let mut selected = select_render(graph, kind, filter)?;
    selected.omitted_nodes += selected.nodes.len().saturating_sub(limits.nodes);
    selected.nodes.truncate(limits.nodes);
    let retained: HashSet<_> = selected.nodes.iter().copied().collect();
    selected.roots.retain(|id| retained.contains(id));
    let before = selected.edges.len();
    selected.edges.retain(|id| {
        let edge = &graph.data.edges[*id as usize];
        retained.contains(&edge.from) && retained.contains(&edge.to)
    });
    selected.edges.sort_by(|a, b| {
        let key = |id: &u32| {
            let e = &graph.data.edges[*id as usize];
            (
                &graph.data.nodes[e.from as usize].id,
                &graph.data.nodes[e.to as usize].id,
                &e.kind,
                &e.certainty,
                &e.provenance,
            )
        };
        key(a).cmp(&key(b))
    });
    selected.edges.truncate(limits.edges);
    selected.omitted_edges += before - selected.edges.len();
    Ok(selected)
}

/// Complete filtered membership for rendering. Overview/file/class selection
/// must remain complete for grouping and pagination; neighborhood traversal
/// keeps GraphIndex's intrinsic safety bounds and omission counts.
pub fn select_render(
    graph: &GraphIndex,
    kind: &SelectionKind,
    filter: &Filter,
) -> Result<Selection> {
    let mut selected = unbounded(graph, kind, filter)?;
    // Preserve traversal/path order and root priority for both consumers.
    let mut seen = HashSet::new();
    selected.nodes.retain(|id| seen.insert(*id));
    Ok(selected)
}

pub(super) fn node(graph: &GraphIndex, symbol: &str) -> Result<u32> {
    graph.node_index(symbol).ok_or_else(|| {
        Error::new(
            "symbol_missing",
            format!("No symbol {symbol} in this generation"),
        )
    })
}

fn unbounded(graph: &GraphIndex, kind: &SelectionKind, filter: &Filter) -> Result<Selection> {
    match kind {
        SelectionKind::Overview => Ok(all(graph, filter, |_| true)),
        SelectionKind::File { file } => Ok(all(graph, filter, |node| node.file == *file)),
        SelectionKind::Class {
            build,
            project,
            container,
        } => Ok(all(graph, filter, |node| {
            node.build == *build && node.project == *project && node.container == *container
        })),
        SelectionKind::EntryPoints { direction, depth } => neighborhood(
            graph,
            entry_roots(graph),
            Walk {
                direction: *direction,
                depth: *depth,
            },
            filter,
        ),
        SelectionKind::Neighborhood {
            symbols,
            direction,
            depth,
        } => neighborhood(
            graph,
            symbols
                .iter()
                .map(|id| node(graph, id))
                .collect::<Result<Vec<_>>>()?,
            Walk {
                direction: *direction,
                depth: *depth,
            },
            filter,
        ),
        SelectionKind::Path { from, to } => {
            let from = node(graph, from)?;
            let to = node(graph, to)?;
            let nodes = graph.shortest_path(from, to, filter).unwrap_or_default();
            let pairs: HashSet<_> = nodes.windows(2).map(|w| (w[0], w[1])).collect();
            let edges = graph
                .data
                .edges
                .iter()
                .enumerate()
                .filter(|(id, e)| {
                    pairs.contains(&(e.from, e.to)) && graph.accepts_edge(*id as u32, filter)
                })
                .map(|(id, _)| id as u32)
                .collect();
            Ok(Selection {
                roots: if nodes.is_empty() { vec![] } else { vec![from] },
                nodes,
                edges,
                ..Default::default()
            })
        }
    }
}

fn all(
    graph: &GraphIndex,
    filter: &Filter,
    accepts: impl Fn(&crate::model::Node) -> bool,
) -> Selection {
    let mut nodes: Vec<_> = graph
        .data
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| filter.accepts_node(n) && accepts(n))
        .map(|(i, _)| i as u32)
        .collect();
    nodes.sort_by(|a, b| {
        graph.data.nodes[*a as usize]
            .id
            .cmp(&graph.data.nodes[*b as usize].id)
    });
    let included: HashSet<_> = nodes.iter().copied().collect();
    let edges = graph
        .data
        .edges
        .iter()
        .enumerate()
        .filter(|(i, e)| {
            included.contains(&e.from)
                && included.contains(&e.to)
                && graph.accepts_edge(*i as u32, filter)
        })
        .map(|(i, _)| i as u32)
        .collect();
    Selection {
        nodes,
        edges,
        ..Default::default()
    }
}

struct Walk {
    direction: Direction,
    depth: u32,
}

fn neighborhood(
    graph: &GraphIndex,
    mut roots: Vec<u32>,
    walk: Walk,
    filter: &Filter,
) -> Result<Selection> {
    let Walk { direction, depth } = walk;
    if depth > 64 {
        return Err(Error::new(
            "invalid_request",
            "Maximum traversal depth is 64",
        ));
    }
    roots.retain(|id| filter.accepts_node(&graph.data.nodes[*id as usize]));
    roots.sort_by(|a, b| {
        graph.data.nodes[*a as usize]
            .id
            .cmp(&graph.data.nodes[*b as usize].id)
    });
    roots.dedup();
    let slice = graph.multi_neighborhood(&MultiNeighborhood {
        roots: &roots,
        direction,
        depth,
        filter,
    });
    Ok(Selection {
        nodes: slice.nodes,
        edges: slice.edges,
        roots,
        omitted_nodes: slice.omitted_nodes,
        omitted_edges: slice.omitted_edges,
    })
}

fn entry_roots(graph: &GraphIndex) -> Vec<u32> {
    if let Some(entries) = &graph.data.entry_points {
        return entries
            .candidates
            .iter()
            .chain(&entries.application_starts)
            .chain(&entries.framework_entries)
            .filter_map(|id| graph.node_index(id))
            .collect();
    }
    let called: HashSet<_> = graph.data.edges.iter().map(|e| e.to).collect();
    graph
        .data
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, n)| {
            !called.contains(&(*i as u32))
                && !n.body_unavailable
                && !matches!(
                    n.kind.as_str(),
                    "field" | "fieldinit" | "toplevel" | "initializer" | "inherited-reference"
                )
        })
        .map(|(i, _)| i as u32)
        .collect()
}
