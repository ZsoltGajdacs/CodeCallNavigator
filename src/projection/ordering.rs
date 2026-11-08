//! Deterministic traversal ranks and branch ownership, before pagination.
use super::{FlowDirection, NodeOrder, ViewEdge, ViewNode};
use petgraph::{algo::kosaraju_scc, graph::DiGraph};
use std::collections::VecDeque;

pub(super) fn traversal(
    nodes: &[ViewNode],
    edges: &[ViewEdge],
    mut roots: Vec<usize>,
    direction: FlowDirection,
) -> Vec<(usize, NodeOrder)> {
    let adjacent = adjacency(nodes, edges, direction);
    sort_nodes(&mut roots, nodes);
    if roots.is_empty() {
        roots = component_roots(nodes, &adjacent);
    }
    let mut orders = vec![None; nodes.len()];
    visit(&roots, &adjacent, &mut orders);
    // A manually supplied root can be missing from a bounded page of a graph.
    // Retain every selected component rather than dropping its methods.
    for root in component_roots(nodes, &adjacent) {
        if orders[root].is_none() {
            visit(&[root], &adjacent, &mut orders);
        }
    }
    let orders: Vec<_> = orders
        .into_iter()
        .map(|order| order.expect("every component visited"))
        .collect();
    let sequence = branch_sequence(nodes, &orders);
    let mut result: Vec<_> = orders.into_iter().enumerate().collect();
    result.sort_by_key(|(node, order)| (order.rank, sequence[*node]));
    result
}

fn sort_nodes(indices: &mut Vec<usize>, nodes: &[ViewNode]) {
    indices.sort_by(|&a, &b| nodes[a].key.cmp(&nodes[b].key));
    indices.dedup();
}

fn adjacency(nodes: &[ViewNode], edges: &[ViewEdge], direction: FlowDirection) -> Vec<Vec<usize>> {
    let mut adjacent = vec![Vec::new(); nodes.len()];
    for edge in edges {
        let (parent, child) = match direction {
            FlowDirection::Callees => (edge.from, edge.to),
            FlowDirection::Callers => (edge.to, edge.from),
        };
        adjacent[parent].push(child);
    }
    for children in &mut adjacent {
        sort_nodes(children, nodes);
    }
    adjacent
}

fn component_roots(nodes: &[ViewNode], adjacent: &[Vec<usize>]) -> Vec<usize> {
    let graph = component_graph(nodes.len(), adjacent);
    let components = kosaraju_scc(&graph);
    let mut membership = vec![0; nodes.len()];
    for (component, member) in components
        .iter()
        .enumerate()
        .flat_map(|(i, members)| members.iter().map(move |member| (i, member)))
    {
        membership[member.index()] = component;
    }
    let mut incoming = vec![false; components.len()];
    for (parent, child) in adjacent
        .iter()
        .enumerate()
        .flat_map(|(parent, children)| children.iter().map(move |&child| (parent, child)))
    {
        if membership[parent] != membership[child] {
            incoming[membership[child]] = true;
        }
    }
    let mut roots: Vec<_> = components
        .iter()
        .enumerate()
        .filter(|(i, _)| !incoming[*i])
        .filter_map(|(_, members)| {
            members
                .iter()
                .map(|n| n.index())
                .min_by(|&a, &b| nodes[a].key.cmp(&nodes[b].key))
        })
        .collect();
    sort_nodes(&mut roots, nodes);
    roots
}

fn component_graph(node_count: usize, adjacent: &[Vec<usize>]) -> DiGraph<(), ()> {
    let mut graph = DiGraph::new();
    let indices: Vec<_> = (0..node_count).map(|_| graph.add_node(())).collect();
    for (parent, children) in adjacent.iter().enumerate() {
        for &child in children {
            graph.add_edge(indices[parent], indices[child], ());
        }
    }
    graph
}

fn seed_roots(roots: &[usize], orders: &mut [Option<NodeOrder>]) -> VecDeque<usize> {
    let mut queue = VecDeque::new();
    for &root in roots {
        if orders[root].is_none() {
            orders[root] = Some(NodeOrder::default());
            queue.push_back(root);
        }
    }
    queue
}

fn visit(roots: &[usize], adjacent: &[Vec<usize>], orders: &mut [Option<NodeOrder>]) {
    let mut queue = seed_roots(roots, orders);
    while let Some(parent) = queue.pop_front() {
        let rank = orders[parent]
            .as_ref()
            .expect("queued node has a rank")
            .rank
            + 1;
        for &child in &adjacent[parent] {
            if orders[child].is_none() {
                orders[child] = Some(NodeOrder {
                    rank,
                    parent: Some(parent),
                });
                queue.push_back(child);
            }
        }
    }
}

fn branch_sequence(nodes: &[ViewNode], orders: &[NodeOrder]) -> Vec<usize> {
    let mut children = vec![Vec::new(); nodes.len()];
    let mut roots = Vec::new();
    for (node, order) in orders.iter().enumerate() {
        match order.parent {
            Some(parent) => children[parent].push(node),
            None => roots.push(node),
        }
    }
    sort_nodes(&mut roots, nodes);
    for siblings in &mut children {
        sort_nodes(siblings, nodes);
    }
    let mut sequence = vec![0; nodes.len()];
    let mut stack: Vec<_> = roots.into_iter().rev().collect();
    let mut position = 0;
    while let Some(node) = stack.pop() {
        sequence[node] = position;
        position += 1;
        stack.extend(children[node].iter().rev().copied());
    }
    sequence
}
