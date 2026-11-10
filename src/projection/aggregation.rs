//! Aggregate analyzed relationships, order them, then select a bounded page.
use super::{
    EDGE_LIMIT, FocusTarget, GraphView, GroupKind, Grouping, MethodRow, NODE_LIMIT, NodeOrder,
    Request, Scope, Selection, SourcePath, ViewEdge, ViewNode,
};
use super::{
    focus_level::{FocusProjection, PresentationLevel},
    groups::{Group, groups, relative_file},
    labels, layers, ordering,
};
use codegraph::{graph::GraphIndex, model::Node};
use std::collections::{BTreeMap, HashMap, HashSet};

fn group_node(node: &Node, request: &Request, group: Group) -> ViewNode {
    let display = if group.kind == GroupKind::Class {
        labels::class(node)
    } else {
        labels::Label {
            text: group.label.clone(),
            description: group.label,
        }
    };
    ViewNode {
        key: group.key,
        label: display.text,
        description: display.description,
        group_kind: Some(group.kind),
        detail: node.project.clone(),
        layer: layers::assignment(node, request).map(str::to_owned),
        ..Default::default()
    }
}

fn visible_node(
    node: &Node,
    index: u32,
    request: &Request,
    level: Option<PresentationLevel>,
) -> ViewNode {
    let group = match level {
        Some(level) => level.representative(node, request),
        None if request.scope == Scope::Overview && request.grouping != Grouping::Method => {
            groups(node, request)
                .into_iter()
                .find(|group| !request.expanded.contains(&group.key))
        }
        None => None,
    };
    if let Some(group) = group {
        return group_node(node, request, group);
    }
    let display = labels::method(node);
    ViewNode {
        key: node.id.clone(),
        label: display.text,
        description: display.description,
        detail: node.container.clone(),
        member: Some(index),
        layer: layers::assignment(node, request).map(str::to_owned),
        ..Default::default()
    }
}

struct Aggregates {
    nodes: Vec<ViewNode>,
    edges: Vec<ViewEdge>,
    members: HashMap<u32, usize>,
    collapsed_edges: usize,
}

fn aggregate_nodes(
    graph: &GraphIndex,
    request: &Request,
    selection: &Selection,
    level: Option<PresentationLevel>,
) -> Aggregates {
    let summary = graph.filter_summary(&request.filter);
    let starts: HashSet<_> = graph
        .data
        .entry_points
        .as_ref()
        .into_iter()
        .flat_map(|entries| &entries.application_starts)
        .collect();
    let mut representatives = BTreeMap::new();
    let mut members = HashMap::new();
    for &index in &selection.nodes {
        let mut node = visible_node(&graph.data.nodes[index as usize], index, request, level);
        node.application_start = node.member.is_some() && starts.contains(&node.key);
        members.insert(index, node.key.clone());
        let representative = representatives.entry(node.key.clone()).or_insert(node);
        add_member_metadata(representative, &graph.data.nodes[index as usize], index);
        representative.count += 1;
        representative.cyclic |= summary.cyclic[index as usize];
    }
    for node in representatives.values_mut() {
        finish_metadata(node);
    }
    let indices: HashMap<_, _> = representatives
        .keys()
        .enumerate()
        .map(|(i, key)| (key.clone(), i))
        .collect();
    Aggregates {
        nodes: representatives.into_values().collect(),
        edges: Vec::new(),
        collapsed_edges: 0,
        members: members
            .into_iter()
            .map(|(member, key)| (member, indices[&key]))
            .collect(),
    }
}

fn add_member_metadata(representative: &mut ViewNode, source: &Node, index: u32) {
    if representative.group_kind.is_some() && !source.file.is_empty() {
        representative.files.push(SourcePath {
            label: relative_file(source).to_string_lossy().into_owned(),
            path: source.file.clone(),
            language: source.language.clone(),
        });
    }
    if representative.group_kind == Some(GroupKind::Class) {
        let display = labels::method(source);
        representative.methods.push(MethodRow {
            member: index,
            id: source.id.clone(),
            name: source.label.clone(),
            label: display.text,
            description: display.description,
        });
    }
}

fn finish_metadata(node: &mut ViewNode) {
    node.methods
        .sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
    let rows = node.methods.len().min(12) as f32;
    node.size = [
        237.6,
        61.2 + if rows > 0.0 { 24.0 * rows + 8.0 } else { 0.0 },
    ];
    node.files.sort();
    node.files.dedup_by(|a, b| a.path == b.path);
}

fn aggregate_edges(graph: &GraphIndex, selection: &Selection, aggregates: &mut Aggregates) {
    let mut relations = BTreeMap::new();
    for &index in &selection.edges {
        let edge = &graph.data.edges[index as usize];
        let Some((&from, &to)) = aggregates
            .members
            .get(&edge.from)
            .zip(aggregates.members.get(&edge.to))
        else {
            continue;
        };
        if from == to && aggregates.nodes[from].member.is_none() {
            aggregates.collapsed_edges += 1;
            aggregates.nodes[from].internal_edges += 1;
            continue;
        }
        *relations
            .entry((from, to, edge.kind.clone(), edge.certainty.clone()))
            .or_insert(0_u64) += u64::from(edge.count);
    }
    aggregates.edges = relations
        .into_iter()
        .map(|((from, to, kind, certainty), count)| ViewEdge {
            from,
            to,
            kind,
            certainty,
            count,
            ..Default::default()
        })
        .collect();
}

