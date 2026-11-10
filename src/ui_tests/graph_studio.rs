//! User-visible Rune Kit interactions, using the real runtime and production view.
use super::{click, runtime};
use crate::{
    app::State,
    jobs,
    presentation::{Drawer, Overlay, Section},
    scenes, view,
};
use rune::{Env, Key, KeyEvent, Mods, Runtime};

fn settle(runtime: &mut Runtime<State>) {
    for _ in 0..60 {
        runtime.frame(1.0 / 60.0);
    }
}

fn press(runtime: &mut Runtime<State>, key: Key) {
    runtime.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    settle(runtime);
}

fn narrow_runtime(max_panes: bool) -> Runtime<State> {
    let mut state = State::new();
    scenes::install(&mut state);
    if max_panes {
        state.ui.sidebar.set_width(400.0);
        state.ui.inspector.set_width(420.0);
    }
    let mut runtime = Runtime::new(state, view::view, (1050.0, 680.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    runtime.on_key = Some(view::key);
    settle(&mut runtime);
    runtime
}

fn assert_visible(runtime: &Runtime<State>, key: &str) {
    let element = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("Missing {key}"));
    assert!(
        element.visible[2] >= element.rect[2] - 1.0 && element.visible[3] >= element.rect[3] - 1.0,
        "{key} is clipped: rect {:?}, visible {:?}",
        element.rect,
        element.visible
    );
    assert!(
        element.rect[0] >= 0.0 && element.rect[0] + element.rect[2] <= 1050.5,
        "{key} extends outside the window: {:?}",
        element.rect
    );
    assert!(
        element.visible[2] > 10.0 && element.visible[3] > 10.0,
        "{key} is not usable"
    );
}

#[test]
fn long_build_paths_scroll_horizontally() {
    let mut runtime = narrow_runtime(false);
    runtime.update(|state, _| {
        state.targets = vec![crate::app::TargetChoice {
            target: crate::discovery::BuildTarget {
                path: "/project/build.csproj".into(),
                language: "C#".into(),
                kind: "project".into(),
                label: format!("/project/{}/build.csproj", "long-directory/".repeat(8)),
            },
            selected: false,
        }];
    });
    settle(&mut runtime);
    let before = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("build-0"))
        .unwrap()
        .rect;
    runtime.wheel_xy(before[0] + 50.0, before[1] + 10.0, -200.0, 0.0);
    runtime.frame(1.0 / 60.0);
    let after = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("build-0"))
        .unwrap()
        .rect;
    assert!(after[0] < before[0], "long build path did not scroll");
    assert_visible(&runtime, "choose-folder");
}

#[test]
fn build_scrollbar_stays_at_the_bottom_while_the_list_scrolls() {
    let mut runtime = narrow_runtime(false);
    runtime.update(|state, _| {
        state.targets = (0..40)
            .map(|index| crate::app::TargetChoice {
                target: crate::discovery::BuildTarget {
                    path: format!("/project/{index}.csproj").into(),
                    language: "C#".into(),
                    kind: "project".into(),
                    label: format!("/project/{}/{index}.csproj", "long-directory/".repeat(8)),
                },
                selected: false,
            })
            .collect();
    });
    settle(&mut runtime);
    let viewport = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("builds-scroll"))
        .unwrap()
        .rect;
    let first = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("build-0"))
        .unwrap()
        .rect;
    runtime.wheel(viewport[0] + 50.0, viewport[1] + 50.0, -200.0);
    runtime.frame(1.0 / 60.0);
    let after = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("build-0"))
        .unwrap()
        .rect;
    let viewport_after = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("builds-scroll"))
        .unwrap()
        .rect;
    assert!(after[1] < first[1], "build list did not scroll vertically");
    assert_eq!(
        viewport, viewport_after,
        "scrollbar viewport moved with the list"
    );
    runtime.frame(0.15);
    runtime.wheel_xy(viewport[0] + 50.0, viewport[1] + 50.0, -200.0, 0.0);
    runtime.frame(1.0 / 60.0);
    let horizontally_scrolled = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("build-0"))
        .unwrap()
        .rect;
    assert!(
        horizontally_scrolled[0] < after[0],
        "build list did not scroll horizontally"
    );
}

