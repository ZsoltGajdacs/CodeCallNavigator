use super::*;
use codegraph::model::{Edge, GraphData, Node};

fn graph() -> GraphIndex {
    GraphIndex::new(Arc::new(GraphData {
        nodes: (0..6)
            .map(|i| Node {
                id: format!("m{i}"),
                label: format!("method{i}()"),
                container: format!("Class{}", i / 2),
                project: "app".into(),
                build: "/build".into(),
                ..Default::default()
            })
            .collect(),
        edges: [(0, 1), (1, 2), (2, 3), (3, 4)]
            .into_iter()
            .map(|(from, to)| Edge {
                evidence: Vec::new(),
                from,
                to,
                kind: "call".into(),
                certainty: "direct".into(),
                count: 1,
                provenance: String::new(),
            })
            .collect(),
        ..Default::default()
    }))
    .unwrap()
}
fn request(target: FocusTarget) -> Request {
    Request {
        grouping: Grouping::Method,
        expanded: HashSet::new(),
        scope: Scope::Overview,
        focus_target: Some(target),
        direction: FlowDirection::Callees,
        depth: 1,
        filter: Filter::default(),
        page: 0,
        architecture: None,
        layer_tab: None,
    }
}
#[test]
fn focus_keeps_callers_above_and_callees_below_without_unrelated_nodes() {
    let view = project(&graph(), &request(FocusTarget::Method("m2".into())));
    let ranked: Vec<_> = view
        .nodes
        .iter()
        .zip(&view.ordering)
        .map(|(n, o)| (n.key.as_str(), o.rank))
        .collect();
    assert_eq!(ranked, [("m1", -1), ("m2", 0), ("m3", 1)]);
    assert_eq!(view.focus_keys, ["m2"]);
    assert!(view.focus_valid);
}
#[test]
fn grouped_depth_ignores_internal_calls_and_class_rows_are_complete() {
    let graph = graph();
    let mut req = request(FocusTarget::Group(
        "[\"class\",\"/build\",\"app\",\"Class1\"]".into(),
    ));
    req.grouping = Grouping::Project;
    req.expanded
        .insert("[\"project\",\"/build\",\"app\"]".into());
    let view = project(&graph, &req);
    assert_eq!(view.nodes.len(), 3);
    assert!(view.nodes.iter().all(|n| n.methods.len() == 2));
    assert_eq!(
        view.ordering.iter().map(|o| o.rank).collect::<Vec<_>>(),
        [-1, 0, 1]
    );
}

#[test]
fn class_focus_expands_collapsed_neighbor_folders_before_traversing() {
    let mut data = graph().data.as_ref().clone();
    for (index, node) in data.nodes.iter_mut().enumerate() {
        node.file = format!(
            "/build/{}/{}.java",
            if index < 2 { "callers" } else { "services" },
            node.container
        );
    }
    // Class2 shares a folder with the selected class, but its unrelated method
    // must not turn the folder into an extra root or a connection hub.
    data.nodes.push(Node {
        id: "unrelated".into(),
        build: "/build".into(),
        project: "app".into(),
        container: "Unrelated".into(),
        file: "/build/services/Unrelated.java".into(),
        ..Default::default()
    });
    data.edges.push(Edge {
        evidence: Vec::new(),
        from: 5,
        to: 6,
        kind: "call".into(),
        certainty: "direct".into(),
        count: 1,
        provenance: String::new(),
    });
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let mut req = request(FocusTarget::Group(
        "[\"class\",\"/build\",\"app\",\"Class1\"]".into(),
    ));
    req.grouping = Grouping::Folder;
    req.expanded.extend([
        "[\"project\",\"/build\",\"app\"]".into(),
        "[\"folder\",\"/build\",\"app\",\"services\"]".into(),
    ]);
    let view = project(&graph, &req);
    assert!(
        view.nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Class))
    );
    assert_eq!(
        view.nodes
            .iter()
            .map(|node| node.label.as_str())
            .collect::<Vec<_>>(),
        ["Class0", "Class1", "Class2"]
    );
    assert_eq!(view.edges.len(), 2);
    assert_eq!(view.focus_keys.len(), 1);
}
#[test]
fn class_methods_focus_retains_all_its_methods_and_external_context() {
    let graph = graph();
    let req = request(FocusTarget::ClassMethods(
        "[\"class\",\"/build\",\"app\",\"Class1\"]".into(),
    ));
    let view = project(&graph, &req);
    let keys: Vec<_> = view.nodes.iter().map(|n| n.key.as_str()).collect();
    assert_eq!(keys, ["m1", "m2", "m3", "m4"]);
    assert_eq!(view.focus_keys, ["m2", "m3"]);
    let ranks: Vec<_> = view.ordering.iter().map(|order| order.rank).collect();
    assert_eq!(ranks, [-1, 0, 1, 2]);
    let class_callee = view.nodes.iter().position(|node| node.key == "m3").unwrap();
    let class_caller = view.nodes.iter().position(|node| node.key == "m2").unwrap();
    assert_eq!(view.ordering[class_callee].parent, Some(class_caller));
}

