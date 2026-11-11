//! Native graph traversal behavior over an explicit multi-class call graph.
use super::click;
use crate::{
    app::State,
    jobs,
    navigation::Navigation,
    projection::{FocusTarget, GroupKind, ViewNode},
    view,
};
use codecallnavigator::{
    graph::GraphIndex,
    model::{Edge, GraphData, Node},
};
use rune::{Env, Runtime};
use std::sync::Arc;

fn frames(runtime: &mut Runtime<State>, count: usize) {
    for _ in 0..count {
        runtime.frame(1.0 / 60.0);
    }
}

fn runtime() -> Runtime<State> {
    let mut state = State::new();
    state.scene = true;
    state.jobs.boot.cancel();
    let mut nodes: Vec<_> = (0..19)
        .map(|i| {
            let container = match i {
                0 => "Caller",
                17 => "Sink",
                18 => "Unrelated",
                _ => "Center",
            };
            Node {
                id: format!("m{i}"),
                project: "app".into(),
                build: "/build".into(),
                namespace: "sample".into(),
                container: container.into(),
                kind: "method".into(),
                label: format!("work{i}"),
                signature: format!("{container}::work({i})"),
                ..Default::default()
            }
        })
        .collect();
    // Same readable name, different signatures and stable identities.
    nodes[1].label = "work".into();
    nodes[2].label = "work".into();
    let edges = [(0, 1), (1, 2), (2, 17)]
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
        .collect();
    state.graph = Some(Arc::new(
        GraphIndex::new(Arc::new(GraphData {
            nodes,
            edges,
            ..Default::default()
        }))
        .unwrap(),
    ));
    state.changed();
    let mut runtime = Runtime::new(state, view::view, (1420.0, 900.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    frames(&mut runtime, 3);
    runtime
}

fn graph_control(runtime: &Runtime<State>, matches: impl Fn(&ViewNode) -> bool) -> String {
    let index = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .position(matches)
        .expect("Expected graph node to be visible");
    format!("graph-node-{index}")
}

fn node_control(runtime: &Runtime<State>, label: &str) -> String {
    graph_control(runtime, |node| node.label == label)
}

fn method_control(runtime: &Runtime<State>, member: u32) -> String {
    graph_control(runtime, |node| node.member == Some(member))
}

fn double_click(runtime: &mut Runtime<State>, key: &str) {
    let (x, y) = runtime
        .find(key)
        .unwrap_or_else(|| panic!("Missing control {key}"));
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.05);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.05);
    frames(runtime, 2);
}

fn centered(runtime: &Runtime<State>, member: u32) {
    let node = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|n| n.member == Some(member))
        .unwrap();
    let center = runtime.state.nav.camera.screen([
        node.position[0] + node.size[0] / 2.0,
        node.position[1] + node.size[1] / 2.0,
    ]);
    assert!((center[0] - runtime.state.viewport[0] / 2.0).abs() < 0.1);
    assert!(
        (center[1] - runtime.state.viewport[1] / 2.0).abs() < 0.1,
        "member={member} center={center:?} viewport={:?} pan={:?}",
        runtime.state.viewport,
        runtime.state.nav.camera.pan
    );
}

fn classes(runtime: &mut Runtime<State>) {
    let expanded = runtime.state.nav.expanded.clone();
    let key = node_control(runtime, "app");
    double_click(runtime, &key);
    assert_eq!(runtime.state.nav.expanded, expanded);
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|n| n.group_kind == Some(GroupKind::Class))
    );
}

fn grouped_runtime(grouping: crate::projection::Grouping) -> Runtime<State> {
    let mut runtime = runtime();
    let mut data = runtime.state.graph.as_ref().unwrap().data.as_ref().clone();
    for node in &mut data.nodes {
        node.namespace = format!("sample.{}", node.container.to_lowercase());
        node.file = format!(
            "/build/src/{}/{}.java",
            node.container.to_lowercase(),
            node.container
        );
    }
    runtime.state.graph = Some(Arc::new(GraphIndex::new(Arc::new(data)).unwrap()));
    runtime.state.set_grouping(grouping);
    frames(&mut runtime, 3);
    runtime
}