pub(super) fn project_page(
    graph: &GraphIndex,
    request: &Request,
    selection: Selection,
) -> GraphView {
    let mut aggregates = aggregate_nodes(graph, request, &selection, None);
    aggregate_edges(graph, &selection, &mut aggregates);
    let roots = selection
        .roots
        .iter()
        .filter_map(|root| aggregates.members.get(root).copied())
        .collect();
    let ordered = ordering::traversal(
        &aggregates.nodes,
        &aggregates.edges,
        roots,
        selection.direction,
    );
    let ordered = layers::order(&aggregates.nodes, ordered, request);
    let mut view = page(aggregates, ordered, request.page, &[]);
    view.layer_bands = layers::bands(request);
    let summary = graph.filter_summary(&request.filter);
    (view.hidden_nodes, view.hidden_edges) = (summary.hidden_nodes, summary.hidden_edges);
    view.omitted_nodes += selection.omitted_nodes;
    view.omitted_edges += selection.omitted_edges;
    view.message = selection.message;
    view
}

pub(super) fn focused(graph: &GraphIndex, request: &Request, target: &FocusTarget) -> GraphView {
    let mut all_request = request.clone();
    all_request.scope = Scope::Overview;
    all_request.focus_target = None;
    let mut selection = super::selection::all(graph, &all_request);
    layers::select(graph, &all_request, &mut selection);
    let Some(focus) = FocusProjection::resolve(graph, &all_request, &selection.nodes, target)
    else {
        return super::project(graph, &all_request);
    };
    let mut aggregates = aggregate_nodes(graph, &all_request, &selection, Some(focus.level));
    aggregate_edges(graph, &selection, &mut aggregates);
    let mut roots: Vec<_> = focus
        .members
        .iter()
        .filter_map(|index| aggregates.members.get(index).copied())
        .collect();
    roots.sort_unstable();
    roots.dedup();
    let focus_keys = roots
        .iter()
        .map(|&i| aggregates.nodes[i].key.clone())
        .collect();
    let ordered = if matches!(target, FocusTarget::ClassMethods(_)) {
        super::focus::class_methods(&aggregates.nodes, &aggregates.edges, &roots, request.depth)
    } else {
        super::focus::traversal(&aggregates.nodes, &aggregates.edges, &roots, request.depth)
    };
    let visible: HashSet<_> = ordered.iter().map(|(node, _)| *node).collect();
    aggregates
        .edges
        .retain(|edge| visible.contains(&edge.from) && visible.contains(&edge.to));
    aggregates.collapsed_edges = visible
        .iter()
        .map(|&index| aggregates.nodes[index].internal_edges)
        .sum();
    let mut view = page(aggregates, ordered, request.page, &roots);
    view.focus_keys = focus_keys;
    view.focus_valid = true;
    view.message = format!("Callers above · Callees below · depth {}", request.depth);
    let summary = graph.filter_summary(&request.filter);
    (view.hidden_nodes, view.hidden_edges) = (summary.hidden_nodes, summary.hidden_edges);
    view
}

fn page(
    aggregates: Aggregates,
    ordered: Vec<(usize, NodeOrder)>,
    page: usize,
    roots: &[usize],
) -> GraphView {
    let total_nodes = ordered.len();
    let (selected, pages) = select_page(&ordered, page, roots);
    let positions: HashMap<_, _> = selected
        .iter()
        .enumerate()
        .map(|(position, (original, _))| (*original, position))
        .collect();
    let nodes = selected
        .iter()
        .map(|(index, _)| aggregates.nodes[*index].clone())
        .collect();
    let ordering = selected
        .into_iter()
        .map(|(_, mut order)| {
            order.parent = order.parent.and_then(|p| positions.get(&p).copied());
            order
        })
        .collect();
    let total_edges = aggregates.edges.len();
    let edges: Vec<_> = aggregates
        .edges
        .into_iter()
        .filter_map(|mut edge| {
            edge.from = *positions.get(&edge.from)?;
            edge.to = *positions.get(&edge.to)?;
            Some(edge)
        })
        .take(EDGE_LIMIT)
        .collect();
    GraphView {
        nodes,
        ordering,
        omitted_nodes: total_nodes.saturating_sub(positions.len()),
        omitted_edges: total_edges.saturating_sub(edges.len()),
        edges,
        collapsed_edges: aggregates.collapsed_edges,
        pages,
        ..Default::default()
    }
}

