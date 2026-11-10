//! Two directed traversals around presentation roots; independent of canvas geometry.
use super::{NodeOrder, ViewEdge, ViewNode};
use std::collections::{HashSet, VecDeque};

type Reach = Option<(u32, Option<usize>)>;

pub(super) fn traversal(
    nodes: &[ViewNode],
    edges: &[ViewEdge],
    roots: &[usize],
    depth: u32,
) -> Vec<(usize, NodeOrder)> {
    let incoming = distances(&adjacency(nodes.len(), edges, true), roots, depth);
    let outgoing = distances(&adjacency(nodes.len(), edges, false), roots, depth);
    let mut ordered: Vec<_> = (0..nodes.len())
        .filter_map(|node| {
            let (rank, parent) = match (incoming[node], outgoing[node]) {
                (Some((a, p)), Some((b, _))) if a <= b => (-(a as i32), p),
                (_, Some((b, p))) => (b as i32, p),
                (Some((a, p)), None) => (-(a as i32), p),
                _ => return None,
            };
            Some((node, NodeOrder { rank, parent }))
        })
        .collect();
    sort_branches(nodes, &mut ordered);
    ordered
}

fn adjacency(count: usize, edges: &[ViewEdge], incoming: bool) -> Vec<Vec<usize>> {
    let mut adjacent = vec![Vec::new(); count];
    for edge in edges {
        let (from, to) = if incoming {
            (edge.to, edge.from)
        } else {
            (edge.from, edge.to)
        };
        adjacent[from].push(to);
    }
    // Aggregate indices are already sorted by stable identity.
    for neighbors in &mut adjacent {
        neighbors.sort_unstable();
        neighbors.dedup();
    }
    adjacent
}

fn distances(adjacent: &[Vec<usize>], roots: &[usize], depth: u32) -> Vec<Reach> {
    let mut reaches = vec![None; adjacent.len()];
    let mut queue = VecDeque::new();
    for &root in roots {
        reaches[root] = Some((0, None));
        queue.push_back(root);
    }
    while let Some(parent) = queue.pop_front() {
        let distance = reaches[parent].expect("queued reach").0;
        if distance >= depth {
            continue;
        }
        discover(&adjacent[parent], parent, &mut reaches, &mut queue);
    }
    reaches
}

fn branch_sequence(nodes: &[ViewNode], ordered: &[(usize, NodeOrder)]) -> Vec<usize> {
    let mut children = vec![Vec::new(); nodes.len()];
    let mut roots = Vec::new();
    for (node, order) in ordered {
        if let Some(parent) = order.parent {
            children[parent].push(*node);
        } else {
            roots.push(*node);
        }
    }
    roots.sort_by_key(|&i| &nodes[i].key);
    for siblings in &mut children {
        siblings.sort_by_key(|&i| &nodes[i].key);
    }
    let mut stack: Vec<_> = roots.into_iter().rev().collect();
    let mut sequence = vec![0; nodes.len()];
    let mut position = 0;
    while let Some(node) = stack.pop() {
        sequence[node] = position;
        position += 1;
        stack.extend(children[node].iter().rev());
    }
    sequence
}

fn discover(children: &[usize], parent: usize, reaches: &mut [Reach], queue: &mut VecDeque<usize>) {
    let distance = reaches[parent].expect("discovered parent").0 + 1;
    for &child in children {
        if reaches[child].is_none() {
            reaches[child] = Some((distance, Some(parent)));
            queue.push_back(child);
        }
    }
}

/// Preserve the internal class call tree while placing external context around it.
pub(super) fn class_methods(
    nodes: &[ViewNode],
    edges: &[ViewEdge],
    roots: &[usize],
    depth: u32,
) -> Vec<(usize, NodeOrder)> {
    let members: HashSet<_> = roots.iter().copied().collect();
    let internal_edges: Vec<_> = edges
        .iter()
        .filter(|edge| members.contains(&edge.from) && members.contains(&edge.to))
        .cloned()
        .collect();
    let internal = super::ordering::traversal(
        nodes,
        &internal_edges,
        Vec::new(),
        super::FlowDirection::Callees,
    );
    let mut internal_orders = vec![NodeOrder::default(); nodes.len()];
    let mut max_rank = 0;
    for (node, order) in internal
        .into_iter()
        .filter(|(node, _)| members.contains(node))
    {
        max_rank = max_rank.max(order.rank);
        internal_orders[node] = order;
    }
    let mut ordered = traversal(nodes, edges, roots, depth);
    for (node, order) in &mut ordered {
        if members.contains(node) {
            *order = internal_orders[*node].clone();
        } else if order.rank > 0 {
            order.rank += max_rank;
        }
    }
    sort_branches(nodes, &mut ordered);
    ordered
}

fn sort_branches(nodes: &[ViewNode], ordered: &mut [(usize, NodeOrder)]) {
    let sequence = branch_sequence(nodes, ordered);
    ordered.sort_by_key(|(node, order)| (order.rank, sequence[*node]));
}