#[test]
fn minimum_window_keeps_graph_controls_visible_without_toolbar_scrolling() {
    for max_panes in [false, true] {
        let mut runtime = narrow_runtime(max_panes);
        runtime.update(|s, _| s.graph_view.pages = 3);
        settle(&mut runtime);
        for key in [
            "filter-tests",
            "filter-generated",
            "filter-unavailable",
            "filter-uncertain",
            "filter-references",
            "overview",
            "collapse-all",
            "grouping",
            "direction",
            "depth-trigger",
            "next-page",
            "zoom-out",
            "fit",
            "zoom-in",
        ] {
            assert_visible(&runtime, key);
        }
        let before = runtime.state.nav.camera.zoom;
        click(&mut runtime, "zoom-in");
        assert!(runtime.state.nav.camera.zoom > before);
    }
}

#[test]
fn activity_rail_switches_and_collapses_drawers_without_navigating_the_graph() {
    let mut runtime = runtime();
    let nav = serde_json::to_value(&runtime.state.nav).unwrap();
    let revision = runtime.state.revision;
    click(&mut runtime, "drawer-search");
    settle(&mut runtime);
    assert_eq!(runtime.state.ui.drawer, Drawer::Search);
    assert!(runtime.find("search/value/input").is_some());
    click(&mut runtime, "drawer-search");
    settle(&mut runtime);
    assert!(!runtime.state.ui.sidebar.is_open());
    assert!(runtime.find("filter-tests").is_some());
    assert_eq!(serde_json::to_value(&runtime.state.nav).unwrap(), nav);
    assert_eq!(runtime.state.revision, revision);
}

#[test]
fn command_f_reopens_search_and_focuses_the_actual_editor() {
    let mut runtime = runtime();
    runtime.on_key = Some(view::key);
    click(&mut runtime, "drawer-repository");
    settle(&mut runtime);
    assert!(!runtime.state.ui.sidebar.is_open());
    runtime.key(KeyEvent {
        key: Key::Command('f'),
        mods: Mods {
            cmd: true,
            logo: true,
            ..Default::default()
        },
    });
    settle(&mut runtime);
    assert_eq!(runtime.state.ui.drawer, Drawer::Search);
    assert!(runtime.state.ui.sidebar.is_open());
    let input = runtime
        .elements()
        .into_iter()
        .find(|e| {
            e.key.as_deref() == Some("input") && e.label.as_deref() == Some("Search all symbols")
        })
        .unwrap();
    assert!(
        input.focused,
        "Cmd-F must focus the actual editor, not only its backing state"
    );
}

#[test]
fn kit_checkbox_changes_graph_filter_and_inspector_disclosure_hides_only_its_section() {
    let mut runtime = runtime();
    assert!(!runtime.state.nav.filter.include_tests);
    click(&mut runtime, "filter-tests");
    assert!(runtime.state.nav.filter.include_tests);
    runtime.update(|state, _| {
        state.select_method(0, crate::graph_interaction::SelectionAction::Inspect);
    });
    settle(&mut runtime);
    assert!(runtime.find("path-from").is_some());
    click(&mut runtime, "path-section/trigger");
    settle(&mut runtime);
    assert!(!runtime.state.ui.section_open(Section::Path));
    assert!(runtime.find("path-from").is_none());
    assert!(runtime.find("focus-selected").is_some());
    click(&mut runtime, "path-section/trigger");
    settle(&mut runtime);
    assert!(runtime.find("path-from").is_some());
}