fn select_page(
    ordered: &[(usize, NodeOrder)],
    page: usize,
    roots: &[usize],
) -> (Vec<(usize, NodeOrder)>, usize) {
    let pinned: HashSet<_> = roots.iter().copied().take(NODE_LIMIT - 1).collect();
    let capacity = NODE_LIMIT - pinned.len();
    let pages = ordered
        .len()
        .saturating_sub(pinned.len())
        .div_ceil(capacity)
        .max(1);
    let mut selected: Vec<_> = ordered
        .iter()
        .filter(|(node, _)| pinned.contains(node))
        .cloned()
        .collect();
    let roots: HashSet<_> = roots.iter().copied().collect();
    let mut remaining: Vec<_> = ordered
        .iter()
        .filter(|(node, _)| !pinned.contains(node))
        .collect();
    // All selected class methods take precedence over external context, including
    // the final method when a class exactly fills the canvas node budget.
    remaining.sort_by_key(|(node, _)| !roots.contains(node));
    selected.extend(
        remaining
            .into_iter()
            .skip(page.min(pages - 1) * capacity)
            .take(capacity)
            .cloned(),
    );
    let sequence: HashMap<_, _> = ordered
        .iter()
        .enumerate()
        .map(|(i, (node, _))| (*node, i))
        .collect();
    selected.sort_by_key(|(node, _)| sequence[node]);
    (selected, pages)
}

#[cfg(test)]
mod file_tests {
    use super::*;
    use codegraph::{graph::Filter, model::GraphData};
    use std::sync::Arc;

    #[test]
    fn files_are_distinct_filtered_and_qualified_before_paging() {
        let mut nodes = Vec::new();
        for (i, (project, file, test)) in [
            ("a", "/root/src/A.cs", false),
            ("a", "/root/src/A.cs", false),
            ("a", "/root/src/sub/A.cs", false),
            ("a", "/root/src/Test.cs", true),
            ("b", "/other/src/A.cs", false),
        ]
        .into_iter()
        .enumerate()
        {
            nodes.push(Node {
                id: i.to_string(),
                build: "/root/app.csproj".into(),
                project: project.into(),
                file: file.into(),
                namespace: "Space".into(),
                container: "Type".into(),
                is_test: test,
                ..Default::default()
            });
        }
        let graph = GraphIndex::new(Arc::new(GraphData {
            nodes,
            ..Default::default()
        }))
        .unwrap();
        let mut request = Request {
            focus_target: None,
            grouping: Grouping::Folder,
            expanded: HashSet::new(),
            scope: Scope::Overview,
            direction: super::super::FlowDirection::Callees,
            depth: 2,
            filter: Filter::default(),
            page: 0,
            architecture: None,
            layer_tab: None,
        };
        request.filter.include_tests = false;
        let view = super::super::project(&graph, &request);
        let a = view.nodes.iter().find(|n| n.label == "a").unwrap();
        assert_eq!(a.group_kind, Some(super::super::GroupKind::Project));
        assert_eq!(
            a.files.iter().map(|f| f.label.as_str()).collect::<Vec<_>>(),
            ["src/A.cs", "src/sub/A.cs"]
        );
        request.expanded.insert(a.key.clone());
        let view = super::super::project(&graph, &request);
        let folder = view.nodes.iter().find(|n| n.label == "src").unwrap();
        assert_eq!(folder.files.len(), 2);
        request.grouping = Grouping::Namespace;
        let view = super::super::project(&graph, &request);
        let space = view.nodes.iter().find(|n| n.label == "Space").unwrap();
        assert_eq!(space.files.len(), 2);
    }
    #[test]
    fn file_membership_is_complete_and_separates_identical_projects_in_other_builds() {
        let nodes = (0..(NODE_LIMIT + 2))
            .map(|index| Node {
                id: index.to_string(),
                build: if index == NODE_LIMIT + 1 {
                    "/other".into()
                } else {
                    "/root".into()
                },
                project: "same".into(),
                namespace: "shared".into(),
                container: index.to_string(),
                file: format!("/root/src/{index}.java"),
                ..Default::default()
            })
            .collect();
        let graph = GraphIndex::new(Arc::new(GraphData {
            nodes,
            ..Default::default()
        }))
        .unwrap();
        let request = Request {
            focus_target: None,
            grouping: Grouping::Namespace,
            expanded: HashSet::new(),
            scope: Scope::Overview,
            direction: super::super::FlowDirection::Callees,
            depth: 2,
            filter: Filter::default(),
            page: 0,
            architecture: None,
            layer_tab: None,
        };
        let view = super::super::project(&graph, &request);
        assert_eq!(view.nodes.len(), 2);
        let mut counts: Vec<_> = view.nodes.iter().map(|node| node.files.len()).collect();
        counts.sort();
        assert_eq!(counts, [1, NODE_LIMIT + 1]);
        let mut expanded = request;
        expanded
            .expanded
            .extend(view.nodes.iter().map(|n| n.key.clone()));
        let namespaces = super::super::project(&graph, &expanded);
        expanded
            .expanded
            .extend(namespaces.nodes.iter().map(|n| n.key.clone()));
        let classes = super::super::project(&graph, &expanded);
        assert_eq!(classes.nodes.len(), NODE_LIMIT);
        assert_eq!(classes.pages, 2);
        assert!(classes.nodes.iter().all(|node| node.files.len() == 1));
    }
}
