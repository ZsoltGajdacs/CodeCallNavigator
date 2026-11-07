#[path = "../src/projection.rs"]
#[allow(dead_code)]
mod projection;

use codegraph::{
    graph::{Direction, Filter, GraphIndex},
    model::{Edge, GraphData, Node},
};
use projection::{Grouping, Request, project};
use std::{collections::HashSet, path::Path, sync::Arc};

fn request() -> Request {
    Request {
        grouping: Grouping::Folder,
        expanded: HashSet::new(),
        focus: None,
        direction: Direction::Both,
        depth: 2,
        path: None,
        filter: Filter::default(),
        page: 0,
    }
}

fn sample() -> GraphIndex {
    let nodes = (0..3)
        .map(|i| Node {
            id: format!("build:Project:Method{i}"),
            project: "Project".into(),
            build: "/example/app.sln".into(),
            label: format!("Method{i}"),
            signature: format!("Type.Method{i}()"),
            container: "Type".into(),
            file: "/example/core/Type.cs".into(),
            ..Default::default()
        })
        .collect();
    let edges = vec![
        Edge {
            from: 0,
            to: 1,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
        },
        Edge {
            from: 1,
            to: 2,
            kind: "call".into(),
            certainty: "dispatch".into(),
            count: 1,
        },
    ];
    GraphIndex::new(Arc::new(GraphData {
        nodes,
        edges,
        ..Default::default()
    }))
    .unwrap()
}

#[test]
fn hierarchy_expansion_partitions_members_without_losing_global_namespace_methods() {
    let graph = sample();
    let mut query = request();
    let first = project(&graph, &query);
    assert_eq!(first.nodes.len(), 1);
    assert_eq!(first.nodes[0].count, 3);
    assert_eq!(first.nodes[0].internal_edges, 2);
    assert_eq!(first.collapsed_edges, 2);
    assert!(first.edges.is_empty());
    query.expanded.insert(first.nodes[0].key.clone());
    let folder = project(&graph, &query);
    assert_eq!(folder.nodes[0].label, "core");
    query.expanded.insert(folder.nodes[0].key.clone());
    let class = project(&graph, &query);
    assert_eq!(class.nodes[0].label, "Type");
    query.expanded.insert(class.nodes[0].key.clone());
    let methods = project(&graph, &query);
    assert_eq!(methods.nodes.len(), 3);
    assert_eq!(methods.edges.len(), 2);
    assert_eq!(methods.collapsed_edges, 0);
    assert!(methods.nodes.iter().all(|n| n.member.is_some()));
}

#[test]
fn path_view_respects_uncertainty_and_reports_absence_instead_of_inventing_a_bridge() {
    let graph = sample();
    let mut query = request();
    query.path = Some((0, 2));
    assert_eq!(project(&graph, &query).nodes.len(), 3);
    query.filter.include_uncertain = false;
    let hidden = project(&graph, &query);
    assert!(hidden.nodes.is_empty());
    assert!(hidden.message.starts_with("No directed path"));
    assert_eq!(hidden.hidden_edges, 1);
}

#[test]
fn real_graphviz_layout_keeps_explicit_relationship_identity() {
    let graph = sample();
    let mut query = request();
    query.path = Some((0, 2));
    let view =
        projection::layout(project(&graph, &query), Path::new("/opt/homebrew/bin/dot")).unwrap();
    assert_eq!(view.nodes.len(), 3);
    assert!(view.size[0] > 300.0);
    for edge in view.edges {
        assert!(edge.points.len() >= 4);
        assert!(edge.points[0][0] > view.nodes[edge.from].position[0]);
        assert!(edge.points.last().unwrap()[0] < view.nodes[edge.to].position[0] + 40.0);
    }
}

#[test]
fn page_reports_cross_page_relationships_in_omitted_counts() {
    let nodes = (0..1501)
        .map(|i| Node {
            id: format!("node{i}"),
            project: format!("Project{i:04}"),
            build: "fixture".into(),
            ..Default::default()
        })
        .collect();
    let graph = GraphIndex::new(Arc::new(GraphData {
        nodes,
        edges: vec![Edge {
            from: 0,
            to: 1500,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
        }],
        ..Default::default()
    }))
    .unwrap();
    let mut query = request();
    let first = project(&graph, &query);
    assert_eq!(first.nodes.len(), 1500);
    assert_eq!(first.omitted_nodes, 1);
    assert_eq!(first.omitted_edges, 1);
    assert_eq!(first.pages, 2);
    query.page = 1;
    let last = project(&graph, &query);
    assert_eq!(last.nodes.len(), 1);
    assert_eq!(last.nodes[0].label, "Project1500");
    assert_eq!(last.omitted_edges, 1);
}

#[test]
fn method_recursion_stays_visible_when_group_internals_are_summarized() {
    let mut data = (*sample().data).clone();
    data.edges.push(Edge {
        from: 0,
        to: 0,
        kind: "call".into(),
        certainty: "direct".into(),
        count: 1,
    });
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let mut query = request();
    query.focus = Some(0);
    let view = project(&graph, &query);
    let selected = view
        .nodes
        .iter()
        .position(|node| node.member == Some(0))
        .unwrap();
    assert!(view.nodes[selected].cyclic);
    assert!(
        view.edges
            .iter()
            .any(|edge| edge.from == selected && edge.to == selected)
    );
    assert_eq!(view.collapsed_edges, 0);
}
