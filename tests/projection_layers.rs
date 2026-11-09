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
        focus_target: None,
        grouping: Grouping::Layer,
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

fn architecture_graph() -> (GraphIndex, Arc<codegraph::architecture::LayerAssignments>) {
    use codegraph::architecture::{Assignment, LayerAssignments, LayerDefinition, file_key};
    let names = ["Controller", "Service", "Entity"];
    let mut data = GraphData {
        nodes: names
            .iter()
            .map(|name| Node {
                id: (*name).into(),
                label: (*name).into(),
                project: "Project".into(),
                build: "/example/app.sln".into(),
                ..Default::default()
            })
            .collect(),
        edges: [(0, 1), (1, 2)]
            .into_iter()
            .map(|(from, to)| Edge {
                from,
                to,
                kind: "call".into(),
                certainty: "direct".into(),
                provenance: "compiler".into(),
                count: 1,
            })
            .collect(),
        ..Default::default()
    };
    for (node, name) in data.nodes.iter_mut().zip(names) {
        node.file = format!("/example/{name}.cs");
        node.container = name.into();
    }
    data.edges.push(Edge {
        from: 1,
        to: 0,
        kind: "call".into(),
        certainty: "direct".into(),
        provenance: "compiler".into(),
        count: 1,
    });
    let files = data
        .nodes
        .iter()
        .zip(names)
        .map(|(node, layer)| {
            (
                file_key(&node.build, &node.project, &node.file),
                Assignment {
                    layer: layer.into(),
                    rule: "fixture".into(),
                },
            )
        })
        .collect();
    let layers = names
        .into_iter()
        .map(|name| LayerDefinition {
            name: name.into(),
            support: name == "Entity",
            ..Default::default()
        })
        .collect();
    let assignments = Arc::new(LayerAssignments {
        files,
        layers,
        origin: "fixture".into(),
    });
    (GraphIndex::new(Arc::new(data)).unwrap(), assignments)
}

#[test]
fn layer_expansion_and_graphviz_bands_preserve_reverse_relationships_and_cycles() {
    let (graph, assignments) = architecture_graph();
    let mut query = request();
    query.grouping = Grouping::Layer;
    query.architecture = Some(assignments);
    let collapsed = project(&graph, &query);
    assert_eq!(collapsed.nodes.len(), 3);
    assert!(
        collapsed
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(projection::GroupKind::Layer))
    );
    assert_eq!(collapsed.edges.len(), 3);
    for kind in [
        projection::GroupKind::Layer,
        projection::GroupKind::Project,
        projection::GroupKind::Class,
    ] {
        let view = project(&graph, &query);
        assert!(view.nodes.iter().all(|node| node.group_kind == Some(kind)));
        query
            .expanded
            .extend(view.nodes.iter().map(|node| node.key.clone()));
    }
    let expanded = project(&graph, &query);
    assert!(expanded.nodes.iter().all(|node| node.member.is_some()));
    assert_eq!(expanded.edges.len(), 3);
    assert_eq!(expanded.nodes.iter().filter(|node| node.cyclic).count(), 2);
    let layout = projection::layout(expanded, Path::new("/opt/homebrew/bin/dot")).unwrap();
    let node = |layer: &str| {
        layout
            .nodes
            .iter()
            .find(|node| node.layer.as_deref() == Some(layer))
            .unwrap()
    };
    assert!(node("Controller").position[1] < node("Service").position[1]);
    assert!(node("Entity").position[0] > node("Controller").position[0]);
    assert!(layout.edges.iter().all(|edge| edge.points.len() >= 4));
    assert_eq!(layout.layer_bands.len(), 3);
    assert!(layout.layer_bands.iter().all(|band| band.size[0] > 0.0));
}

#[test]
fn layer_tabs_restrict_focused_methods_and_report_cross_tab_relationships() {
    let (graph, assignments) = architecture_graph();
    let mut assignments = (*assignments).clone();
    assignments.layers[2].tab = Some("Database".into());
    let mut query = request();
    query.grouping = Grouping::Layer;
    query.architecture = Some(Arc::new(assignments));
    let main = project(&graph, &query);
    assert_eq!(
        (
            main.nodes.len(),
            main.edges.len(),
            main.omitted_nodes,
            main.omitted_edges
        ),
        (2, 2, 1, 1)
    );
    query.layer_tab = Some("Database".into());
    let database = project(&graph, &query);
    assert_eq!(
        (
            database.nodes.len(),
            database.edges.len(),
            database.omitted_edges
        ),
        (1, 0, 3)
    );
    assert_eq!(database.nodes[0].label, "Entity");
    query.scope = Scope::Focus(2);
    let focused = project(&graph, &query);
    assert_eq!((focused.nodes.len(), focused.edges.len()), (1, 0));
    assert!(focused.focus_valid);
    assert!(focused.layer_bands.is_empty());
    query.scope = Scope::Focus(1);
    let removed = project(&graph, &query);
    assert!(!removed.focus_valid);
    assert_eq!(removed.nodes[0].label, "Entity");
}