#[test]
fn depth_extends_both_sides_and_is_independent_of_overview_direction() {
    let graph = graph();
    let mut req = request(FocusTarget::Method("m2".into()));
    req.depth = 2;
    let first = project(&graph, &req);
    req.direction = FlowDirection::Callers;
    let second = project(&graph, &req);
    let describe = |view: &GraphView| {
        view.nodes
            .iter()
            .zip(&view.ordering)
            .map(|(n, o)| (n.key.clone(), o.rank))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        describe(&first),
        [
            ("m0".into(), -2),
            ("m1".into(), -1),
            ("m2".into(), 0),
            ("m3".into(), 1),
            ("m4".into(), 2)
        ]
    );
    assert_eq!(describe(&first), describe(&second));
    assert_eq!(first.omitted_nodes, 0);
}

#[test]
fn recursive_shared_nodes_are_unique_and_ties_go_above() {
    let mut data = graph().data.as_ref().clone();
    data.edges
        .extend([(2, 1), (3, 2), (2, 2)].into_iter().map(|(from, to)| Edge {
            evidence: Vec::new(),
            from,
            to,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
            provenance: String::new(),
        }));
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let view = project(&graph, &request(FocusTarget::Method("m2".into())));
    assert_eq!(view.nodes.len(), 3);
    assert_eq!(
        view.nodes
            .iter()
            .zip(&view.ordering)
            .find(|(n, _)| n.key == "m3")
            .unwrap()
            .1
            .rank,
        -1
    );
    assert!(view.edges.iter().any(|e| e.from == e.to));
    assert_eq!(view.edges.len(), 5);
}

#[test]
fn expanded_group_focus_keeps_its_level_and_invalid_focus_returns_overview() {
    let graph = graph();
    let mut req = request(FocusTarget::Group(
        "[\"project\",\"/build\",\"app\"]".into(),
    ));
    req.grouping = Grouping::Project;
    req.expanded
        .insert("[\"project\",\"/build\",\"app\"]".into());
    let view = project(&graph, &req);
    assert_eq!(view.focus_keys.len(), 1);
    assert!(
        view.nodes
            .iter()
            .all(|n| n.group_kind == Some(GroupKind::Project))
    );
    req.focus_target = Some(FocusTarget::Method("missing".into()));
    let fallback = project(&graph, &req);
    assert!(!fallback.focus_valid);
    assert_eq!(fallback.nodes.len(), 3);
}

#[path = "level_tests.rs"]
mod level_tests;

#[test]
fn focused_pagination_pins_the_anchor_and_counts_only_neighborhood_omissions() {
    let mut data = graph().data.as_ref().clone();
    data.nodes = (0..NODE_LIMIT + 5)
        .map(|i| Node {
            id: format!("m{i:05}"),
            label: format!("m{i}()"),
            ..Default::default()
        })
        .collect();
    data.edges = (1..NODE_LIMIT + 4)
        .map(|to| Edge {
            evidence: Vec::new(),
            from: 0,
            to: to as u32,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
            provenance: String::new(),
        })
        .collect();
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let mut req = request(FocusTarget::Method("m00000".into()));
    let first = project(&graph, &req);
    assert_eq!(first.pages, 2);
    assert_eq!(first.nodes.len(), NODE_LIMIT);
    assert_eq!(first.omitted_nodes, 4);
    req.page = 1;
    let last = project(&graph, &req);
    assert_eq!(last.nodes.len(), 5);
    assert!(last.nodes.iter().any(|n| n.key == "m00000"));
    assert!(
        !last
            .nodes
            .iter()
            .any(|n| n.key == format!("m{:05}", NODE_LIMIT + 4))
    );
}

