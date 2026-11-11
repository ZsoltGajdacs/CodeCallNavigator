//! Modal search through the production view, controller, index, and Rune input runtime.
use super::{click, wait_for};
use crate::{
    app::State,
    flow_scenes, jobs,
    projection::{FocusTarget, GroupKind, Grouping, ViewNode},
    scenes, search, view,
};
use codecallnavigator::{
    graph::GraphIndex,
    search::{SearchCategory, SearchTarget},
};
use rune::{Cx, El, Env, Key, KeyEvent, Modifier, Mods, Runtime};
use std::sync::Arc;

fn focus_view(state: &State, cx: &mut Cx<State>) -> El {
    view::view(state, cx).on_window_focus(search::focus_changed)
}

fn runtime(flow: bool) -> Runtime<State> {
    let mut state = State::new();
    if flow {
        flow_scenes::install(&mut state);
    } else {
        scenes::install(&mut state);
    }
    let mut runtime = Runtime::new(state, focus_view, (1050.0, 680.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    runtime.on_key = Some(view::key);
    runtime.on_key_activity = Some(search::key_activity);
    runtime.on_pointer_activity = Some(search::pointer_activity);
    runtime.set_modifier_keys(true);
    runtime.frame(0.0);
    runtime
}

fn press(runtime: &mut Runtime<State>, key: Key) {
    runtime.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    runtime.frame(1.0 / 60.0);
}

fn query(runtime: &mut Runtime<State>, text: &str) {
    runtime.update(|state, _| {
        state.search.input = rune::TextEdit::new(text);
        state.search.input.focused = true;
        state.request_search();
    });
    wait_for(runtime, |state| {
        matches!(state.search.results, search::Results::Ready(_))
    });
}

fn shift_tap(runtime: &mut Runtime<State>, side: Modifier) {
    for pressed in [true, false] {
        runtime.key(KeyEvent {
            key: Key::Modifier {
                modifier: side,
                pressed,
            },
            mods: Mods {
                shift: pressed,
                ..Default::default()
            },
        });
        runtime.frame(0.025);
    }
}

fn selected_node(runtime: &mut Runtime<State>) -> ViewNode {
    wait_for(runtime, |state| {
        !state.needs_layout && !state.graph_interaction.center_after_layout
    });
    let selected = runtime
        .state
        .visual_selection
        .as_ref()
        .expect("selected graph node");
    let node = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|node| &node.key == selected)
        .expect("selected node is visible");
    let center = runtime.state.nav.camera.screen([
        node.position[0] + node.size[0] / 2.0,
        node.position[1] + node.size[1] / 2.0,
    ]);
    assert!(
        (center[0] - runtime.state.viewport[0] / 2.0).abs() < 0.1,
        "selected node centered horizontally: {center:?}"
    );
    assert!(
        (center[1] - runtime.state.viewport[1] / 2.0).abs() < 0.1,
        "selected node centered vertically: {center:?}"
    );
    node.clone()
}

fn selected_class(runtime: &mut Runtime<State>) -> ViewNode {
    let node = selected_node(runtime);
    assert_eq!(
        (node.group_kind, node.member),
        (Some(GroupKind::Class), None),
        "class search selects the class card"
    );
    assert!(
        matches!(&runtime.state.nav.focus_target, Some(FocusTarget::Group(key)) if key == &node.key)
    );
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|node| { node.group_kind == Some(GroupKind::Class) && node.member.is_none() }),
        "class search keeps connections at class level rather than drilling into methods"
    );
    node
}

#[test]
fn double_shift_opens_and_pointer_or_focus_interruption_cancels_it() {
    let mut runtime = runtime(false);
    shift_tap(&mut runtime, Modifier::ShiftLeft);
    shift_tap(&mut runtime, Modifier::ShiftRight);
    assert!(runtime.state.search.open);
    press(&mut runtime, Key::Escape);
    assert!(!runtime.state.search.open);
    shift_tap(&mut runtime, Modifier::ShiftLeft);
    runtime.pointer_move(100.0, 100.0);
    runtime.frame(0.01);
    shift_tap(&mut runtime, Modifier::ShiftLeft);
    assert!(!runtime.state.search.open);
    runtime.window_focus(false);
    runtime.frame(0.01);
    runtime.window_focus(true);
    runtime.frame(0.01);
    shift_tap(&mut runtime, Modifier::ShiftLeft);
    assert!(!runtime.state.search.open);
}

