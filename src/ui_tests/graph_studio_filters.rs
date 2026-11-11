//! Filter dimensions, saved preferences, history and responsive accessibility.
use super::{click, narrow_runtime, runtime, settle};
use crate::{app::State, jobs, scenes, view};
use codecallnavigator::model::{RelationshipKind, TargetResolution};
use rune::{Env, Runtime};

#[test]
fn grouped_filters_toggle_preserving_zoom_and_centering_the_focus() {
    let mut runtime = narrow_runtime(true);
    let before = runtime.state.nav.filter.clone();
    let zoom = runtime.state.nav.camera.zoom;
    for key in [
        "filter-tests",
        "filter-generated",
        "filter-unavailable",
        "filter-virtual-calls",
        "filter-possible",
        "filter-direct",
        "filter-other-resolution",
        "filter-calls",
        "filter-framework",
        "filter-http-navigation",
        "filter-other-relationships",
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
        ],
        [
            !before.include_tests,
            !before.include_generated,
            !before.include_unavailable,
        ]
    );
    assert_eq!(
        (
            after.relationship_kinds.len(),
            after.target_resolutions.len(),
            runtime.state.nav.camera.zoom
        ),
        (0, 0, zoom),
    );
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
fn relationship_type_and_resolution_are_independent_and_restored_by_history() {
    let mut runtime = runtime();
    click(&mut runtime, "filter-references");
    settle(&mut runtime);
    assert!(
        !runtime
            .state
            .nav
            .filter
            .relationship_kinds
            .contains(&RelationshipKind::FunctionReference)
    );
    assert!(
        runtime
            .state
            .nav
            .filter
            .target_resolutions
            .contains(&TargetResolution::VirtualDispatch)
    );
    click(&mut runtime, "filter-virtual-calls");
    settle(&mut runtime);
    assert!(
        !runtime
            .state
            .nav
            .filter
            .target_resolutions
            .contains(&TargetResolution::VirtualDispatch)
    );
    assert!(
        !runtime
            .state
            .nav
            .filter
            .relationship_kinds
            .contains(&RelationshipKind::FunctionReference)
    );
    click(&mut runtime, "back");
    settle(&mut runtime);
    assert!(
        runtime
            .state
            .nav
            .filter
            .target_resolutions
            .contains(&TargetResolution::VirtualDispatch)
    );
    assert!(
        !runtime
            .state
            .nav
            .filter
            .relationship_kinds
            .contains(&RelationshipKind::FunctionReference)
    );
    click(&mut runtime, "forward");
    settle(&mut runtime);
    assert!(
        !runtime
            .state
            .nav
            .filter
            .target_resolutions
            .contains(&TargetResolution::VirtualDispatch)
    );
}

fn assert_filter_bar(runtime: &Runtime<State>) {
    let elements = runtime.elements();
    let canvas = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("graph-filters"))
        .unwrap();
    for heading in ["Code", "Relationship type", "Target resolution"] {
        assert!(elements.iter().any(|e| e.text.as_deref() == Some(heading)));
    }
    let mut previous: Option<[f32; 4]> = None;
    for (key, label) in [
        ("filter-tests", "Test code"),
        ("filter-generated", "Generated code"),
        ("filter-unavailable", "Methods without implementation"),
        ("filter-calls", "Calls"),
        ("filter-references", "Function references"),
        ("filter-framework", "Framework / lifecycle"),
        ("filter-http-navigation", "HTTP / navigation"),
        ("filter-other-relationships", "Other relationships"),
        ("filter-direct", "Directly resolved"),
        ("filter-virtual-calls", "Virtual / interface dispatch"),
        ("filter-possible", "Possible targets"),
        ("filter-other-resolution", "Other / unspecified"),
    ] {
        let control = elements
            .iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap();
        assert_filter_accessibility(control, label);
        assert!(
            control.rect[0] >= canvas.rect[0]
                && control.rect[0] + control.rect[2] <= canvas.rect[0] + canvas.rect[2] + 0.5
                && control.visible[2] >= control.rect[2] - 0.5
                && control.visible[3] >= control.rect[3] - 0.5,
            "{key} exceeds the graph or is clipped: {:?} / {:?}",
            control.rect,
            control.visible
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
    assert_eq!(
        (control.role, control.label.as_deref(), control.focusable),
        ("checkbox", Some(label), true),
        "{label} accessibility"
    );
    assert!(
        control
            .help
            .as_deref()
            .is_some_and(|help| help.len() > label.len()),
        "{label} must explain its meaning"
    );
}

#[test]
fn grouped_filters_fit_and_expose_full_names_in_both_themes() {
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
fn filter_groups_explain_independent_dimensions() {
    let runtime = runtime();
    for heading in ["Code", "Relationship type", "Target resolution"] {
        assert!(
            runtime
                .elements()
                .iter()
                .any(|element| element.text.as_deref() == Some(heading)),
            "Missing group {heading}"
        );
    }
    assert!(runtime.elements().iter().any(|element| {
        element.label.as_deref() == Some("Function references")
            && element
                .help
                .as_deref()
                .is_some_and(|help| help.contains("does not establish invocation"))
    }));
}

#[test]
fn saved_sessions_migrate_legacy_filters_and_preserve_new_empty_selections() {
    let runtime = runtime();
    let mut saved = serde_json::to_value(runtime.state.session()).unwrap();
    saved["navigation"]["filter"] = serde_json::json!({
        "include_tests": true, "include_generated": false, "include_unavailable": true,
        "include_virtual_calls": false, "include_uncertain": false, "include_references": false,
    });
    let mut loaded: crate::navigation::SavedSession = serde_json::from_value(saved).unwrap();
    assert_eq!(
        (
            loaded.navigation.filter.include_tests,
            loaded.navigation.filter.include_generated,
            loaded.navigation.filter.include_unavailable
        ),
        (true, false, true)
    );
    assert_eq!(
        (
            loaded.navigation.filter.relationship_kinds.len(),
            loaded.navigation.filter.target_resolutions.len()
        ),
        (RelationshipKind::ALL.len(), TargetResolution::ALL.len())
    );
    loaded.navigation.filter.relationship_kinds.clear();
    loaded.navigation.filter.target_resolutions.clear();
    let reloaded: crate::navigation::SavedSession =
        serde_json::from_str(&serde_json::to_string(&loaded).unwrap()).unwrap();
    assert_eq!(loaded.navigation.filter, reloaded.navigation.filter);
}