#[test]
fn missing_layer_facts_explain_setup_without_affecting_other_groupings() {
    let (graph, _) = architecture_graph();
    let mut query = request();
    query.grouping = Grouping::Layer;
    let missing = project(&graph, &query);
    assert!(missing.nodes.is_empty());
    assert!(missing.message.contains("Refresh"));
    query.grouping = Grouping::Project;
    assert_eq!(project(&graph, &query).nodes.len(), 1);
}

#[test]
fn layer_filter_counts_do_not_report_hidden_node_relationships_as_omitted() {
    let (graph, assignments) = architecture_graph();
    let mut data = (*graph.data).clone();
    data.nodes[2].is_test = true;
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let mut query = request();
    query.grouping = Grouping::Layer;
    query.architecture = Some(assignments);
    let view = project(&graph, &query);
    assert_eq!((view.nodes.len(), view.edges.len()), (2, 2));
    assert_eq!((view.hidden_nodes, view.hidden_edges), (1, 1));
    assert_eq!((view.omitted_nodes, view.omitted_edges), (0, 0));
}

#[test]
fn parallel_architecture_flows_align_their_starts_and_keep_independent_band_order() {
    let names = [
        "Controller",
        "Service",
        "Listener",
        "Handler",
        "FinalEvent",
        "Utilities",
        "Tests",
    ];
    let view = projection::GraphView {
        nodes: names
            .iter()
            .map(|name| projection::ViewNode {
                key: (*name).into(),
                label: (*name).into(),
                layer: Some((*name).into()),
                ..Default::default()
            })
            .collect(),
        layer_bands: names
            .iter()
            .map(|name| projection::LayerBand {
                name: (*name).into(),
                flow: matches!(*name, "Listener" | "Handler" | "FinalEvent")
                    .then(|| "Events".into()),
                support: *name == "Utilities",
                ..Default::default()
            })
            .collect(),
        edges: vec![
            projection::ViewEdge {
                from: 1,
                to: 0,
                ..Default::default()
            },
            projection::ViewEdge {
                from: 3,
                to: 1,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let view = projection::layout(view, Path::new("/opt/homebrew/bin/dot")).unwrap();
    let node = |name: &str| view.nodes.iter().find(|node| node.label == name).unwrap();
    assert!((node("Controller").position[1] - node("Listener").position[1]).abs() < 1.0);
    assert!(node("Controller").position[1] < node("Service").position[1]);
    assert!(node("Listener").position[1] < node("Handler").position[1]);
    assert!(node("Listener").position[0] > node("Controller").position[0]);
    assert!(node("Utilities").position[0] > node("Listener").position[0]);
    assert!(node("Tests").position[1] > node("FinalEvent").position[1]);
    assert!(view.edges.iter().all(|edge| edge.points.len() >= 4));
}

#[test]
fn layer_source_paths_use_dotted_java_build_directories_as_roots() {
    use codegraph::architecture::{Assignment, LayerAssignments, LayerDefinition, file_key};
    let node = Node {
        id: "method".into(),
        build: "/root/billing.service".into(),
        project: "app".into(),
        file: "/root/billing.service/src/main/java/Service.java".into(),
        language: "java".into(),
        ..Default::default()
    };
    let key = file_key(&node.build, &node.project, &node.file);
    let graph = GraphIndex::new(Arc::new(GraphData {
        nodes: vec![node],
        ..Default::default()
    }))
    .unwrap();
    let mut query = request();
    query.grouping = Grouping::Layer;
    query.architecture = Some(Arc::new(LayerAssignments {
        files: [(
            key,
            Assignment {
                layer: "Service".into(),
                rule: "fixture".into(),
            },
        )]
        .into_iter()
        .collect(),
        layers: vec![LayerDefinition {
            name: "Service".into(),
            ..Default::default()
        }],
        origin: "fixture".into(),
    }));
    let view = project(&graph, &query);
    assert_eq!(view.nodes[0].files[0].label, "src/main/java/Service.java");
}