#[test]
fn package_focus_and_drill_roundtrip_preserve_overview_expansion_and_history() {
    use crate::projection::Grouping;
    let mut runtime = grouped_runtime(Grouping::Namespace);
    let expanded = runtime.state.nav.expanded.clone();
    let project = node_control(&runtime, "app");
    double_click(&mut runtime, &project);
    assert!(matches!(
        runtime.state.nav.focus_target,
        Some(FocusTarget::GroupChildren(_))
    ));
    assert_eq!(runtime.state.nav.expanded, expanded);
    // Persisted explicit drill state restores the same namespace-level graph.
    let saved = serde_json::to_value(&runtime.state.nav).unwrap();
    runtime.state.nav = serde_json::from_value(saved).unwrap();
    runtime.state.changed();
    frames(&mut runtime, 3);
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Namespace))
    );
    let center = node_control(&runtime, "sample.center");
    click(&mut runtime, &center);
    frames(&mut runtime, 40);
    assert_eq!(runtime.state.graph_view.nodes.len(), 3);
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Namespace))
    );
    let focused = serde_json::to_value(&runtime.state.nav).unwrap();
    let center = node_control(&runtime, "sample.center");
    double_click(&mut runtime, &center);
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Class))
    );
    assert_eq!(runtime.state.nav.expanded, expanded);
    click(&mut runtime, "back");
    assert_eq!(serde_json::to_value(&runtime.state.nav).unwrap(), focused);
    click(&mut runtime, "forward");
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Class))
    );
}

#[test]
fn folder_drill_aligns_every_branch_then_class_focus_collapses_expanded_neighbors() {
    use crate::projection::Grouping;
    let mut runtime = grouped_runtime(Grouping::Folder);
    let project = node_control(&runtime, "app");
    double_click(&mut runtime, &project);
    let src = node_control(&runtime, "src");
    double_click(&mut runtime, &src);
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Folder))
    );
    let folder = node_control(&runtime, "src/center");
    click(&mut runtime, &folder);
    frames(&mut runtime, 40);
    assert_eq!(runtime.state.graph_view.nodes.len(), 3);
    let folder = node_control(&runtime, "src/center");
    double_click(&mut runtime, &folder);
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Class))
    );
    let caller = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|node| node.label == "Caller")
        .unwrap()
        .key
        .clone();
    runtime.state.nav.expanded.insert(caller);
    let before = runtime.state.nav.expanded.clone();
    let center = node_control(&runtime, "Center");
    click(&mut runtime, &center);
    frames(&mut runtime, 40);
    assert_eq!(runtime.state.nav.expanded, before);
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Class))
    );
    assert_eq!(runtime.state.graph_view.nodes.len(), 3);
    runtime.state.set_depth(2);
    frames(&mut runtime, 3);
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|node| node.group_kind == Some(GroupKind::Class))
    );
}

#[test]
fn single_click_walks_both_sides_and_restores_previous_selection_and_camera() {
    let mut runtime = runtime();
    runtime.state.focus_graph_method(1);
    frames(&mut runtime, 3);
    centered(&runtime, 1);
    let previous = serde_json::to_value(&runtime.state.nav).unwrap();
    let before = runtime.state.back.len();
    let key = method_control(&runtime, 2);
    click(&mut runtime, &key);
    assert_eq!(runtime.state.selected(), Some(2));
    assert_eq!(runtime.state.back.len(), before);
    frames(&mut runtime, 40);
    assert_eq!(runtime.state.back.len(), before + 1);
    assert_eq!(
        runtime.state.nav.focus_target,
        Some(FocusTarget::Method("m2".into()))
    );
    let members: Vec<_> = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .filter_map(|n| n.member)
        .collect();
    assert_eq!(members, [1, 2, 17]);
    centered(&runtime, 2);
    click(&mut runtime, "back");
    assert_eq!(serde_json::to_value(&runtime.state.nav).unwrap(), previous);
    click(&mut runtime, "forward");
    assert_eq!(runtime.state.selected(), Some(2));
    centered(&runtime, 2);
}

#[test]
fn double_click_reaches_classes_then_class_method_tree_without_extra_history() {
    let mut runtime = runtime();
    let before = runtime.state.back.len();
    classes(&mut runtime);
    assert_eq!(runtime.state.back.len(), before + 1);
    let center = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|n| n.label == "Center")
        .unwrap();
    assert_eq!(center.methods.len(), 16);
    assert_ne!(center.methods[0].id, center.methods[1].id);
    let overloads: Vec<_> = center
        .methods
        .iter()
        .filter(|row| row.member == 1 || row.member == 2)
        .collect();
    assert_eq!(overloads[0].label, "work");
    assert_eq!(overloads[1].label, "work");
    assert_ne!(overloads[0].id, overloads[1].id);
    let key = node_control(&runtime, "Center");
    double_click(&mut runtime, &key);
    assert_eq!(runtime.state.back.len(), before + 2);
    assert!(matches!(
        runtime.state.nav.focus_target,
        Some(FocusTarget::ClassMethods(_))
    ));
    assert_eq!(runtime.state.graph_view.focus_keys.len(), 16);
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|n| n.member.is_some())
    );
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .any(|n| n.member == Some(0))
    );
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .any(|n| n.member == Some(17))
    );
    assert!(
        !runtime
            .state
            .graph_view
            .nodes
            .iter()
            .any(|n| n.member == Some(18))
    );
}

