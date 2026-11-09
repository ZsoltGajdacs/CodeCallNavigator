#[path = "../src/projection.rs"]
#[allow(dead_code)]
mod projection;

use codegraph::{
    graph::{Filter, GraphIndex},
    model::{Edge, GraphData, Node},
};
use projection::{FlowDirection, Grouping, Request, Scope, project};
use std::{collections::HashSet, path::Path, sync::Arc};

fn request() -> Request {
    Request {
        grouping: Grouping::Folder,
        expanded: HashSet::new(),
        scope: Scope::Overview,
        direction: FlowDirection::Callees,
        depth: 2,
        filter: Filter::default(),
        page: 0,
        architecture: None,
        layer_tab: None,
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
            provenance: String::new(),
            from: 0,
            to: 1,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
        },
        Edge {
            provenance: String::new(),
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
    assert_eq!(
        (
            first.nodes.len(),
            first.nodes[0].count,
            first.nodes[0].internal_edges,
            first.collapsed_edges,
            first.edges.len()
        ),
        (1, 3, 2, 2, 0)
    );
    query.expanded.insert(first.nodes[0].key.clone());
    let folder = project(&graph, &query);
    assert_eq!(folder.nodes[0].label, "core");
    query.expanded.insert(folder.nodes[0].key.clone());
    let class = project(&graph, &query);
    assert_eq!(class.nodes[0].label, "Type");
    query.expanded.insert(class.nodes[0].key.clone());
    let methods = project(&graph, &query);
    let member_count = methods.nodes.iter().filter(|n| n.member.is_some()).count();
    assert_eq!(
        (
            methods.nodes.len(),
            methods.edges.len(),
            methods.collapsed_edges,
            member_count
        ),
        (3, 2, 0, 3)
    );
}

#[test]
fn path_view_respects_virtual_calls_and_reports_absence_instead_of_inventing_a_bridge() {
    let graph = sample();
    let mut query = request();
    query.scope = Scope::Path(0, 2);
    assert_eq!(project(&graph, &query).nodes.len(), 3);
    query.filter.include_uncertain = false;
    assert_eq!(project(&graph, &query).nodes.len(), 3);
    query.filter.include_virtual_calls = false;
    let hidden = project(&graph, &query);
    assert!(hidden.nodes.is_empty());
    assert!(hidden.message.starts_with("No directed path"));
    assert_eq!(hidden.hidden_edges, 1);
}

#[test]
fn virtual_calls_filter_updates_grouped_counts_and_focused_neighborhoods() {
    let graph = sample();
    let mut query = request();
    query.filter.include_uncertain = false;
    assert_eq!(project(&graph, &query).collapsed_edges, 2);
    query.filter.include_virtual_calls = false;
    let hidden = project(&graph, &query);
    assert_eq!((hidden.collapsed_edges, hidden.hidden_edges), (1, 1));
    query.grouping = Grouping::Method;
    query.scope = Scope::Focus(0);
    assert_eq!(project(&graph, &query).nodes.len(), 2);
    query.filter.include_virtual_calls = true;
    let visible = project(&graph, &query);
    assert_eq!((visible.nodes.len(), visible.edges.len()), (3, 2));
    assert_eq!(visible.edges[1].certainty, "dispatch");
}

#[test]
fn real_graphviz_layout_keeps_explicit_relationship_identity() {
    let graph = sample();
    let mut query = request();
    query.scope = Scope::Path(0, 2);
    let view =
        projection::layout(project(&graph, &query), Path::new("/opt/homebrew/bin/dot")).unwrap();
    assert_eq!(view.nodes.len(), 3);
    assert!(view.size[1] > 300.0);
    for edge in view.edges {
        assert!(edge.points.len() >= 4);
        assert!(edge.points[0][1] > view.nodes[edge.from].position[1]);
        assert!(edge.points.last().unwrap()[1] < view.nodes[edge.to].position[1] + 40.0);
    }
}

#[test]
fn page_reports_cross_page_relationships_in_omitted_counts() {
    let graph = paginated_graph(0, 1500);
    let mut query = request();
    let first = project(&graph, &query);
    assert_eq!(
        (
            first.nodes.len(),
            first.omitted_nodes,
            first.omitted_edges,
            first.pages
        ),
        (1500, 1, 1, 2)
    );
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
        provenance: String::new(),
        from: 0,
        to: 0,
        kind: "call".into(),
        certainty: "direct".into(),
        count: 1,
    });
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let mut query = request();
    query.scope = Scope::Focus(0);
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

