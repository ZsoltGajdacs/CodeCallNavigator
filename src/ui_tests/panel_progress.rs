//! Real Rune layout and paint instructions prove responsive panel placement and
//! ongoing loader rotation; pure copy/model tests cannot observe these boundaries.
use super::{click, project_explorer::capture};
use crate::{app::State, presentation::Overlay, scenes, view};
use rune::{Env, Runtime};
use std::{fs, path::Path};

const PHASE: &str = "Compiling all Java source sets for Recurring-billing (Maven)…";

fn runtime(size: (f32, f32), dark: bool) -> Runtime<State> {
    let mut state = State::fixture();
    scenes::install(&mut state);
    state.root = rune::TextEdit::new("/Users/example/Projects/Recurring-billing");
    state.recent_projects = vec![state.root.text.clone()];
    // Subminimum captures stress popover fitting, not unsupported docked layouts.
    if size.0 < 1050.0 {
        state.ui.sidebar.set_open(false);
        state.ui.inspector.set_open(false);
    }
    state.analyzing = true;
    state.status = PHASE.into();
    state.warning = "Automatic refresh requires readable recorded input scopes".into();
    let mut runtime = Runtime::new(state, view::view, size, Env::new(dark, 1.0));
    runtime.on_key = Some(view::key);
    runtime.set_live_visible(true);
    settle(&mut runtime);
    runtime
}

fn settle(runtime: &mut Runtime<State>) {
    for _ in 0..60 {
        runtime.frame(1.0 / 60.0);
    }
}

fn element(runtime: &Runtime<State>, key: &str) -> rune::ElementInfo {
    runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("Missing {key}"))
}

fn assert_inside(inner: [f32; 4], outer: [f32; 4], name: &str) {
    assert!(
        inner[0] >= outer[0] - 1.0
            && inner[1] >= outer[1] - 1.0
            && inner[0] + inner[2] <= outer[0] + outer[2] + 1.0
            && inner[1] + inner[3] <= outer[1] + outer[3] + 1.0,
        "{name} outside bounds: {inner:?} / {outer:?}"
    );
}

#[test]
fn active_phase_stays_in_title_bar_while_warnings_and_panels_are_visible() {
    let mut runtime = runtime((1050.0, 680.0), true);
    for overlay in [None, Some(Overlay::Projects), Some(Overlay::Activity)] {
        runtime.update(move |state, _| state.ui.overlay = overlay);
        settle(&mut runtime);
        let phase = element(&runtime, "titlebar-progress");
        assert_eq!(phase.text.as_deref(), Some(PHASE));
        assert_inside(
            phase.rect,
            element(&runtime, "workspace-identity-band").rect,
            "Active phase",
        );
        assert!(phase.visible[2] > 100.0 && phase.visible[3] > 10.0);
        assert_eq!(
            element(&runtime, "activity-title").text.as_deref(),
            Some("Updating graph")
        );
    }
    runtime.update(|state, _| {
        state.analyzing = false;
        state.status = "Analysis cancelled; the previous graph remains available".into();
        state.ui.overlay = None;
    });
    settle(&mut runtime);
    assert_eq!(
        element(&runtime, "activity-title").text.as_deref(),
        Some("Graph needs attention")
    );
}

