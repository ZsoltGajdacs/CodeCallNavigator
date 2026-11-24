//! Graph Studio shell. Content and application actions live in focused modules.
mod activity;
pub(crate) use activity::poll_stage_clock;
mod activity_details;
mod analysis_failure;
mod automatic_refresh;
mod build_picker;
mod drawer;
mod explorer;
mod freshness_diagnostics;
mod graph_canvas;
mod graph_controls;
pub(crate) use graph_controls::{CameraAction, CameraControls, camera_controls, depth_select};
pub(crate) mod pane_controls;
mod project_picker;
mod search_modal;
mod workspace_navigation;
mod workspace_toolbar;

use crate::{app::State, inspector, presentation, theme};
use rune::{Cx, El, Key, KeyEvent, Tx, col, row};
use rune_kit::{Button, ButtonVariant, Kit, shell::SplitView};

pub fn view(state: &State, cx: &mut Cx<State>) -> El {
    if state.flow_comparison.controls.visible {
        return crate::flow_comparison_view::view(state, cx);
    }
    if state.flow_workspace.current.is_some() {
        return crate::flow_view::view(state, cx);
    }
    graph_workspace(state, cx)
}

fn graph_workspace(state: &State, cx: &mut Cx<State>) -> El {
    let k = theme::kit(cx.system_dark());
    let mut layer = Vec::new();
    let main = col()
        .w_full()
        .h_full()
        .min_w(0.0)
        .min_h(0.0)
        .child(graph_controls::toolbar(state, &k))
        .child(canvas_workspace(state, cx, &k, &mut layer));
    // The workspace is edge-to-edge, not an inset rounded card. A second rounded
    // main-pane mask would intersect every graph card's own rounded clip and
    // force an offscreen compositing layer per node in Rune.
    let mut shell_kit = k.clone();
    shell_kit.radius.lg = 0.0;
    let split = SplitView::new(main)
        .sidebar(
            &state.ui.sidebar,
            presentation::sidebar,
            drawer::view(state, &k),
        )
        .inspector(
            &state.ui.inspector,
            presentation::inspector,
            inspector::view(state, &k),
        )
        .titlebar_inset(0.0)
        .hide_sidebar_toggle()
        .frame_inset(0.0)
        .reduced_motion(cx.is_reduced_motion())
        .build(&shell_kit)
        .grow(1.0)
        .min_w(0.0);
    let actions = workspace_toolbar::view(state, &k, &mut layer);
    let body = col()
        .key("application")
        .on_layout(|s: &mut State, rect| s.source_popup.interaction.window = [rect[2], rect[3]])
        .w_full()
        .h_full()
        .fill(k.c.bg_base)
        .child(workspace_toolbar::identity(state, &k, cx.titlebar_inset()))
        .child(
            rune::el()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .on_layout(|s: &mut State, rect| {
                    s.constrain_panes(rect[2]);
                })
                .child(split)
                .child(actions.absolute(0.0, 0.0)),
        );
    with_overlays(state, &k, body, layer)
}

pub(crate) fn comparison_button(k: &Kit) -> El {
    Button::new("Compare flows")
        .variant(ButtonVariant::Outline)
        .on_press(State::show_flow_comparison)
        .build(k)
        .key("compare-flows")
}

pub(crate) fn search_button(k: &Kit) -> El {
    search_modal::button(k)
}

pub(crate) fn with_overlays(state: &State, k: &Kit, body: El, mut layer: Vec<El>) -> El {
    if let Some(popup) = crate::source_popup::view(state, k) {
        layer.push(popup);
    }
    build_picker::layer(state, k, &mut layer);
    project_picker::layer(state, k, &mut layer);
    search_modal::layer(state, k, &mut layer);
    rune_kit::root(body, layer)
}

pub fn key(state: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    if matches!(
        state.ui.overlay,
        Some(presentation::Overlay::Builds | presentation::Overlay::ConfirmAnalysis(..))
    ) {
        return false;
    }
    if crate::search::key(state, event, tx) {
        return true;
    }
    if state.search.open {
        return false;
    }
    if state.flow_comparison.controls.visible {
        return crate::flow_comparison_view::key(state, event);
    }
    if state.flow_workspace.current.is_some() {
        return crate::flow_view::key(state, event);
    }
    graph_key(state, event, tx)
}

fn graph_key(state: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    if event.is_command('o') {
        state.choose_folder(tx);
        return true;
    }
    if rune_kit::shell::is_sidebar_toggle(event) {
        state.toggle_drawer(state.ui.drawer);
        return true;
    }
    match &event.key {
        Key::Left if event.mods.alt => state.history(false),
        Key::Right if event.mods.alt => state.history(true),
        Key::Escape => dismiss(state),
        _ => return false,
    }
    true
}

fn dismiss(state: &mut State) {
    if state.source_popup.active.is_some() {
        state.close_source();
    } else if state.ui.overlay.is_some() {
        state.close_overlay();
    } else {
        state.overview();
    }
}

pub(crate) fn freshness_notice(state: &State, k: &Kit) -> Option<El> {
    activity::freshness_notice(state, k)
}

fn canvas_workspace(state: &State, cx: &mut Cx<State>, k: &Kit, layer: &mut Vec<El>) -> El {
    // Separate bottom surfaces wrap vertically when the available canvas is narrow.
    let controls = theme::floating_surface(k)
        .key("floating-graph-controls")
        .child(graph_controls::footer(state, k, layer));
    let activity = theme::floating_surface(k)
        .child(activity::view(state, k, layer))
        .max_w(380.0);
    // Reserve a symmetric lane for the left activity card so it cannot displace
    // the centered controls. Narrow canvases use two independent rows instead.
    let centered = row().w_full().justify_center().child(controls);
    let bottom = if state.viewport[0] < 1440.0 {
        col().gap(12.0).child(centered).child(row().child(activity))
    } else {
        row()
            .min_h(64.0)
            .items_end()
            .child(centered)
            .child(activity.pin(Some(0.0), None, None, Some(0.0)))
    }
    .key("floating-controls")
    .on_layout(|state: &mut State, rect| state.set_canvas_controls_top(rect[1]))
    .pin(Some(12.0), None, Some(12.0), Some(12.0));
    col()
        .key("canvas-workspace")
        .grow(1.0)
        .min_h(0.0)
        .min_w(0.0)
        .child(graph_canvas::graph_canvas(state, cx, k))
        .child(bottom)
}