#[test]
fn clicking_a_class_method_row_enters_method_tree_without_expanding_header() {
    let mut runtime = runtime();
    classes(&mut runtime);
    let class_key = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|n| n.label == "Center")
        .unwrap()
        .key
        .clone();
    let before = runtime.state.back.len();
    click(&mut runtime, "graph-method-1");
    frames(&mut runtime, 3);
    assert_eq!(
        runtime.state.nav.focus_target,
        Some(FocusTarget::Method("m1".into()))
    );
    assert!(!runtime.state.nav.expanded.contains(&class_key));
    assert_eq!(runtime.state.back.len(), before + 1);
    centered(&runtime, 1);
    let before = runtime.state.back.len();
    let key = method_control(&runtime, 1);
    double_click(&mut runtime, &key);
    assert_eq!(runtime.state.back.len(), before + 1);
    assert_eq!(
        runtime.state.nav.focus_target,
        Some(FocusTarget::Method("m1".into()))
    );
    assert!(runtime.state.flow_workspace.current.is_none());
}

fn visibility_runtime() -> Runtime<State> {
    use codecallnavigator::{model::Visibility, store::SqliteStore};
    use std::sync::atomic::AtomicBool;
    let mut runtime = runtime();
    let mut data = (*runtime.state.graph.as_ref().unwrap().data).clone();
    for node in &mut data.nodes {
        node.language = "java".into();
        node.container = format!("sample.{}", node.container);
    }
    for node in &mut data.nodes[1..17] {
        node.container_visibility = Some(Visibility::Public);
        node.visibility = Some(Visibility::Private);
    }
    data.nodes[2].visibility = Some(Visibility::Protected);
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(directory.path()).unwrap();
    store.publish(&data, &AtomicBool::new(false)).unwrap();
    runtime.state.graph = Some(Arc::new(
        GraphIndex::new(Arc::new(store.load_active().unwrap().unwrap())).unwrap(),
    ));
    runtime.state.changed();
    frames(&mut runtime, 3);
    runtime
}

#[test]
fn qualified_classes_and_overloads_show_visibility_after_storage_roundtrip() {
    let mut runtime = visibility_runtime();
    classes(&mut runtime);
    let center = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|node| node.label == "+ Center")
        .unwrap();
    assert!(center.key.contains("sample.Center"));
    assert_eq!(center.description, "public · sample.Center");
    let header_key = node_control(&runtime, "+ Center");
    let header = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some(&header_key))
        .unwrap();
    assert!(
        header
            .label
            .as_deref()
            .unwrap()
            .contains("public · sample.Center")
    );
    assert_eq!(header.help.as_deref(), Some("public · sample.Center"));
    assert_eq!(center.methods[0].label, "− work");
    assert_eq!(center.methods[1].label, "# work");
    for (member, label, access) in [(1, "− work", "private"), (2, "# work", "protected")] {
        let key = format!("graph-method-{member}");
        let element = runtime
            .elements()
            .into_iter()
            .find(|element| element.key.as_deref() == Some(&key))
            .unwrap();
        assert!(element.label.as_deref().unwrap().starts_with(label));
        assert!(element.label.as_deref().unwrap().contains(access));
        assert!(element.help.as_deref().unwrap().contains(access));
        click(&mut runtime, &key);
        frames(&mut runtime, 3);
        assert_eq!(
            runtime.state.nav.focus_target,
            Some(FocusTarget::Method(format!("m{member}")))
        );
        assert!(
            runtime
                .state
                .graph_view
                .nodes
                .iter()
                .any(|node| node.member == Some(member) && node.label == label)
        );
        click(&mut runtime, "back");
        frames(&mut runtime, 60);
    }
}

#[test]
fn legacy_focus_restores_and_filter_removal_returns_to_overview() {
    let mut runtime = runtime();
    runtime.state.nav = serde_json::from_value::<Navigation>(serde_json::json!({
        "grouping":"Method", "focus":"m2", "selected":"m2", "depth":1
    }))
    .unwrap();
    runtime.state.changed();
    frames(&mut runtime, 3);
    assert_eq!(runtime.state.graph_view.focus_keys, ["m2"]);
    centered(&runtime, 2);
    let mut data = runtime.state.graph.as_ref().unwrap().data.as_ref().clone();
    data.nodes[2].is_generated = true;
    runtime.state.graph = Some(Arc::new(GraphIndex::new(Arc::new(data)).unwrap()));
    runtime.state.changed();
    frames(&mut runtime, 4);
    assert!(!runtime.state.has_graph_focus());
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|n| n.member != Some(2))
    );
}