#[test]
fn graph_controls_center_in_canvas_and_avoid_activity_after_pane_changes() {
    for size in [
        (1800.0, 1000.0),
        (1420.0, 900.0),
        (1050.0, 680.0),
        (600.0, 800.0),
    ] {
        let mut runtime = runtime(size, true);
        let panes = [
            (false, false, 280.0),
            (true, false, 320.0),
            (false, true, 300.0),
            (true, true, 260.0),
        ];
        // The native minimum is 1050 points; subminimum component stress uses
        // at most one docked pane, leaving a meaningful canvas to inspect.
        let cases = if size.0 < 1050.0 {
            &panes[..3]
        } else {
            &panes[..]
        };
        for &(files, inspector, width) in cases {
            runtime.update(move |state, _| {
                state.ui.sidebar.set_open(files);
                state.ui.sidebar.set_width(width);
                state.ui.inspector.set_open(inspector);
                state.ui.inspector.set_width(width);
            });
            settle(&mut runtime);
            let canvas = element(&runtime, "graph-canvas").rect;
            let controls = element(&runtime, "floating-graph-controls").rect;
            let activity = element(&runtime, "status-bar").rect;
            assert!(
                ((controls[0] + controls[2] / 2.0) - (canvas[0] + canvas[2] / 2.0)).abs() <= 1.0,
                "Controls not centered: {controls:?}, canvas {canvas:?}"
            );
            assert_inside(controls, canvas, "Graph controls");
            assert_inside(activity, canvas, "Activity");
            let separate = controls[0] >= activity[0] + activity[2] - 0.5
                || activity[0] >= controls[0] + controls[2] - 0.5
                || controls[1] + controls[3] <= activity[1] + 0.5
                || activity[1] + activity[3] <= controls[1] + 0.5;
            assert!(
                separate,
                "Activity and controls overlap: {activity:?} / {controls:?}"
            );
        }
    }
}

fn turns(runtime: &Runtime<State>) -> Vec<f32> {
    let bounds = element(runtime, "activity-indicator").rect;
    runtime
        .instances
        .iter()
        .filter(|instance| {
            let center = [
                instance.rect[0] + instance.rect[2] / 2.0,
                instance.rect[1] + instance.rect[3] / 2.0,
            ];
            instance.params[1] == rune::render::KIND_GLYPH
                && center[0] > bounds[0]
                && center[0] < bounds[0] + bounds[2]
                && center[1] > bounds[1]
                && center[1] < bounds[1] + bounds[3]
        })
        .map(|instance| instance.params[0])
        .collect()
}

#[test]
fn busy_loader_paint_rotates_across_frames_and_updates_then_stops() {
    let mut runtime = runtime((1050.0, 680.0), true);
    let first = turns(&runtime);
    assert_eq!(first.len(), 2, "Busy ring and arc must both paint");
    runtime.frame(0.15);
    let second = turns(&runtime);
    assert_ne!(
        first, second,
        "Loader painted the same rotation after time advanced"
    );
    runtime.update(|state, _| state.status = "Resolving Java calls…".into());
    runtime.frame(0.15);
    assert_ne!(
        second,
        turns(&runtime),
        "Progress update froze loader paint"
    );
    runtime.set_system_reduced_motion(true);
    settle(&mut runtime);
    let still = turns(&runtime);
    runtime.frame(0.15);
    assert_eq!(
        still,
        turns(&runtime),
        "Reduced motion must stop ring rotation"
    );
    runtime.update(|state, _| state.analyzing = false);
    settle(&mut runtime);
    let indicator = element(&runtime, "activity-indicator");
    assert!(
        !runtime
            .elements()
            .iter()
            .any(|e| e.path.starts_with(&indicator.path) && e.loop_phase.is_some()),
        "Idle indicator retained a busy loop"
    );
}

fn save(runtime: &mut Runtime<State>, size: (f32, f32), output: &Path, name: &str) {
    capture(runtime, size, &output.join(format!("{name}.png")));
    let tree = runtime
        .elements()
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "key": e.key, "text": e.text, "rect": e.rect, "visible": e.visible,
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        output.join(format!("{name}.tree.json")),
        serde_json::to_vec_pretty(&tree).unwrap(),
    )
    .unwrap();
}

