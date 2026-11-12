//! Map presentation requests to public CLI semantic operations and prepare display metadata.
use super::{FlowDirection, Grouping, Request, Scope, Selection, Summary};
use codecallnavigator::{graph::GraphIndex, query::SelectionKind};

pub(crate) fn kind(graph: &GraphIndex, request: &Request) -> SelectionKind {
    if super::focus_target(graph, request).is_some() {
        return SelectionKind::Overview;
    }
    match request.scope {
        Scope::Path(from, to) => SelectionKind::Path {
            from: graph.data.nodes[from as usize].id.clone(),
            to: graph.data.nodes[to as usize].id.clone(),
        },
        Scope::Focus(node) => SelectionKind::Neighborhood {
            symbols: vec![graph.data.nodes[node as usize].id.clone()],
            direction: request.direction.into(),
            depth: request.depth,
        },
        Scope::Overview if request.grouping == Grouping::Method => SelectionKind::EntryPoints {
            direction: request.direction.into(),
            depth: request.depth,
        },
        Scope::Overview => SelectionKind::Overview,
    }
}

pub(crate) fn prepared(
    graph: &GraphIndex,
    request: &Request,
    selected: codecallnavigator::query::Selection,
    summary: Summary,
) -> Selection {
    let message = message(graph, request, &selected);
    Selection {
        nodes: selected.nodes,
        edges: selected.edges,
        roots: selected.roots,
        direction: if matches!(request.scope, Scope::Path(..)) {
            FlowDirection::Callees
        } else {
            request.direction
        },
        message,
        omitted_nodes: selected.omitted_nodes,
        omitted_edges: selected.omitted_edges,
        summary,
        classifications: Default::default(),
    }
}

fn message(
    graph: &GraphIndex,
    request: &Request,
    selected: &codecallnavigator::query::Selection,
) -> String {
    match request.scope {
        Scope::Path(..) if selected.nodes.is_empty() => {
            "No directed path in the analyzed graph with these filters".into()
        }
        Scope::Path(..) => format!(
            "Shortest directed path · {} steps",
            selected.nodes.len().saturating_sub(1)
        ),
        Scope::Focus(..) => format!("Focused neighborhood · depth {}", request.depth),
        Scope::Overview => overview_message(graph, request, selected),
    }
}

fn overview_message(
    graph: &GraphIndex,
    request: &Request,
    selected: &codecallnavigator::query::Selection,
) -> String {
    match request.grouping {
        Grouping::Layer if request.architecture.is_none() => super::LAYER_NOTICE.into(),
        Grouping::Method => entry_message(graph, request, selected),
        _ => "Whole workspace · expand groups to explore methods".into(),
    }
}

fn entry_message(
    graph: &GraphIndex,
    request: &Request,
    selected: &codecallnavigator::query::Selection,
) -> String {
    let mut message = if selected.roots.is_empty() {
        "No entry-point methods match this graph and its filters · search and focus a method".into()
    } else {
        format!("Entry-point methods · depth {}", request.depth)
    };
    if graph.data.entry_points.is_none() {
        message.push_str(" · Refresh to enable application-start highlighting");
    }
    message
}

pub(crate) fn fixture(graph: &GraphIndex, request: &Request) -> Selection {
    let selected =
        codecallnavigator::query::select_render(graph, &kind(graph, request), &request.filter)
            .expect("valid fixture query");
    let source = graph.filter_summary(&request.filter);
    let summary = Summary {
        cyclic: source
            .cyclic
            .iter()
            .enumerate()
            .filter(|(_, cyclic)| **cyclic)
            .map(|(id, _)| id as u32)
            .collect(),
        hidden_nodes: source.hidden_nodes,
        hidden_edges: source.hidden_edges,
    };
    let mut prepared = prepared(graph, request, selected, summary);
    prepared.classifications = prepared
        .edges
        .iter()
        .map(|id| (*id, graph.classification(*id)))
        .collect();
    let mut visibility = request.clone();
    if super::focus_target(graph, request).is_some() {
        visibility.scope = Scope::Overview;
    }
    super::layers::select(graph, &visibility, &mut prepared);
    if super::needs_overview_fallback(graph, request, &prepared) {
        let mut overview = request.clone();
        overview.scope = Scope::Overview;
        overview.focus_target = None;
        return fixture(graph, &overview);
    }
    prepared
}