#[test]
fn explicit_zoom_cancels_pending_focus_camera_placement() {
    let mut runtime = runtime();
    runtime.state.focus_graph_method(2);
    runtime.state.zoom(1.2);
    let camera = runtime.state.nav.camera.clone();
    frames(&mut runtime, 3);
    assert_eq!(runtime.state.nav.camera.pan, camera.pan);
    assert_eq!(runtime.state.nav.camera.zoom, camera.zoom);
}

#[test]
fn changing_depth_recenters_the_existing_focus_without_changing_zoom() {
    let mut runtime = runtime();
    runtime.state.focus_graph_method(2);
    frames(&mut runtime, 3);
    let zoom = runtime.state.nav.camera.zoom;
    runtime.state.set_depth(2);
    frames(&mut runtime, 3);
    centered(&runtime, 2);
    assert_eq!(runtime.state.nav.camera.zoom, zoom);
}

#[test]
fn a_large_class_scrolls_to_its_last_method_without_zooming_or_drilling() {
    let mut runtime = runtime();
    classes(&mut runtime);
    let camera = runtime.state.nav.camera.clone();
    let navigation = serde_json::to_value(&runtime.state.nav).unwrap();
    let (x, y) = runtime.find("graph-method-1").unwrap();
    runtime.wheel_xy(x, y, 0.0, -10_000.0);
    frames(&mut runtime, 30);
    let row = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("graph-method-1"))
        .unwrap();
    assert!(
        row.scrolled[1] > 0.0,
        "The class methods must scroll independently"
    );
    assert_eq!(runtime.state.nav.camera.pan, camera.pan);
    assert_eq!(runtime.state.nav.camera.zoom, camera.zoom);
    assert_eq!(
        serde_json::to_value(&runtime.state.nav).unwrap(),
        navigation
    );
    // Signature sorting puts work(9) last after the overloads work(10)..work(16).
    click(&mut runtime, "graph-method-9");
    frames(&mut runtime, 3);
    assert_eq!(
        runtime.state.nav.focus_target,
        Some(FocusTarget::Method("m9".into()))
    );
    centered(&runtime, 9);
}

#[test]
fn selecting_a_class_uses_box_hops_and_omits_unrelated_classes() {
    let mut runtime = runtime();
    classes(&mut runtime);
    let key = node_control(&runtime, "Center");
    click(&mut runtime, &key);
    frames(&mut runtime, 40);
    let view = &runtime.state.graph_view;
    assert_eq!(
        view.nodes
            .iter()
            .map(|node| node.label.as_str())
            .collect::<Vec<_>>(),
        ["Caller", "Center", "Sink"]
    );
    assert_eq!(
        view.ordering
            .iter()
            .map(|order| order.rank)
            .collect::<Vec<_>>(),
        [-1, 0, 1]
    );
    assert_eq!(view.nodes[1].methods.len(), 16);
    let node = &view.nodes[1];
    let center = runtime.state.nav.camera.screen([
        node.position[0] + node.size[0] / 2.0,
        node.position[1] + node.size[1] / 2.0,
    ]);
    assert!((center[0] - runtime.state.viewport[0] / 2.0).abs() < 0.1);
    assert!((center[1] - runtime.state.viewport[1] / 2.0).abs() < 0.1);
}

#[test]
fn restored_layer_focus_survives_missing_assignments_then_validates_membership() {
    use crate::projection::Grouping;
    use codecallnavigator::architecture::LayerAssignments;
    let mut runtime = runtime();
    runtime.state.nav.grouping = Grouping::Layer;
    runtime.state.nav.focus_method("m2".into());
    runtime.state.nav.selected = Some("m2".into());
    runtime.state.changed();
    frames(&mut runtime, 4);
    assert_eq!(
        runtime.state.nav.focus_target,
        Some(FocusTarget::Method("m2".into()))
    );
    assert!(runtime.state.architecture.is_none());
    assert!(!runtime.state.graph_view.focus_valid);

    let zoom = runtime.state.nav.camera.zoom;
    runtime.state.architecture = Some(Arc::new(LayerAssignments::default()));
    runtime.state.changed();
    frames(&mut runtime, 4);
    assert!(runtime.state.graph_view.focus_valid);
    assert_eq!(runtime.state.graph_view.focus_keys, ["m2"]);
    assert_eq!(runtime.state.nav.camera.zoom, zoom);
    centered(&runtime, 2);

    let mut data = runtime.state.graph.as_ref().unwrap().data.as_ref().clone();
    data.nodes[2].is_generated = true;
    runtime.state.graph = Some(Arc::new(GraphIndex::new(Arc::new(data)).unwrap()));
    runtime.state.changed();
    frames(&mut runtime, 4);
    assert!(!runtime.state.has_graph_focus());
}

#[path = "traversal/emphasis.rs"]
mod emphasis;
