//! Layer visibility and presentation order, without changing the analyzed graph.
use super::{GraphView, Grouping, LayerBand, NodeOrder, Request, Scope, Selection, ViewNode};
use codecallnavigator::{architecture::file_key, graph::GraphIndex, model::Node};
use std::collections::{HashMap, HashSet};

pub(super) fn active(request: &Request) -> bool {
    request.grouping == Grouping::Layer && request.scope == Scope::Overview
}

pub(super) fn assignment<'a>(node: &Node, request: &'a Request) -> Option<&'a str> {
    if request.grouping != Grouping::Layer {
        return None;
    }
    request
        .architecture
        .as_ref()?
        .files
        .get(&file_key(&node.build, &node.project, &node.file))
        .map(|assignment| assignment.layer.as_str())
}

pub(super) fn select(graph: &GraphIndex, request: &Request, selection: &mut Selection) {
    if !active(request) {
        return;
    }
    let Some(architecture) = &request.architecture else {
        selection.nodes.clear();
        selection.edges.clear();
        selection.message = "Choose a layer preset or repository configuration. Refresh an older graph to collect architectural facts.".into();
        return;
    };
    let tabs: HashMap<_, _> = architecture
        .layers
        .iter()
        .map(|layer| (layer.name.as_str(), layer.tab.as_deref()))
        .collect();
    let visible: HashSet<_> = selection.nodes.iter().copied().collect();
    selection
        .edges
        .retain(|&index| contains_edge(graph, &visible, index));
    let before = selection.nodes.len();
    selection.nodes.retain(|&index| {
        let layer = assignment(&graph.data.nodes[index as usize], request).unwrap_or("Other");
        tabs.get(layer).copied().flatten() == request.layer_tab.as_deref()
    });
    let included: HashSet<_> = selection.nodes.iter().copied().collect();
    let edges_before = selection.edges.len();
    selection
        .edges
        .retain(|&index| contains_edge(graph, &included, index));
    selection.omitted_nodes += before - selection.nodes.len();
    selection.omitted_edges += edges_before - selection.edges.len();
}

fn contains_edge(graph: &GraphIndex, nodes: &HashSet<u32>, index: u32) -> bool {
    let edge = &graph.data.edges[index as usize];
    nodes.contains(&edge.from) && nodes.contains(&edge.to)
}

pub(super) fn bands(request: &Request) -> Vec<LayerBand> {
    if !active(request) {
        return Vec::new();
    }
    let Some(architecture) = &request.architecture else {
        return Vec::new();
    };
    let mut bands: Vec<_> = architecture
        .layers
        .iter()
        .filter(|layer| layer.tab == request.layer_tab)
        .map(|layer| LayerBand {
            name: layer.name.clone(),
            flow: layer.flow.clone(),
            support: layer.support,
            beside: layer.beside.clone(),
            ..Default::default()
        })
        .collect();
    if request.layer_tab.is_none() {
        for name in ["Tests", "Other"] {
            if !bands.iter().any(|band| band.name == name) {
                bands.push(LayerBand {
                    name: name.into(),
                    ..Default::default()
                });
            }
        }
    }
    bands
}

pub(super) fn order(
    nodes: &[ViewNode],
    mut ordered: Vec<(usize, NodeOrder)>,
    request: &Request,
) -> Vec<(usize, NodeOrder)> {
    if !active(request) {
        return ordered;
    }
    let ranks: HashMap<_, _> = bands(request)
        .into_iter()
        .enumerate()
        .map(|(index, band)| (band.name, index))
        .collect();
    ordered.sort_by(|(left, _), (right, _)| {
        let rank = |index: usize| {
            nodes[index]
                .layer
                .as_ref()
                .and_then(|layer| ranks.get(layer))
                .copied()
                .unwrap_or(usize::MAX)
        };
        (rank(*left), &nodes[*left].key).cmp(&(rank(*right), &nodes[*right].key))
    });
    for (index, order) in &mut ordered {
        order.rank = nodes[*index]
            .layer
            .as_ref()
            .and_then(|layer| ranks.get(layer))
            .copied()
            .unwrap_or(ranks.len()) as i32;
        order.parent = None;
    }
    ordered
}

pub(super) fn measure(view: &mut GraphView) {
    if view.layer_bands.is_empty() {
        return;
    }
    reserve_headers(view);
    for band in &mut view.layer_bands {
        let members: Vec<_> = view
            .nodes
            .iter()
            .filter(|node| node.layer.as_deref() == Some(&band.name))
            .collect();
        if members.is_empty() {
            continue;
        }
        let left = members
            .iter()
            .map(|node| node.position[0])
            .fold(f32::INFINITY, f32::min)
            - 12.0;
        let top = members
            .iter()
            .map(|node| node.position[1])
            .fold(f32::INFINITY, f32::min)
            - 28.0;
        let right = members
            .iter()
            .map(|node| node.position[0] + node.size[0])
            .fold(0.0, f32::max)
            + 12.0;
        let bottom = members
            .iter()
            .map(|node| node.position[1] + node.size[1])
            .fold(0.0, f32::max)
            + 12.0;
        band.position = [left, top];
        band.size = [right - left, bottom - top];
    }
    view.layer_bands.retain(|band| band.size[0] > 0.0);
}

fn reserve_headers(view: &mut GraphView) {
    for node in &mut view.nodes {
        node.position[1] += 24.0;
    }
    for edge in &mut view.edges {
        for point in &mut edge.points {
            point[1] += 24.0;
        }
    }
    view.size[1] += 24.0;
}