#[test]
fn modal_focuses_real_input_and_restores_query_category_and_keyboard_selection() {
    let mut runtime = runtime(false);
    click(&mut runtime, "open-search");
    let input = runtime.elements().into_iter().find(|element| {
        element.key.as_deref() == Some("input")
            && element.label.as_deref() == Some("Search classes and methods")
    });
    assert!(input.is_some_and(|input| input.focused));
    click(&mut runtime, "search-methods");
    query(&mut runtime, "order");
    assert_eq!(runtime.state.search.hits().len(), 6);
    assert_eq!(runtime.state.search.selected, 0);
    press(&mut runtime, Key::Down);
    assert_eq!(runtime.state.search.selected, 1);
    press(&mut runtime, Key::Up);
    assert_eq!(runtime.state.search.selected, 0);
    click(&mut runtime, "close-search");
    click(&mut runtime, "open-search");
    assert_eq!(
        (
            runtime.state.search.category,
            runtime.state.search.input.text.as_str(),
            runtime.state.search.input.anchor,
            runtime.state.search.input.focus
        ),
        (SearchCategory::Methods, "order", 0, 5),
        "reopening retains the category and selects the query",
    );
    press(&mut runtime, Key::Enter);
    assert!(!runtime.state.search.open);
    assert!(matches!(
        runtime.state.nav.focus_target,
        Some(FocusTarget::Method(_))
    ));
}

#[test]
fn tabs_return_classes_and_methods_and_class_activation_reveals_members_once() {
    let mut runtime = runtime(false);
    click(&mut runtime, "open-search");
    query(&mut runtime, "Orders");
    assert_eq!(runtime.state.search.hits().len(), 7);
    click(&mut runtime, "search-classes");
    wait_for(&mut runtime, |state| state.search.hits().len() == 1);
    assert!(
        matches!(&runtime.state.search.hits()[0].target, SearchTarget::Class {members, ..} if members.len() == 6)
    );
    let back = runtime.state.back.len();
    press(&mut runtime, Key::Enter);
    assert_eq!(
        (
            runtime.state.search.open,
            runtime.state.nav.filter.include_tests,
            runtime.state.nav.filter.include_generated
        ),
        (false, true, true)
    );
    selected_class(&mut runtime);
    assert_eq!(runtime.state.back.len(), back + 1);
}

#[test]
fn class_activation_from_layer_grouping_uses_existing_class_keys() {
    let mut runtime = runtime(false);
    runtime.update(|state, _| state.nav.grouping = Grouping::Layer);
    click(&mut runtime, "open-search");
    click(&mut runtime, "search-classes");
    query(&mut runtime, "Orders");
    press(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.nav.grouping, Grouping::Project);
    assert!(
        matches!(runtime.state.nav.focus_target, Some(FocusTarget::Group(ref key)) if key.starts_with("[\"class\","))
    );
}

#[test]
fn modal_blocks_background_shortcuts_and_dismisses_on_outside_click() {
    let mut runtime = runtime(false);
    click(&mut runtime, "open-search");
    let nav = serde_json::to_value(&runtime.state.nav).unwrap();
    runtime.key(KeyEvent {
        key: Key::Command('o'),
        mods: Mods {
            cmd: true,
            logo: true,
            ..Default::default()
        },
    });
    runtime.frame(1.0 / 60.0);
    assert!(runtime.file_dialog().is_none());
    assert_eq!(serde_json::to_value(&runtime.state.nav).unwrap(), nav);
    runtime.pointer_down(10.0, 650.0);
    runtime.pointer_up(10.0, 650.0);
    runtime.frame(1.0 / 60.0);
    assert!(!runtime.state.search.open);
    assert_eq!(serde_json::to_value(&runtime.state.nav).unwrap(), nav);
}

#[test]
fn search_is_available_above_flowchart_and_activation_returns_to_graph() {
    let mut runtime = runtime(true);
    assert!(runtime.state.flow_workspace.current.is_some());
    click(&mut runtime, "open-search");
    assert!(runtime.find("search-dialog").is_some());
    click(&mut runtime, "search-methods");
    query(&mut runtime, "save");
    press(&mut runtime, Key::Enter);
    assert!(!runtime.state.search.open);
    assert!(runtime.state.flow_workspace.current.is_none());
    assert_eq!(runtime.state.nav.focus.as_deref(), Some("flow/save"));
}

#[test]
fn replacement_graph_and_query_discard_old_results_before_activation() {
    let mut runtime = runtime(false);
    let original_focus = runtime.state.nav.focus.clone();
    click(&mut runtime, "open-search");
    click(&mut runtime, "search-methods");
    query(&mut runtime, "LoadOrder");
    runtime.update(|state, _| {
        let mut graph = state.graph.as_ref().unwrap().data.as_ref().clone();
        graph.nodes[1].id = "new/Replacement".into();
        graph.nodes[1].label = "Replacement".into();
        graph.nodes[1].signature = "Orders:Replacement()".into();
        graph.entry_points = None;
        state.graph = Some(Arc::new(GraphIndex::new(Arc::new(graph)).unwrap()));
        // Activation can race the next poll, so stale rows must already be inert.
        state.activate_search(0);
        assert!(state.search.open);
        state.search.input.text = "Replacement".into();
    });
    wait_for(&mut runtime, |state| {
        state
            .search
            .hits()
            .first()
            .is_some_and(|hit| hit.name == "Replacement")
    });
    assert_eq!(runtime.state.search.hits().len(), 1);
    runtime.update(|state, _| {
        state.search.input.text = "NoMatchingName".into();
        state.activate_search(0);
        assert!(state.search.open);
    });
    wait_for(
        &mut runtime,
        |state| matches!(&state.search.results, search::Results::Ready(page) if page.hits.is_empty()),
    );
    assert_eq!(runtime.state.nav.focus, original_focus);
}