#[test]
fn overlay_filters_toggle_preserving_zoom_and_centering_the_focus() {
    let mut runtime = narrow_runtime(true);
    let before = runtime.state.nav.filter.clone();
    let zoom = runtime.state.nav.camera.zoom;
    for key in [
        "filter-tests",
        "filter-generated",
        "filter-unavailable",
        "filter-virtual-calls",
        "filter-uncertain",
        "filter-references",
    ] {
        click(&mut runtime, key);
        settle(&mut runtime);
    }
    let after = &runtime.state.nav.filter;
    assert_eq!(
        [
            after.include_tests,
            after.include_generated,
            after.include_unavailable,
            after.include_virtual_calls,
            after.include_uncertain,
            after.include_references
        ],
        [
            !before.include_tests,
            !before.include_generated,
            !before.include_unavailable,
            !before.include_virtual_calls,
            !before.include_uncertain,
            !before.include_references
        ]
    );
    assert_eq!(runtime.state.nav.camera.zoom, zoom);
    let anchor = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|node| runtime.state.graph_view.focus_keys.contains(&node.key))
        .unwrap();
    let center = runtime.state.nav.camera.screen([
        anchor.position[0] + anchor.size[0] / 2.0,
        anchor.position[1] + anchor.size[1] / 2.0,
    ]);
    assert!((center[0] - runtime.state.viewport[0] / 2.0).abs() < 0.1);
    assert!((center[1] - runtime.state.viewport[1] / 2.0).abs() < 0.1);
    let saved: crate::navigation::SavedSession =
        serde_json::from_str(&serde_json::to_string(&runtime.state.session()).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_value(saved.navigation.filter).unwrap(),
        serde_json::to_value(&runtime.state.nav.filter).unwrap()
    );
}

#[test]
fn virtual_calls_toggle_is_independent_and_restored_by_history() {
    let mut runtime = runtime();
    click(&mut runtime, "filter-uncertain");
    settle(&mut runtime);
    assert!(!runtime.state.nav.filter.include_uncertain);
    assert!(runtime.state.nav.filter.include_virtual_calls);
    click(&mut runtime, "filter-virtual-calls");
    settle(&mut runtime);
    assert!(!runtime.state.nav.filter.include_virtual_calls);
    assert!(!runtime.state.nav.filter.include_uncertain);
    click(&mut runtime, "back");
    settle(&mut runtime);
    assert!(runtime.state.nav.filter.include_virtual_calls);
    assert!(!runtime.state.nav.filter.include_uncertain);
    click(&mut runtime, "forward");
    settle(&mut runtime);
    assert!(!runtime.state.nav.filter.include_virtual_calls);
}

fn assert_filter_bar(runtime: &Runtime<State>) {
    let elements = runtime.elements();
    let canvas = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("graph-canvas"))
        .unwrap();
    let virtual_label = if canvas.rect[2] < 1150.0 {
        "Virtual calls"
    } else {
        "Virtual / interface calls"
    };
    assert!(
        elements
            .iter()
            .any(|e| e.text.as_deref() == Some(virtual_label))
    );
    let mut previous: Option<[f32; 4]> = None;
    for (key, label) in [
        ("filter-tests", "Test code"),
        ("filter-generated", "Generated code"),
        ("filter-unavailable", "Unavailable bodies"),
        ("filter-virtual-calls", "Virtual / interface calls"),
        ("filter-uncertain", "Possible / uncertain targets"),
        ("filter-references", "Callbacks / references"),
    ] {
        let control = elements
            .iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap();
        assert_filter_accessibility(control, label);
        assert!(
            control.rect[0] >= canvas.rect[0]
                && control.rect[0] + control.rect[2] <= canvas.rect[0] + canvas.rect[2] + 0.5
                && control.visible[2] >= control.rect[2] - 0.5,
            "{key} exceeds the graph or is clipped: {:?}",
            control.rect
        );
        if let Some(previous) = previous {
            assert!(
                control.rect[0] >= previous[0] + previous[2] - 0.5
                    || control.rect[1] >= previous[1] + previous[3] - 0.5,
                "{key} overlaps the preceding filter"
            );
        }
        previous = Some(control.rect);
    }
}

