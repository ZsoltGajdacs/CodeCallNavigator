//! Observable focused grouping, independent of overview expansion state.
use super::*;

fn key(parts: &[&str]) -> String {
    serde_json::to_string(parts).unwrap()
}

fn folder_graph() -> GraphIndex {
    let specifications = [
        ("app", "callers/ui/deep", "Caller"),
        ("app", "services/core", "Center"),
        ("app", "utilities/data/deep", "Sink"),
        ("other", "shared", "Shallow"),
        ("third", "", "Root"),
        ("app", "utilities/unrelated", "Unrelated"),
        ("app", "elsewhere", "Detached"),
    ];
    let nodes = specifications
        .into_iter()
        .enumerate()
        .map(|(index, (project, folder, container))| Node {
            id: format!("m{index}"),
            label: format!("method{index}"),
            build: "/build".into(),
            project: project.into(),
            container: container.into(),
            namespace: container.into(),
            file: if folder.is_empty() {
                format!("/build/{container}.java")
            } else {
                format!("/build/{folder}/{container}.java")
            },
            ..Default::default()
        })
        .collect();
    let edges = [(0, 1), (1, 2), (1, 3), (1, 4), (5, 6)]
        .into_iter()
        .map(|(from, to)| Edge {
            evidence: Vec::new(),
            from,
            to,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 2,
            provenance: String::new(),
        })
        .collect();
    GraphIndex::new(Arc::new(GraphData {
        nodes,
        edges,
        ..Default::default()
    }))
    .unwrap()
}

#[test]
fn folder_focus_matches_depth_across_projects_and_keeps_shallow_and_root_folders() {
    let graph = folder_graph();
    let mut req = request(FocusTarget::Group(key(&[
        "folder",
        "/build",
        "app",
        "services/core",
    ])));
    req.grouping = Grouping::Folder;
    // Neither collapsed projects nor deeply expanded unrelated branches determine focus granularity.
    req.expanded
        .insert(key(&["class", "/build", "other", "Shallow"]));
    let view = project(&graph, &req);
    assert!(view.focus_valid);
    assert!(
        view.nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Folder))
    );
    let identities: HashSet<_> = view.nodes.iter().map(|node| node.key.clone()).collect();
    for parts in [
        vec!["folder", "/build", "app", "callers/ui"],
        vec!["folder", "/build", "app", "services/core"],
        vec!["folder", "/build", "app", "utilities/data"],
        vec!["folder", "/build", "other", "shared"],
    ] {
        assert!(identities.contains(&key(&parts)), "missing {parts:?}");
    }
    assert_eq!(view.nodes.len(), 5);
    let root = view
        .nodes
        .iter()
        .find(|node| {
            node.files
                .iter()
                .any(|file| file.path == "/build/Root.java")
        })
        .unwrap();
    assert_eq!(root.count, 1);
    assert_eq!(view.edges.len(), 4);
    assert!(view.edges.iter().all(|edge| edge.count == 2));
    // Synthetic root identity is a usable focus target, not just a display label.
    req.focus_target = Some(FocusTarget::Group(root.key.clone()));
    let root_view = project(&graph, &req);
    assert!(root_view.focus_valid);
    assert!(
        root_view
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Folder))
    );
    assert!(root_view.nodes.iter().any(|node| node.key == root.key));
}

#[test]
fn namespace_focus_preserves_exact_packages_and_project_qualified_global_namespaces() {
    let mut data = folder_graph().data.as_ref().clone();
    for (node, namespace) in data.nodes.iter_mut().zip([
        "example.api",
        "example.service",
        "",
        "example.api",
        "",
        "unrelated",
        "detached",
    ]) {
        node.namespace = namespace.into();
    }
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let mut req = request(FocusTarget::Group(key(&[
        "namespace",
        "/build",
        "app",
        "example.service",
    ])));
    req.grouping = Grouping::Namespace;
    req.expanded.extend([
        key(&["project", "/build", "app"]),
        key(&["namespace", "/build", "app", "example.service"]),
        key(&["project", "/build", "other"]),
    ]);
    let view = project(&graph, &req);
    assert!(
        view.nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Namespace))
    );
    assert_eq!(view.nodes.len(), 5);
    assert_eq!(
        view.nodes
            .iter()
            .filter(|node| node.label == "example.api")
            .count(),
        2
    );
    assert_eq!(
        view.nodes
            .iter()
            .filter(|node| node.label == "(global namespace)")
            .count(),
        2
    );
    assert_eq!(
        view.focus_keys,
        [key(&["namespace", "/build", "app", "example.service"])]
    );
    assert_eq!(view.edges.len(), 4);
    req.filter.include_generated = false;
    let mut data = graph.data.as_ref().clone();
    data.nodes[2].is_generated = true;
    let filtered = GraphIndex::new(Arc::new(data)).unwrap();
    assert_eq!(project(&filtered, &req).nodes.len(), 4);
}