#[test]
fn typing_searches_the_index_and_escape_restores_the_previous_editor_focus() {
    let mut runtime = runtime(false);
    click(&mut runtime, "root-path");
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.key.as_deref() == Some("input")
                && element.label.as_deref() == Some("Project folder")
                && element.focused),
        "project editor focused before opening"
    );
    runtime.key(KeyEvent {
        key: Key::Command('f'),
        mods: Mods {
            cmd: true,
            logo: true,
            ..Default::default()
        },
    });
    runtime.frame(1.0 / 60.0);
    press(&mut runtime, Key::Text("ValidateOrder".into()));
    wait_for(&mut runtime, |state| !state.search.hits().is_empty());
    assert_eq!(runtime.state.search.input.text, "ValidateOrder");
    assert_eq!(runtime.state.search.hits().len(), 1);
    assert_eq!(runtime.state.search.hits()[0].name, "ValidateOrder");
    press(&mut runtime, Key::Escape);
    assert!(!runtime.state.search.open);
    assert!(runtime.elements().iter().any(|element| {
        element.key.as_deref() == Some("input")
            && element.label.as_deref() == Some("Project folder")
            && element.focused
    }));
}

#[test]
fn method_activation_leaves_layer_tab_and_preserves_navigation_history() {
    let mut runtime = runtime(false);
    runtime.update(|state, _| {
        state.nav.grouping = Grouping::Layer;
        state.nav.layer_tab = Some("Unrelated layer".into());
    });
    click(&mut runtime, "open-search");
    click(&mut runtime, "search-methods");
    query(&mut runtime, "OrderTest");
    let back = runtime.state.back.len();
    press(&mut runtime, Key::Enter);
    assert_eq!(
        (
            runtime.state.nav.grouping,
            runtime.state.nav.focus.as_deref(),
            runtime.state.nav.filter.include_tests,
            runtime.state.back.len()
        ),
        (
            Grouping::Method,
            Some("fixture/project/OrderTest"),
            true,
            back + 1
        ),
        "method activation changes level, reveals the method, and creates one history entry",
    );
    runtime.update(|state, _| state.history(false));
    assert_eq!(runtime.state.nav.grouping, Grouping::Layer);
    assert_eq!(
        runtime.state.nav.layer_tab.as_deref(),
        Some("Unrelated layer")
    );
}

#[test]
fn modal_is_centered_and_bounded_at_minimum_size_in_both_themes() {
    let mut runtime = runtime(false);
    for dark in [true, false] {
        runtime.set_system_dark(dark);
        if !runtime.state.search.open {
            click(&mut runtime, "open-search");
        }
        runtime.frame(1.0 / 60.0);
        let panel = runtime
            .elements()
            .into_iter()
            .find(|element| element.key.as_deref() == Some("search-dialog"))
            .unwrap()
            .rect;
        assert!(
            (panel[0] + panel[2] / 2.0 - 525.0).abs() < 1.0,
            "modal must be horizontally centered: {panel:?}"
        );
        assert!(panel[0] >= 24.0 && panel[0] + panel[2] <= 1026.0);
        assert!(panel[1] >= 24.0 && panel[1] + panel[3] <= 656.0);
    }
}

#[test]
fn choosing_category_keeps_typing_in_the_search_editor() {
    let mut runtime = runtime(false);
    click(&mut runtime, "open-search");
    click(&mut runtime, "search-methods");
    press(&mut runtime, Key::Text("OrderTest".into()));
    assert_eq!(runtime.state.search.input.text, "OrderTest");
    wait_for(&mut runtime, |state| !state.search.hits().is_empty());
    assert_eq!(runtime.state.search.hits().len(), 1);
    assert_eq!(runtime.state.search.hits()[0].name, "OrderTest");
}

#[test]
fn class_then_method_search_selects_and_centers_the_requested_node_level() {
    let mut runtime = runtime(false);
    runtime.update(|state, _| state.nav.grouping = Grouping::Method);
    click(&mut runtime, "open-search");
    click(&mut runtime, "search-classes");
    query(&mut runtime, "Orders");
    let initial_back = runtime.state.back.len();
    press(&mut runtime, Key::Enter);
    selected_class(&mut runtime);
    assert_eq!(runtime.state.back.len(), initial_back + 1);
    click(&mut runtime, "open-search");
    click(&mut runtime, "search-methods");
    query(&mut runtime, "OrderTest");
    press(&mut runtime, Key::Enter);
    assert_eq!(runtime.state.nav.grouping, Grouping::Method);
    assert_eq!(runtime.state.selected(), Some(5));
    let method = selected_node(&mut runtime);
    assert_eq!(
        (
            method.member,
            method.group_kind,
            runtime.state.nav.selected.as_deref()
        ),
        (Some(5), None, Some(method.key.as_str()))
    );
    assert_eq!(runtime.state.back.len(), initial_back + 2);
}