fn assert_filter_accessibility(control: &rune::ElementInfo, label: &str) {
    let help = if control.key.as_deref() == Some("filter-virtual-calls") {
        "Calls to declared virtual or interface methods. An override may execute at runtime."
    } else {
        label
    };
    assert_eq!(
        (
            control.role,
            control.label.as_deref(),
            control.help.as_deref(),
            control.focusable
        ),
        ("checkbox", Some(label), Some(help), true),
        "{label} accessibility"
    );
}

#[test]
fn overlay_filters_fit_and_expose_full_names_in_both_themes() {
    for dark in [false, true] {
        for (width, height, panes) in [
            (1050.0, 680.0, true),
            (1420.0, 900.0, true),
            (1030.0, 680.0, false),
            (1130.0, 680.0, false),
            (1230.0, 900.0, false),
            (1420.0, 900.0, false),
        ] {
            let mut state = State::new();
            scenes::install(&mut state);
            state.ui.sidebar.set_width(400.0);
            state.ui.inspector.set_width(420.0);
            state.ui.sidebar.set_open(panes);
            state.ui.inspector.set_open(panes);
            let mut runtime = Runtime::new(state, view::view, (width, height), Env::new(dark, 1.0));
            runtime.poll = Some(jobs::poll);
            settle(&mut runtime);
            assert_filter_bar(&runtime);
        }
    }
}

#[test]
fn depth_select_changes_real_neighborhood_and_escape_dismisses_before_overview() {
    let mut runtime = runtime();
    runtime.on_key = Some(view::key);
    let focus = runtime.state.nav.focus.clone();
    click(&mut runtime, "depth-trigger");
    settle(&mut runtime);
    assert_eq!(runtime.state.ui.overlay, Some(Overlay::Depth));
    press(&mut runtime, Key::Escape);
    assert_eq!(runtime.state.ui.overlay, None);
    assert_eq!(runtime.state.nav.focus, focus);
    click(&mut runtime, "depth-trigger");
    settle(&mut runtime);
    click(&mut runtime, "option1");
    settle(&mut runtime);
    assert_eq!(runtime.state.nav.depth, 2);
    assert_eq!(runtime.state.ui.overlay, None);
    press(&mut runtime, Key::Escape);
    assert!(runtime.state.nav.focus.is_none());
}

#[test]
fn kit_pane_drag_and_visibility_preserve_camera_and_persist_preferences() {
    let mut runtime = runtime();
    let camera = runtime.state.nav.camera.clone();
    let revision = runtime.state.revision;
    let (x, y) = runtime.find("sidebar-grip").unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_move(x + 40.0, y);
    runtime.pointer_up(x + 40.0, y);
    settle(&mut runtime);
    assert_eq!(runtime.state.ui.sidebar.width(), 320.0);
    click(&mut runtime, "toggle-inspector");
    settle(&mut runtime);
    assert!(!runtime.state.ui.inspector.is_open());
    assert_eq!(runtime.state.nav.camera.pan, camera.pan);
    assert_eq!(runtime.state.nav.camera.zoom, camera.zoom);
    assert_eq!(runtime.state.revision, revision);
    let session: crate::navigation::SavedSession =
        serde_json::from_value(serde_json::to_value(runtime.state.session()).unwrap()).unwrap();
    assert_eq!(session.presentation.sidebar_width, 320.0);
    assert!(!session.presentation.inspector_open);
}

#[test]
fn disabled_kit_analysis_control_does_not_start_work() {
    let mut runtime = runtime();
    assert!(runtime.state.store.is_none());
    assert!(runtime.state.targets.is_empty());
    click(&mut runtime, "analyze");
    assert!(!runtime.state.analyzing);
    assert!(!runtime.state.jobs.analysis.is_pending());
}