#[test]
fn explicit_group_children_choose_one_level_independent_of_expansions() {
    let graph = folder_graph();
    for (grouping, target, expected) in [
        (
            Grouping::Project,
            vec!["project", "/build", "app"],
            GroupKind::Class,
        ),
        (
            Grouping::Namespace,
            vec!["project", "/build", "app"],
            GroupKind::Namespace,
        ),
        (
            Grouping::Namespace,
            vec!["namespace", "/build", "app", "Center"],
            GroupKind::Class,
        ),
        (
            Grouping::Folder,
            vec!["project", "/build", "app"],
            GroupKind::Folder,
        ),
        (
            Grouping::Folder,
            vec!["folder", "/build", "app", "services"],
            GroupKind::Folder,
        ),
        (
            Grouping::Folder,
            vec!["folder", "/build", "app", "services/core"],
            GroupKind::Class,
        ),
    ] {
        let mut req = request(FocusTarget::GroupChildren(key(&target)));
        req.grouping = grouping;
        let before = req.expanded.clone();
        let view = project(&graph, &req);
        assert!(view.focus_valid, "{target:?}");
        assert!(
            view.nodes
                .iter()
                .all(|node| node.group_kind == Some(expected)),
            "{target:?}"
        );
        assert_eq!(req.expanded, before);
    }
}

#[test]
fn method_focus_ignores_group_expansion_and_uses_method_hops() {
    let graph = folder_graph();
    let mut req = request(FocusTarget::Method("m1".into()));
    req.grouping = Grouping::Folder;
    req.expanded.insert(key(&["project", "/build", "app"]));
    let view = project(&graph, &req);
    assert_eq!(view.nodes.len(), 5);
    assert!(view.nodes.iter().all(|node| node.member.is_some()));
    assert_eq!(view.edges.len(), 4);
}

#[test]
fn project_focus_collapses_connected_projects_even_when_they_are_expanded() {
    let graph = folder_graph();
    let mut req = request(FocusTarget::Group(key(&["project", "/build", "app"])));
    req.grouping = Grouping::Project;
    req.expanded.extend([
        key(&["project", "/build", "app"]),
        key(&["project", "/build", "other"]),
    ]);
    let view = project(&graph, &req);
    assert!(view.focus_valid);
    assert_eq!(view.nodes.len(), 3);
    assert!(
        view.nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Project))
    );
    assert_eq!(view.edges.len(), 2);
}

fn layer_request(graph: &GraphIndex) -> Request {
    use codegraph::architecture::{Assignment, LayerAssignments, LayerDefinition, file_key};
    let mut req = request(FocusTarget::Group(key(&["layer", "Services"])));
    req.grouping = Grouping::Layer;
    req.layer_tab = Some("main".into());
    req.architecture = Some(Arc::new(LayerAssignments {
        files: graph
            .data
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                (
                    file_key(&node.build, &node.project, &node.file),
                    Assignment {
                        layer: match index {
                            1 => "Services",
                            4 => "Hidden",
                            _ => "Support",
                        }
                        .into(),
                        rule: String::new(),
                    },
                )
            })
            .collect(),
        layers: ["Services", "Support", "Hidden"]
            .into_iter()
            .map(|name| LayerDefinition {
                name: name.into(),
                tab: Some(if name == "Hidden" { "other" } else { "main" }.into()),
                ..Default::default()
            })
            .collect(),
        origin: String::new(),
    }));
    req.expanded
        .extend([key(&["layer", "Services"]), key(&["layer", "Support"])]);
    req
}

#[test]
fn layer_focus_and_children_preserve_qualified_identity_and_active_tab_filtering() {
    let graph = folder_graph();
    let mut req = layer_request(&graph);
    let view = project(&graph, &req);
    assert!(view.focus_valid);
    assert_eq!(view.nodes.len(), 2);
    assert!(
        view.nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Layer))
    );
    req.focus_target = Some(FocusTarget::GroupChildren(key(&["layer", "Services"])));
    let children = project(&graph, &req);
    assert!(children.focus_valid);
    assert!(
        children
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Project))
    );
    assert_eq!(children.nodes.len(), 3);
    let project_key = key(&["project", "/build", "app"]);
    assert_eq!(
        children.focus_keys,
        [key(&["layer-member", "Services", &project_key])]
    );
    assert!(children.nodes.iter().all(|node| node.layer.is_some()));
    assert!(
        !children
            .nodes
            .iter()
            .any(|node| node.layer.as_deref() == Some("Hidden"))
    );
    req.focus_target = Some(FocusTarget::Method("m1".into()));
    let methods = project(&graph, &req);
    assert!(
        methods
            .nodes
            .iter()
            .all(|node| node.member.is_some() && node.layer.is_some())
    );
}
