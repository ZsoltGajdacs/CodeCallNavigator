use crate::{app::State, jobs, scenes, view};
use rune::{Env, Runtime};

mod graph_studio;
mod source_popup;

fn runtime() -> Runtime<State> {
    let mut state = State::new();
    scenes::install(&mut state);
    let mut runtime = Runtime::new(state, view::view, (1420.0, 900.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    runtime.frame(0.0);
    runtime
}

fn click(runtime: &mut Runtime<State>, key: &str) {
    let (x, y) = runtime
        .find(key)
        .unwrap_or_else(|| panic!("Missing control {key}"));
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(1.0 / 60.0);
    runtime.frame(1.0 / 60.0);
}

#[test]
fn native_node_selection_and_history_restore_focus_and_camera() {
    let mut runtime = runtime();
    let expected = runtime.state.graph_view.nodes[0].member.unwrap();
    click(&mut runtime, "graph-node-0");
    assert_eq!(runtime.state.selected(), Some(expected));
    let focus = runtime.state.nav.focus.clone();
    let camera = runtime.state.nav.camera.pan;
    click(&mut runtime, "overview");
    assert!(runtime.state.nav.focus.is_none());
    click(&mut runtime, "back");
    assert_eq!(runtime.state.nav.focus, focus);
    assert_eq!(runtime.state.nav.camera.pan, camera);
}

#[test]
fn native_path_controls_show_a_real_directed_path_and_filter_can_reveal_tests() {
    let mut runtime = runtime();
    runtime.state.select(0, false);
    runtime.frame(1.0 / 60.0);
    click(&mut runtime, "path-from");
    runtime.state.select(3, false);
    runtime.frame(1.0 / 60.0);
    click(&mut runtime, "path-to");
    click(&mut runtime, "find-path");
    assert!(runtime.state.nav.show_path);
    assert_eq!(runtime.state.graph_view.nodes.len(), 4);
    assert!(runtime.state.graph_view.message.contains("3 steps"));
    runtime.state.reveal(5);
    runtime.frame(1.0 / 60.0);
    assert!(runtime.state.nav.filter.include_tests);
    assert_eq!(runtime.state.selected(), Some(5));
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .any(|node| node.member == Some(5))
    );
}

#[test]
fn stale_worker_result_updates_the_native_status_text() {
    let mut runtime = runtime();
    let epoch = runtime.state.epoch;
    runtime
        .state
        .jobs
        .stale
        .request((), move |_| (epoch, Ok(true)));
    for _ in 0..100 {
        runtime.frame(1.0 / 60.0);
        if runtime.state.stale {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(runtime.state.stale);
    assert!(runtime.elements().iter().any(|element| {
        element
            .text
            .as_deref()
            .is_some_and(|text| text.contains("Source files changed"))
    }));
}

#[test]
fn cancelling_the_folder_sheet_preserves_the_current_workspace() {
    let mut runtime = runtime();
    let root = runtime.state.root.text.clone();
    click(&mut runtime, "choose-folder");
    assert!(runtime.file_dialog().is_some());
    runtime.resolve_file_dialog(None).unwrap();
    runtime.frame(1.0 / 60.0);
    assert_eq!(runtime.state.root.text, root);
    assert!(runtime.state.warning.is_empty());
}

#[test]
fn native_command_shortcuts_focus_search_and_open_the_folder_sheet() {
    let mut runtime = runtime();
    runtime.on_key = Some(view::key);
    let modifiers = rune::Mods {
        cmd: true,
        logo: true,
        ..Default::default()
    };
    runtime.key(rune::KeyEvent {
        key: rune::Key::Command('f'),
        mods: modifiers,
    });
    runtime.frame(1.0 / 60.0);
    assert!(runtime.state.query.focused);
    runtime.key(rune::KeyEvent {
        key: rune::Key::Command('o'),
        mods: modifiers,
    });
    runtime.frame(1.0 / 60.0);
    assert!(runtime.file_dialog().is_some());
    runtime.resolve_file_dialog(None).unwrap();
}

#[test]
fn restored_navigation_keeps_exact_ids_and_discards_missing_symbols() {
    let runtime = runtime();
    let mut nav = runtime.state.nav.clone();
    nav.selected = Some("fixture/project/LoadOrder".into());
    nav.focus = Some("unknown-build/same-method-name".into());
    nav.path_from = nav.selected.clone();
    nav.path_to = nav.focus.clone();
    nav.show_path = true;
    nav.retain_known(runtime.state.graph.as_ref().unwrap());
    assert_eq!(nav.selected.as_deref(), Some("fixture/project/LoadOrder"));
    assert!(nav.focus.is_none());
    assert!(nav.path_to.is_none());
    assert!(!nav.show_path);
}

#[test]
fn corrupt_session_is_recoverable_while_the_cached_graph_still_opens() {
    let directory = tempfile::tempdir().unwrap();
    let store = codegraph::store::SqliteStore::open(directory.path()).unwrap();
    let runtime = runtime();
    store
        .publish(
            &runtime.state.graph.as_ref().unwrap().data,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
    store
        .save_session(&serde_json::json!({ "navigation": "corrupt" }))
        .unwrap();
    let loaded = jobs::load_boot(directory.path().into()).unwrap();
    assert!(loaded.graph.is_some());
    assert!(loaded.session.is_none());
    assert!(
        loaded
            .warnings
            .iter()
            .any(|warning| warning.contains("navigation could not be restored"))
    );
}

#[test]
fn a_late_discovery_result_cannot_populate_builds_for_a_different_root() {
    let mut runtime = runtime();
    runtime.state.root = rune::TextEdit::new("/project/B");
    runtime.state.root_edited();
    runtime.state.jobs.discovery.request((), |_| {
        std::thread::sleep(std::time::Duration::from_millis(10));
        jobs::DiscoveryOutput {
            root: "/project/A".into(),
            result: Ok(vec![crate::discovery::BuildTarget {
                path: "/project/A/App.csproj".into(),
                language: "csharp".into(),
                kind: "project".into(),
                label: "App".into(),
            }]),
        }
    });
    wait_for(&mut runtime, |state| !state.jobs.discovery.is_pending());
    runtime.frame(1.0 / 60.0);
    assert!(runtime.state.targets.is_empty());
    assert_eq!(runtime.state.root.text, "/project/B");
    assert!(runtime.state.status.contains("Folder changed"));
}

#[test]
fn search_and_neighbor_pages_can_reach_every_indexed_match() {
    use codegraph::{
        graph::GraphIndex,
        model::{Edge, GraphData, Node},
        store::SqliteStore,
    };
    use std::sync::{Arc, atomic::AtomicBool};
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteStore::open(directory.path()).unwrap());
    let nodes = (0..95)
        .map(|index| Node {
            id: format!("build/project/{index}"),
            label: format!("Method{index}"),
            signature: format!("Method{index}()"),
            ..Default::default()
        })
        .collect();
    let edges = (1..95)
        .map(|to| Edge {
            provenance: String::new(),
            from: 0,
            to,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
        })
        .collect();
    let data = Arc::new(GraphData {
        nodes,
        edges,
        ..Default::default()
    });
    let generation = store.publish(&data, &AtomicBool::new(false)).unwrap();
    let mut state = State::new();
    state.graph = Some(Arc::new(GraphIndex::new(data).unwrap()));
    state.store = Some(store);
    state.generation = Some(generation);
    state.query = rune::TextEdit::new("Method");
    state.request_search();
    let mut runtime = Runtime::new(state, view::view, (1420.0, 900.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    wait_for(&mut runtime, |state| !state.hits.is_empty());
    let mut found = runtime.state.hits.clone();
    while runtime.state.search_more {
        runtime.state.step_search(true);
        wait_for(&mut runtime, |state| !state.hits.is_empty());
        found.extend_from_slice(&runtime.state.hits);
    }
    found.sort_unstable();
    found.dedup();
    assert_eq!(found.len(), 95);
    runtime.state.select(0, false);
    wait_for(&mut runtime, |state| state.neighbors.1.len() == 94);
    runtime.state.step_neighbors(false, true);
    assert_eq!(runtime.state.neighbor_pages[1], 1);
    assert_eq!(runtime.state.relationships.len(), 94);
}

fn wait_for(runtime: &mut Runtime<State>, ready: impl Fn(&State) -> bool) {
    for _ in 0..1000 {
        runtime.frame(1.0 / 60.0);
        if ready(&runtime.state) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("The background UI result did not arrive");
}

#[test]
fn method_control_starts_at_entries_and_overview_restores_them() {
    use crate::projection::{FlowDirection, Grouping, Scope};
    let mut runtime = runtime();
    runtime.state.nav.direction = FlowDirection::Callers;
    runtime.state.nav.page = 7;
    click(&mut runtime, "grouping/segment3");
    assert_eq!(runtime.state.nav.grouping, Grouping::Method);
    assert_eq!(runtime.state.nav.direction, FlowDirection::Callees);
    assert_eq!(runtime.state.nav.page, 0);
    assert!(matches!(
        runtime.state.request().unwrap().scope,
        Scope::Overview
    ));
    assert!(runtime.find("direction-Both").is_none());
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .any(|node| node.application_start)
    );
    assert!(runtime.elements().iter().any(|element| {
        element
            .label
            .as_deref()
            .is_some_and(|text| text.contains("Application start"))
    }));
    runtime.state.select(1, true);
    runtime.frame(1.0 / 60.0);
    assert!(matches!(
        runtime.state.request().unwrap().scope,
        Scope::Focus(1)
    ));
    click(&mut runtime, "overview");
    assert!(matches!(
        runtime.state.request().unwrap().scope,
        Scope::Overview
    ));
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .any(|node| node.member == Some(0))
    );
    click(&mut runtime, "back");
    assert!(matches!(
        runtime.state.request().unwrap().scope,
        Scope::Focus(1)
    ));
}

#[test]
fn changing_flow_resets_page_refits_and_history_restores_previous_flow() {
    use crate::projection::FlowDirection;
    let mut runtime = runtime();
    runtime.state.select(2, true);
    runtime.frame(1.0 / 60.0);
    runtime.state.nav.page = 3;
    click(&mut runtime, "direction/segment0");
    assert_eq!(runtime.state.nav.direction, FlowDirection::Callers);
    assert_eq!(runtime.state.nav.page, 0);
    assert_eq!(
        runtime.state.nav.focus.as_deref(),
        Some("fixture/project/ValidateOrder")
    );
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .any(|node| node.member == Some(1))
    );
    click(&mut runtime, "back");
    assert_eq!(runtime.state.nav.direction, FlowDirection::Callees);
    assert_eq!(runtime.state.nav.page, 3);
}

#[test]
fn legacy_navigation_directions_and_method_sessions_roundtrip() {
    use crate::{
        app::SavedSession,
        projection::{FlowDirection, Grouping},
    };
    for (legacy, expected) in [
        ("Both", FlowDirection::Callees),
        ("Incoming", FlowDirection::Callers),
        ("Outgoing", FlowDirection::Callees),
    ] {
        let session: SavedSession = serde_json::from_value(serde_json::json!({
            "navigation": {"direction": legacy, "grouping": "Method"}
        }))
        .unwrap();
        assert_eq!(session.navigation.direction, expected);
        assert_eq!(session.navigation.grouping, Grouping::Method);
        let restored: SavedSession =
            serde_json::from_value(serde_json::to_value(&session).unwrap()).unwrap();
        assert_eq!(restored.navigation.direction, expected);
        assert_eq!(restored.navigation.grouping, Grouping::Method);
    }
}

#[test]
fn saved_build_choices_distinguish_legacy_gradle_from_new_maven_selections() {
    use crate::{discovery::BuildIdentity, navigation::SavedSession};
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    for file in ["build.gradle", "pom.xml"] {
        std::fs::write(root.join(file), "").unwrap();
    }
    let legacy: SavedSession = serde_json::from_value(serde_json::json!({
        "root": root,
        "selected_builds": [root]
    }))
    .unwrap();
    let mut runtime = runtime();
    restore_discovered_builds(&mut runtime, &legacy);
    let selected = |runtime: &Runtime<State>| {
        runtime
            .state
            .targets
            .iter()
            .filter(|choice| choice.selected)
            .map(|choice| choice.target.kind.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(selected(&runtime), vec!["gradle"]);

    let mut modern = legacy;
    modern.selected_targets = Some(vec![BuildIdentity {
        path: root,
        kind: "maven".into(),
    }]);
    restore_discovered_builds(&mut runtime, &modern);
    assert_eq!(selected(&runtime), vec!["maven"]);
    let round_trip: SavedSession =
        serde_json::from_value(serde_json::to_value(runtime.state.session()).unwrap()).unwrap();
    restore_discovered_builds(&mut runtime, &round_trip);
    assert_eq!(selected(&runtime), vec!["maven"]);

    modern.selected_targets = Some(Vec::new());
    restore_discovered_builds(&mut runtime, &modern);
    assert!(selected(&runtime).is_empty());
}

fn restore_discovered_builds(
    runtime: &mut Runtime<State>,
    session: &crate::navigation::SavedSession,
) {
    runtime.state.root = rune::TextEdit::new(&session.root);
    runtime.state.saved_builds = session.selected_builds.clone();
    runtime.state.saved_targets = session.selected_targets.clone();
    runtime.state.discover_root();
    for _ in 0..100 {
        runtime.frame(1.0 / 60.0);
        if runtime.state.targets.len() == 2 {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("Maven and Gradle targets were not discovered");
}

#[test]
fn graph_click_centers_and_double_click_focuses_without_extra_history() {
    let mut runtime = runtime();
    let (x, y) = runtime.find("graph-node-0").unwrap();
    let key = runtime.state.graph_view.nodes[0].key.clone();
    let before = runtime.state.back.len();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.05);
    assert_eq!(
        runtime.state.visual_selection.as_deref(),
        Some(key.as_str())
    );
    assert_eq!(runtime.state.back.len(), before);
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.05);
    assert_eq!(runtime.state.back.len(), before + 1);
    assert!(runtime.state.nav.focus.is_some());
}

#[test]
fn graph_single_click_centers_after_click_interval_and_wheel_zooms() {
    let mut runtime = runtime();
    let node = runtime.state.graph_view.nodes[0].clone();
    let (x, y) = runtime.find("graph-node-0").unwrap();
    let zoom = runtime.state.nav.camera.zoom;
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    for _ in 0..90 {
        runtime.frame(1.0 / 60.0);
    }
    let center = runtime.state.nav.camera.screen([
        node.position[0] + node.size[0] / 2.0,
        node.position[1] + node.size[1] / 2.0,
    ]);
    assert!((center[0] - runtime.state.viewport[0] / 2.0).abs() < 0.1);
    assert!((center[1] - runtime.state.viewport[1] / 2.0).abs() < 0.1);
    assert_eq!(runtime.state.nav.camera.zoom, zoom);
    let rect = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("graph-canvas"))
        .unwrap()
        .rect;
    runtime.wheel_xy(rect[0] + 5.0, rect[1] + 5.0, 0.0, -100.0);
    runtime.frame(1.0 / 60.0);
    assert!(runtime.state.nav.camera.zoom > zoom);
}

#[test]
fn group_double_click_expands_and_selection_is_reconciled() {
    let mut runtime = runtime();
    runtime
        .state
        .set_grouping(crate::projection::Grouping::Project);
    runtime.frame(1.0 / 60.0);
    assert!(runtime.state.graph_view.nodes[0].member.is_none());
    let key = runtime.state.graph_view.nodes[0].key.clone();
    let (x, y) = runtime.find("graph-node-0").unwrap();
    let before = runtime.state.back.len();
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.05);
    assert!(runtime.state.nav.expanded.is_empty());
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    runtime.frame(0.05);
    assert!(runtime.state.nav.expanded.contains(&key));
    assert_eq!(runtime.state.back.len(), before + 1);
    runtime.state.graph_view.nodes.clear();
    runtime.state.reconcile_visual_selection();
    assert!(runtime.state.visual_selection.is_none());
}

#[test]
fn inspector_selection_and_restored_selection_update_graph_emphasis() {
    let mut runtime = runtime();
    let first = runtime.state.graph_view.nodes[0].member.unwrap();
    let second = runtime.state.graph_view.nodes[1].member.unwrap();
    click(&mut runtime, "graph-node-0");
    assert_eq!(runtime.state.selected(), Some(first));
    runtime.state.select(second, false);
    runtime.frame(1.0 / 60.0);
    let second_key = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|node| node.member == Some(second))
        .unwrap()
        .key
        .clone();
    assert_eq!(
        runtime.state.visual_selection.as_deref(),
        Some(second_key.as_str())
    );
    runtime.state.visual_selection = None;
    runtime.state.reconcile_visual_selection();
    assert_eq!(
        runtime.state.visual_selection.as_deref(),
        Some(second_key.as_str())
    );
}