#[test]
fn segmented_grouping_supports_keyboard_navigation_and_activation() {
    let mut runtime = runtime();
    runtime.on_key = Some(view::key);
    click(&mut runtime, "grouping/segment0");
    press(&mut runtime, Key::Right);
    press(&mut runtime, Key::Text(" ".into()));
    assert_eq!(
        runtime.state.nav.grouping,
        crate::projection::Grouping::Namespace
    );
}

#[test]
fn a_search_result_reveals_a_method_hidden_by_graph_filters() {
    let mut runtime = runtime();
    runtime.update(|s, _| {
        s.open_search();
        s.hits = vec![5];
    });
    settle(&mut runtime);
    assert!(!runtime.state.nav.filter.include_tests);
    let result = runtime
        .elements()
        .into_iter()
        .find(|e| {
            e.role == "option"
                && e.label
                    .as_deref()
                    .is_some_and(|label| label.contains("hidden, click to reveal"))
        })
        .unwrap();
    let (x, y) = (
        result.visible[0] + result.visible[2] / 2.0,
        result.visible[1] + result.visible[3] / 2.0,
    );
    runtime.pointer_down(x, y);
    runtime.pointer_up(x, y);
    settle(&mut runtime);
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

fn drag_pane(runtime: &mut Runtime<State>, key: &str, delta: f32) {
    let (x, y) = runtime.find(key).unwrap();
    runtime.pointer_down(x, y);
    runtime.pointer_move(x + delta, y);
    runtime.pointer_up(x + delta, y);
    settle(runtime);
}

#[test]
fn live_pane_drags_keep_controls_inside_the_minimum_window() {
    let mut runtime = narrow_runtime(false);
    let camera = runtime.state.nav.camera.clone();
    let revision = runtime.state.revision;
    drag_pane(&mut runtime, "sidebar-grip", 180.0);
    drag_pane(&mut runtime, "inspector-grip", -180.0);
    let shown = runtime.state.ui.sidebar.shown_width() + runtime.state.ui.inspector.shown_width();
    assert!(
        shown <= 1050.0 - 48.0 - 448.0,
        "Live pane resizing consumed graph space: {shown}"
    );
    for key in ["grouping", "direction", "depth-trigger", "zoom-in", "fit"] {
        assert_visible(&runtime, key);
    }
    assert_eq!(runtime.state.nav.camera.pan, camera.pan);
    assert_eq!(runtime.state.nav.camera.zoom, camera.zoom);
    assert_eq!(runtime.state.revision, revision);
}

fn element(runtime: &Runtime<State>, key: &str) -> rune::ElementInfo {
    runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("Missing {key}"))
}

fn assert_inside(inner: [f32; 4], outer: [f32; 4], padding: f32) {
    assert!(
        inner[0] >= outer[0] + padding - 0.5
            && inner[0] + inner[2] <= outer[0] + outer[2] - padding + 0.5,
        "Content {:?} exceeds pane {:?} padding {padding}",
        inner,
        outer
    );
}

#[test]
fn drawer_fields_and_long_inspector_signatures_stay_inside_their_panes() {
    let mut runtime = narrow_runtime(false);
    let drawer = element(&runtime, "drawer").rect;
    assert_inside(element(&runtime, "root-path").rect, drawer, 12.0);
    click(&mut runtime, "drawer-search");
    settle(&mut runtime);
    let drawer = element(&runtime, "drawer").rect;
    assert_inside(element(&runtime, "search").rect, drawer, 12.0);
    let signature = format!(
        "execute({}) -> reactor.core.publisher.Mono<Void>",
        "com.example.application.really.long.package.ExtractionContext".repeat(4)
    );
    runtime.update(|state, _| {
        let mut data = state.graph.as_ref().unwrap().data.as_ref().clone();
        data.nodes[0].signature = signature.clone();
        state.graph = Some(std::sync::Arc::new(
            codegraph::graph::GraphIndex::new(std::sync::Arc::new(data)).unwrap(),
        ));
        state.select_method(0, crate::graph_interaction::SelectionAction::Inspect);
    });
    settle(&mut runtime);
    let inspector = element(&runtime, "inspector-pane").rect;
    let signature_text = runtime
        .elements()
        .into_iter()
        .find(|e| e.text.as_deref() == Some(&signature))
        .unwrap();
    assert_inside(signature_text.rect, inspector, 16.0);
    assert!(
        signature_text.rect[3] > 36.0,
        "Long signature should wrap into readable lines"
    );
}