#[test]
fn method_overview_falls_back_to_uncalled_methods_in_legacy_graphs() {
    let graph = sample();
    let mut query = request();
    query.grouping = Grouping::Method;
    query.depth = 1;
    let view = project(&graph, &query);
    let methods: Vec<_> = view.nodes.iter().filter_map(|n| n.member).collect();
    let ranks: Vec<_> = view.ordering.iter().map(|n| n.rank).collect();
    assert_eq!((methods, ranks), (vec![0, 1], vec![0, 1]));
    assert!(view.message.contains("Refresh"));
    query.direction = FlowDirection::Callers;
    let view = project(&graph, &query);
    assert_eq!(view.nodes.len(), 1);
    assert_eq!(view.nodes[0].member, Some(0));
}

#[test]
fn callers_read_downward_but_real_call_arrows_point_upward() {
    let graph = sample();
    let mut query = request();
    query.scope = Scope::Focus(2);
    query.direction = FlowDirection::Callers;
    let view =
        projection::layout(project(&graph, &query), Path::new("/opt/homebrew/bin/dot")).unwrap();
    let y = |member| {
        view.nodes
            .iter()
            .find(|n| n.member == Some(member))
            .unwrap()
            .position[1]
    };
    assert!(y(2) < y(1));
    assert!(y(1) < y(0));
    for edge in &view.edges {
        assert!(edge.points[0][1] > edge.points.last().unwrap()[1]);
    }
}

#[test]
fn legacy_directions_deserialize_to_the_two_presentation_choices() {
    for (value, expected) in [
        ("Both", FlowDirection::Callees),
        ("Incoming", FlowDirection::Callers),
        ("Outgoing", FlowDirection::Callees),
    ] {
        assert_eq!(
            serde_json::from_str::<FlowDirection>(&format!("\"{value}\"")).unwrap(),
            expected
        );
    }
}

fn branching() -> GraphIndex {
    let names = [
        "Root",
        "A",
        "B",
        "A1",
        "A2",
        "B1",
        "Shared",
        "OtherRoot",
        "OtherChild",
    ];
    let nodes = names
        .iter()
        .map(|name| Node {
            id: name.to_string(),
            label: name.to_string(),
            kind: "method".into(),
            project: name.to_string(),
            build: "fixture".into(),
            ..Default::default()
        })
        .collect();
    let edges = [
        (0, 1),
        (0, 2),
        (1, 3),
        (1, 4),
        (2, 5),
        (1, 6),
        (2, 6),
        (6, 0),
        (7, 8),
    ]
    .into_iter()
    .map(|(from, to)| Edge {
        provenance: String::new(),
        from,
        to,
        kind: "call".into(),
        certainty: "direct".into(),
        count: 1,
    })
    .collect();
    let data = GraphData {
        nodes,
        edges,
        entry_points: Some(codegraph::model::EntryPoints {
            framework_entries: Vec::new(),
            candidates: vec!["Root".into(), "OtherRoot".into()],
            application_starts: vec!["Root".into()],
        }),
        ..Default::default()
    };
    GraphIndex::new(Arc::new(data)).unwrap()
}