fn assert_recent_project_rows(runtime: &Runtime<State>) {
    let panel = element(runtime, "project-picker");
    let elements = runtime.elements();
    let title = elements
        .iter()
        .find(|e| e.path.starts_with(&panel.path) && e.text.as_deref() == Some("Recurring-billing"))
        .expect("Recent-project title");
    let parent = title.path.rsplit_once('/').unwrap().0;
    let path = elements
        .iter()
        .find(|e| {
            e.path.starts_with(parent)
                && e.text.as_deref() == Some(runtime.state.root.text.as_str())
        })
        .expect("Recent-project secondary path");
    for (label, line) in [
        ("Recent-project name", title),
        ("Recent-project path", path),
    ] {
        assert_inside(line.rect, panel.rect, label);
        assert!(
            line.visible[3] >= line.rect[3] - 1.0,
            "{label} vertically clipped: {:?} / {:?}",
            line.rect,
            line.visible
        );
    }
    assert!(
        path.rect[1] >= title.rect[1] + title.rect[3] - 0.5,
        "Recent-project name and secondary path must occupy separate readable lines"
    );
}

fn assert_activity_actions_reachable(runtime: &mut Runtime<State>, size: (f32, f32)) {
    let panel = element(runtime, "activity-popover-content");
    assert_inside(panel.rect, [0.0, 0.0, size.0, size.1], "Analysis details");
    assert!(
        runtime.find("close").is_some(),
        "Details close must remain hit-testable"
    );
    for key in ["freshness-retry", "freshness-auto-toggle"] {
        runtime.update(move |_, tx| tx.scroll_into_view_in("scroll", key));
        settle(runtime);
        let action = element(runtime, key);
        assert_inside(action.rect, panel.rect, key);
        assert!(
            action.visible[2] >= action.rect[2] - 1.0 && action.visible[3] >= action.rect[3] - 1.0,
            "{key} must be fully revealed after scrolling: {:?} / {:?}",
            action.rect,
            action.visible
        );
    }
    assert!(
        runtime.find("freshness-retry").is_some(),
        "Retry action must be hit-testable"
    );
    assert!(
        runtime.find("close").is_some(),
        "Details close must stay reachable after scrolling"
    );
}

/// Risk: Metal clipping/compositing and actual panel typography cannot be checked
/// by cheaper model tests. Explicit presentation inputs do not claim analyzer success.
#[test]
#[ignore = "requires Metal; CODECALLNAVIGATOR_PANEL_UI_CAPTURES optionally retains captures"]
fn native_metal_panel_mockup_and_spinner_captures() {
    let temporary = tempfile::tempdir().unwrap();
    let output = std::env::var_os("CODECALLNAVIGATOR_PANEL_UI_CAPTURES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temporary.path().to_owned());
    fs::create_dir_all(&output).unwrap();
    for (dark, size, name) in [
        (true, (1420.0, 900.0), "dark-1420"),
        (false, (1050.0, 680.0), "light-1050"),
        (true, (600.0, 480.0), "dark-short-600"),
    ] {
        let mut runtime = runtime(size, dark);
        runtime.update(|state, _| {
            state.freshness.status = crate::freshness_workspace::Status::Failed(
                "Automatic refresh requires readable recorded input scopes".into(),
            );
        });
        settle(&mut runtime);
        save(&mut runtime, size, &output, &format!("{name}-busy"));
        runtime.frame(0.15);
        save(
            &mut runtime,
            size,
            &output,
            &format!("{name}-busy-next-frame"),
        );
        click(&mut runtime, "project-picker-trigger");
        settle(&mut runtime);
        let panel = element(&runtime, "project-picker");
        assert_inside(panel.rect, [0.0, 0.0, size.0, size.1], "Project picker");
        assert_recent_project_rows(&runtime);
        save(&mut runtime, size, &output, &format!("{name}-projects"));
        runtime.update(|state, _| state.ui.overlay = None);
        settle(&mut runtime);
        click(&mut runtime, "activity-details");
        settle(&mut runtime);
        assert_activity_actions_reachable(&mut runtime, size);
        save(&mut runtime, size, &output, &format!("{name}-details"));
        click(&mut runtime, "close");
        settle(&mut runtime);
        assert!(runtime.state.ui.overlay.is_none());
        assert!(
            element(&runtime, "activity-details").focused,
            "Closing details must restore keyboard focus to its trigger"
        );
    }
}