#[test]
fn system_appearance_switch_rethemes_the_live_workspace_without_losing_search() {
    let mut runtime = narrow_runtime(false);
    click(&mut runtime, "drawer-search");
    settle(&mut runtime);
    let dark_canvas = element(&runtime, "graph-canvas").fill;
    runtime.set_system_dark(false);
    settle(&mut runtime);
    assert_ne!(element(&runtime, "graph-canvas").fill, dark_canvas);
    assert_eq!(runtime.state.ui.drawer, Drawer::Search);
    assert_visible(&runtime, "search");
    assert_visible(&runtime, "toggle-inspector");
    assert_visible(&runtime, "overview");
    runtime.set_system_dark(true);
    settle(&mut runtime);
    assert_eq!(element(&runtime, "graph-canvas").fill, dark_canvas);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|e| e.text.as_deref() == Some("CodeGraph") && e.visible[2] > 10.0)
    );
}

#[test]
fn reduced_motion_pane_collapse_snaps_width_and_keeps_controls_reachable() {
    let mut runtime = narrow_runtime(false);
    runtime.set_system_reduced_motion(true);
    settle(&mut runtime);
    let camera = runtime.state.nav.camera.clone();
    click(&mut runtime, "drawer-repository");
    assert!(!runtime.state.ui.sidebar.is_open());
    assert!(
        element(&runtime, "sidebar-pane").rect[2] < 1.0,
        "Reduced motion should snap pane width instead of sliding it"
    );
    assert_visible(&runtime, "overview");
    assert_visible(&runtime, "toggle-inspector");
    click(&mut runtime, "drawer-search");
    settle(&mut runtime);
    assert!(runtime.state.ui.sidebar.is_open());
    assert_visible(&runtime, "search");
    assert_eq!(runtime.state.nav.camera.pan, camera.pan);
    assert_eq!(runtime.state.nav.camera.zoom, camera.zoom);
}

#[test]
fn empty_workspace_shows_discovery_progress_and_a_real_directory_error() {
    let directory = tempfile::tempdir().unwrap();
    let mut runtime = Runtime::new(
        State::new(),
        view::view,
        (1050.0, 680.0),
        Env::new(true, 1.0),
    );
    settle(&mut runtime);
    assert!(runtime.elements().iter().any(|e| e.text.as_deref()
        == Some("See how your application connects")
        && e.visible[2] > 10.0));
    runtime.update(|state, _| {
        state.root =
            rune::TextEdit::new(directory.path().join("missing-project").to_string_lossy());
        state.discover_root();
    });
    // Render the actual queued discovery's progress before polling its result.
    settle(&mut runtime);
    assert_eq!(
        element(&runtime, "status-message").text.as_deref(),
        Some("Finding solutions, Java builds and TypeScript projects…")
    );
    runtime.poll = Some(jobs::poll);
    super::wait_for(&mut runtime, |state| !state.warning.is_empty());
    settle(&mut runtime);
    let error = element(&runtime, "status-message");
    assert_eq!(error.text.as_deref(), Some(runtime.state.warning.as_str()));
    assert!(error.visible[2] > 10.0 && error.visible[3] > 10.0);
    assert_visible(&runtime, "choose-folder");
    assert_visible(&runtime, "find-builds");
}
