//! Aggregate analyzed relationships, order them, then select a bounded page.
use super::ordering;
use super::{
    EDGE_LIMIT, GraphView, GroupKind, Grouping, NODE_LIMIT, NodeOrder, Request, Scope, Selection,
    SourcePath, ViewEdge, ViewNode,
};
use codegraph::{
    graph::{Filter, GraphIndex},
    model::Node,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
};

fn key(parts: &[&str]) -> String {
    serde_json::to_string(parts).expect("string arrays serialize")
}

fn relative_file(node: &Node) -> PathBuf {
    let build = Path::new(&node.build);
    let root = if build.extension().is_some() {
        build.parent().unwrap_or(build)
    } else {
        build
    };
    let file = Path::new(&node.file);
    file.strip_prefix(root).unwrap_or(file).to_path_buf()
}

fn groups(node: &Node, grouping: Grouping) -> Vec<(String, String, GroupKind)> {
    let scope = [&*node.build, &*node.project];
    let mut result = vec![(
        key(&["project", scope[0], scope[1]]),
        node.project.clone(),
        GroupKind::Project,
    )];
    match grouping {
        Grouping::Namespace => result.push((
            key(&["namespace", scope[0], scope[1], &node.namespace]),
            if node.namespace.is_empty() {
                "(global namespace)".into()
            } else {
                node.namespace.clone()
            },
            GroupKind::Namespace,
        )),
        Grouping::Folder => {
            let file = relative_file(node);
            let mut folder = PathBuf::new();
            for part in file.parent().into_iter().flat_map(Path::components) {
                folder.push(part);
                let name = folder.to_string_lossy();
                result.push((
                    key(&["folder", scope[0], scope[1], &name]),
                    name.into_owned(),
                    GroupKind::Folder,
                ));
            }
        }
        Grouping::Project | Grouping::Method => {}
    }
    result.push((
        key(&["class", scope[0], scope[1], &node.container]),
        node.container.clone(),
        GroupKind::Class,
    ));
    result
}

fn visible_node(node: &Node, index: u32, request: &Request) -> ViewNode {
    if request.scope == Scope::Overview && request.grouping != Grouping::Method {
        for (key, label, kind) in groups(node, request.grouping) {
            if !request.expanded.contains(&key) {
                return ViewNode {
                    key,
                    label,
                    group_kind: Some(kind),
                    detail: node.project.clone(),
                    ..Default::default()
                };
            }
        }
    }
    ViewNode {
        key: node.id.clone(),
        label: node.label.clone(),
        detail: node.container.clone(),
        member: Some(index),
        ..Default::default()
    }
}

struct Aggregates {
    nodes: Vec<ViewNode>,
    edges: Vec<ViewEdge>,
    members: HashMap<u32, usize>,
    collapsed_edges: usize,
}

fn aggregate_nodes(graph: &GraphIndex, request: &Request, selection: &Selection) -> Aggregates {
    let cyclic: HashSet<u32> = graph
        .cycles(&request.filter)
        .into_iter()
        .flatten()
        .collect();
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
        let mut node = visible_node(&graph.data.nodes[index as usize], index, request);
        node.application_start = node.member.is_some() && starts.contains(&node.key);
        members.insert(index, node.key.clone());
        let representative = representatives.entry(node.key.clone()).or_insert(node);
        if representative.group_kind.is_some() && !graph.data.nodes[index as usize].file.is_empty()
        {
            let source = &graph.data.nodes[index as usize];
            representative.files.push(SourcePath {
                label: relative_file(source).to_string_lossy().into_owned(),
                path: source.file.clone(),
                language: source.language.clone(),
            });
        }
        representative.count += 1;
        representative.cyclic |= cyclic.contains(&index);
    }
    for node in representatives.values_mut() {
        node.files.sort();
        node.files.dedup_by(|a, b| a.path == b.path);
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
    let mut aggregates = aggregate_nodes(graph, request, &selection);
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
    let mut view = page(aggregates, ordered, request.page);
    (view.hidden_nodes, view.hidden_edges) = hidden_counts(graph, &request.filter);
    view.omitted_nodes += selection.omitted_nodes;
    view.omitted_edges += selection.omitted_edges;
    view.message = selection.message;
    view
}

fn page(aggregates: Aggregates, ordered: Vec<(usize, NodeOrder)>, page: usize) -> GraphView {
    let pages = ordered.len().div_ceil(NODE_LIMIT).max(1);
    let selected: Vec<_> = ordered
        .into_iter()
        .skip(page.min(pages - 1) * NODE_LIMIT)
        .take(NODE_LIMIT)
        .collect();
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
        omitted_nodes: aggregates.nodes.len().saturating_sub(positions.len()),
        omitted_edges: total_edges.saturating_sub(edges.len()),
        edges,
        collapsed_edges: aggregates.collapsed_edges,
        pages,
        ..Default::default()
    }
}

fn hidden_counts(graph: &GraphIndex, filter: &Filter) -> (usize, usize) {
    let nodes = graph
        .data
        .nodes
        .iter()
        .filter(|n| !filter.accepts_node(n))
        .count();
    let edges = graph
        .data
        .edges
        .iter()
        .filter(|e| {
            !filter.accepts_edge(e)
                || !filter.accepts_node(&graph.data.nodes[e.from as usize])
                || !filter.accepts_node(&graph.data.nodes[e.to as usize])
        })
        .count();
    (nodes, edges)
}

#[cfg(test)]
mod file_tests {
    use super::*;
    use codegraph::model::GraphData;
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
            grouping: Grouping::Folder,
            expanded: HashSet::new(),
            scope: Scope::Overview,
            direction: super::super::FlowDirection::Callees,
            depth: 2,
            filter: Filter::default(),
            page: 0,
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
            grouping: Grouping::Namespace,
            expanded: HashSet::new(),
            scope: Scope::Overview,
            direction: super::super::FlowDirection::Callees,
            depth: 2,
            filter: Filter::default(),
            page: 0,
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