#[test]
fn multiple_roots_shared_methods_cycles_and_start_highlighting_keep_stable_branches() {
    let graph = branching();
    let mut query = request();
    query.grouping = Grouping::Method;
    let view = project(&graph, &query);
    let starts: Vec<_> = view
        .nodes
        .iter()
        .filter(|n| n.application_start)
        .map(|n| n.key.as_str())
        .collect();
    assert_eq!(
        (view.nodes.len(), view.edges.len(), starts),
        (9, 9, vec!["Root"])
    );
    let at = |name: &str| view.nodes.iter().position(|n| n.key == name).unwrap();
    assert_eq!(
        (
            view.ordering[at("Root")].rank,
            view.ordering[at("OtherRoot")].rank,
            view.ordering[at("Shared")].parent
        ),
        (0, 0, Some(at("A")))
    );
    let branch = [at("A1"), at("A2"), at("Shared"), at("B1")];
    assert!(branch.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(view.nodes[at("Root")].cyclic);
    let layout = projection::layout(view, Path::new("/opt/homebrew/bin/dot")).unwrap();
    let x = |name: &str| {
        layout
            .nodes
            .iter()
            .find(|n| n.key == name)
            .unwrap()
            .position[0]
    };
    let columns = [x("A1"), x("A2"), x("Shared"), x("B1")];
    assert!(columns.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(layout.edges.iter().all(|e| e.points.len() >= 4));
}

#[test]
fn method_entry_filters_depth_and_manual_focus_are_independent() {
    let mut data = (*branching().data).clone();
    data.nodes[7].is_test = true;
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let mut query = request();
    query.grouping = Grouping::Method;
    query.depth = 1;
    let view = project(&graph, &query);
    assert_eq!(
        view.nodes
            .iter()
            .map(|n| n.key.as_str())
            .collect::<Vec<_>>(),
        vec!["Root", "A", "B"]
    );
    query.filter.include_tests = true;
    assert_eq!(project(&graph, &query).nodes.len(), 5);
    query.scope = Scope::Focus(5);
    assert_eq!(project(&graph, &query).nodes[0].key, "B1");
}

#[test]
fn aggregate_overview_uses_directional_component_roots_even_for_cycles() {
    let graph = branching();
    let mut query = request();
    query.grouping = Grouping::Project;
    let down = project(&graph, &query);
    let at = |view: &projection::GraphView, name: &str| {
        view.nodes.iter().position(|n| n.label == name).unwrap()
    };
    assert_eq!(down.ordering[at(&down, "OtherRoot")].rank, 0);
    assert_eq!(down.ordering[at(&down, "OtherChild")].rank, 1);
    assert_eq!(down.nodes.len(), 9);
    query.direction = FlowDirection::Callers;
    let up = project(&graph, &query);
    assert_eq!(up.ordering[at(&up, "OtherChild")].rank, 0);
    assert_eq!(up.ordering[at(&up, "OtherRoot")].rank, 1);
    assert_eq!(up.nodes.len(), 9);
}

#[test]
fn cycles_without_candidates_have_an_actionable_empty_method_overview() {
    let mut data = (*sample().data).clone();
    data.edges.push(Edge {
        provenance: String::new(),
        from: 2,
        to: 0,
        kind: "call".into(),
        certainty: "direct".into(),
        count: 1,
    });
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let mut query = request();
    query.grouping = Grouping::Method;
    let view = project(&graph, &query);
    assert!(view.nodes.is_empty());
    assert!(view.message.contains("search and focus"));
    query.scope = Scope::Focus(0);
    assert_eq!(project(&graph, &query).nodes.len(), 3);
}

#[test]
fn traversal_remains_stable_when_storage_order_changes() {
    let graph = branching();
    let mut query = request();
    query.grouping = Grouping::Project;
    let before = project(&graph, &query);
    let mut data = (*graph.data).clone();
    data.nodes.reverse();
    let last = data.nodes.len() as u32 - 1;
    for edge in &mut data.edges {
        edge.from = last - edge.from;
        edge.to = last - edge.to;
    }
    data.edges.reverse();
    let shuffled = GraphIndex::new(Arc::new(data)).unwrap();
    let after = project(&shuffled, &query);
    assert_eq!(
        before.nodes.iter().map(|n| &n.key).collect::<Vec<_>>(),
        after.nodes.iter().map(|n| &n.key).collect::<Vec<_>>()
    );
    assert_eq!(
        before
            .ordering
            .iter()
            .map(|n| (n.rank, n.parent))
            .collect::<Vec<_>>(),
        after
            .ordering
            .iter()
            .map(|n| (n.rank, n.parent))
            .collect::<Vec<_>>()
    );
}

#[test]
fn aggregate_pagination_follows_traversal_instead_of_symbol_names() {
    let graph = paginated_graph(1500, 0);
    let mut query = request();
    let first = project(&graph, &query);
    assert_eq!(first.nodes.len(), 1500);
    assert!(!first.nodes.iter().any(|n| n.label == "Project0000"));
    assert!(first.nodes.iter().any(|n| n.label == "Project1500"));
    query.page = 1;
    let second = project(&graph, &query);
    assert_eq!(second.nodes[0].label, "Project0000");
    let order = &second.ordering[0];
    assert_eq!(
        (order.rank, order.parent, second.omitted_edges),
        (1, None, 1)
    );
}

fn paginated_graph(from: u32, to: u32) -> GraphIndex {
    let nodes = (0..1501)
        .map(|i| Node {
            id: format!("node{i}"),
            project: format!("Project{i:04}"),
            build: "fixture".into(),
            ..Default::default()
        })
        .collect();
    GraphIndex::new(Arc::new(GraphData {
        nodes,
        edges: vec![Edge {
            provenance: String::new(),
            from,
            to,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
        }],
        ..Default::default()
    }))
    .unwrap()
}

#[test]
fn framework_entry_is_a_method_root_without_verified_start_highlighting() {
    let mut data = (*branching().data).clone();
    let id = data.nodes[1].id.clone();
    data.entry_points = Some(codegraph::model::EntryPoints {
        candidates: vec![],
        application_starts: vec![],
        framework_entries: vec![id.clone()],
    });
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let request = Request {
        grouping: Grouping::Method,
        ..request()
    };
    let view = projection::project(&graph, &request);
    assert!(
        view.nodes
            .iter()
            .any(|node| node.key == id && !node.application_start)
    );
}
