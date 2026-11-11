//! Resolve overview, method-root neighborhoods and directed paths.
use super::{FlowDirection, Grouping, Request, Scope, Selection};
use codecallnavigator::graph::{Filter, GraphIndex, MultiNeighborhood};
use std::collections::HashSet;

pub(super) fn resolve(graph: &GraphIndex, request: &Request) -> Selection {
    match request.scope {
        Scope::Path(from, to) => path(graph, from, to, &request.filter),
        Scope::Focus(node) => neighborhood(graph, request, vec![node], "Focused neighborhood"),
        Scope::Overview if request.grouping == Grouping::Method => methods(graph, request),
        Scope::Overview => all(graph, request),
    }
}

fn methods(graph: &GraphIndex, request: &Request) -> Selection {
    let roots = match &graph.data.entry_points {
        Some(entries) => entries
            .candidates
            .iter()
            .chain(&entries.application_starts)
            .chain(&entries.framework_entries)
            .filter_map(|id| graph.node_index(id))
            .collect(),
        None => legacy_roots(graph),
    };
    let mut selection = neighborhood(graph, request, roots, "Entry-point methods");
    if selection.roots.is_empty() {
        selection.message =
            "No entry-point methods match this graph and its filters · search and focus a method"
                .into();
    }
    if graph.data.entry_points.is_none() {
        selection
            .message
            .push_str(" · Refresh to enable application-start highlighting");
    }
    selection
}

fn legacy_roots(graph: &GraphIndex) -> Vec<u32> {
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

fn neighborhood(
    graph: &GraphIndex,
    request: &Request,
    mut roots: Vec<u32>,
    label: &str,
) -> Selection {
    roots.retain(|&id| {
        graph
            .data
            .nodes
            .get(id as usize)
            .is_some_and(|n| request.filter.accepts_node(n))
    });
    roots.sort_by(|&a, &b| {
        graph.data.nodes[a as usize]
            .id
            .cmp(&graph.data.nodes[b as usize].id)
    });
    roots.dedup();
    let slice = graph.multi_neighborhood(&MultiNeighborhood {
        roots: &roots,
        direction: request.direction.into(),
        depth: request.depth,
        filter: &request.filter,
    });
    Selection {
        nodes: slice.nodes,
        edges: slice.edges,
        roots,
        direction: request.direction,
        message: format!("{label} · depth {}", request.depth),
        omitted_nodes: slice.omitted_nodes,
        omitted_edges: slice.omitted_edges,
    }
}

fn path(graph: &GraphIndex, from: u32, to: u32, filter: &Filter) -> Selection {
    let Some(nodes) = graph.shortest_path(from, to, filter) else {
        return Selection {
            nodes: Vec::new(),
            edges: Vec::new(),
            roots: Vec::new(),
            direction: FlowDirection::Callees,
            message: "No directed path in the analyzed graph with these filters".into(),
            omitted_nodes: 0,
            omitted_edges: 0,
        };
    };
    let pairs: HashSet<_> = nodes.windows(2).map(|pair| (pair[0], pair[1])).collect();
    let edges = graph
        .data
        .edges
        .iter()
        .enumerate()
        .filter(|(_, e)| pairs.contains(&(e.from, e.to)) && filter.accepts_edge(e))
        .map(|(i, _)| i as u32)
        .collect();
    let message = format!(
        "Shortest directed path · {} steps",
        nodes.len().saturating_sub(1)
    );
    Selection {
        nodes,
        edges,
        roots: vec![from],
        direction: FlowDirection::Callees,
        message,
        omitted_nodes: 0,
        omitted_edges: 0,
    }
}

pub(super) fn all(graph: &GraphIndex, request: &Request) -> Selection {
    Selection {
        nodes: graph
            .data
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| request.filter.accepts_node(n))
            .map(|(i, _)| i as u32)
            .collect(),
        edges: graph
            .data
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| request.filter.accepts_edge(e))
            .map(|(i, _)| i as u32)
            .collect(),
        roots: Vec::new(),
        direction: request.direction,
        message: "Whole workspace · expand groups to explore methods".into(),
        omitted_nodes: 0,
        omitted_edges: 0,
    }
}