#[test]
fn real_graphviz_uses_signed_order_and_class_card_dimensions() {
    let graph = graph();
    let req = request(FocusTarget::Method("m2".into()));
    let view = layout(
        project(&graph, &req),
        std::path::Path::new("/opt/homebrew/bin/dot"),
    )
    .unwrap();
    assert!(view.nodes[0].position[1] < view.nodes[1].position[1]);
    assert!(view.nodes[1].position[1] < view.nodes[2].position[1]);
    assert!(view.edges.iter().all(|e| !e.points.is_empty()));
    let mut classes = req;
    classes.grouping = Grouping::Project;
    classes.focus_target = None;
    classes
        .expanded
        .insert("[\"project\",\"/build\",\"app\"]".into());
    let view = layout(
        project(&graph, &classes),
        std::path::Path::new("/opt/homebrew/bin/dot"),
    )
    .unwrap();
    assert!(view.nodes.iter().all(|n| n.size[1] == 117.2));
    for pair in view.nodes.windows(2) {
        assert!(pair[0].position[1] + pair[0].size[1] < pair[1].position[1]);
    }
}

#[test]
fn layer_class_method_focus_resolves_qualified_keys_and_excludes_other_tabs() {
    use codegraph::architecture::{Assignment, LayerAssignments, LayerDefinition, file_key};
    let mut data = graph().data.as_ref().clone();
    for node in &mut data.nodes {
        node.file = format!("/{}.java", node.id);
    }
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let mut req = request(FocusTarget::Method("m2".into()));
    req.grouping = Grouping::Layer;
    req.layer_tab = Some("main".into());
    req.architecture = Some(Arc::new(LayerAssignments {
        files: graph
            .data
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (
                    file_key(&n.build, &n.project, &n.file),
                    Assignment {
                        layer: if i < 4 { "Main" } else { "Hidden" }.into(),
                        rule: String::new(),
                    },
                )
            })
            .collect(),
        layers: vec![
            LayerDefinition {
                name: "Main".into(),
                tab: Some("main".into()),
                ..Default::default()
            },
            LayerDefinition {
                name: "Hidden".into(),
                tab: Some("other".into()),
                ..Default::default()
            },
        ],
        origin: String::new(),
    }));
    let view = project(&graph, &req);
    assert!(view.focus_valid);
    assert_eq!(
        view.nodes
            .iter()
            .map(|n| n.key.as_str())
            .collect::<Vec<_>>(),
        ["m1", "m2", "m3"]
    );
    assert!(view.layer_bands.is_empty());
    let class_key = "[\"class\",\"/build\",\"app\",\"Class1\"]";
    let qualified = serde_json::to_string(&["layer-member", "Main", class_key]).unwrap();
    req.focus_target = Some(FocusTarget::ClassMethods(qualified));
    let classes = project(&graph, &req);
    assert!(classes.focus_valid);
    assert_eq!(classes.focus_keys, ["m2", "m3"]);
    assert!(!classes.nodes.iter().any(|n| n.key == "m4"));
}

#[test]
fn class_methods_fill_the_page_before_external_context_at_the_canvas_limit() {
    for count in [NODE_LIMIT, NODE_LIMIT + 1] {
        let mut nodes: Vec<_> = (0..count)
            .map(|i| Node {
                id: format!("m{i:05}"),
                label: format!("m{i}()"),
                container: "Class".into(),
                build: "/build".into(),
                project: "app".into(),
                ..Default::default()
            })
            .collect();
        nodes.push(Node {
            id: "caller".into(),
            container: "External".into(),
            ..Default::default()
        });
        let graph = GraphIndex::new(Arc::new(GraphData {
            nodes,
            edges: vec![Edge {
                evidence: Vec::new(),
                from: count as u32,
                to: 0,
                kind: "call".into(),
                certainty: "direct".into(),
                provenance: String::new(),
                count: 1,
            }],
            ..Default::default()
        }))
        .unwrap();
        let mut req = request(FocusTarget::ClassMethods(
            "[\"class\",\"/build\",\"app\",\"Class\"]".into(),
        ));
        let first = project(&graph, &req);
        assert_eq!(first.nodes.len(), NODE_LIMIT);
        assert!(first.nodes.iter().all(|node| node.key.starts_with('m')));
        assert_eq!(first.omitted_nodes, count + 1 - NODE_LIMIT);
        let mut visited: HashSet<_> = first.nodes.iter().map(|node| node.key.clone()).collect();
        for page in 1..first.pages {
            req.page = page;
            let view = project(&graph, &req);
            assert!(view.nodes.len() <= NODE_LIMIT);
            visited.extend(view.nodes.iter().map(|node| node.key.clone()));
        }
        assert_eq!(visited.len(), count + 1);
        assert!(visited.contains("caller"));
    }
}
