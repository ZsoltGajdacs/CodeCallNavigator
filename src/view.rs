//! Graph Studio shell. Content and application actions live in focused modules.
mod activity;
mod automatic_refresh;
mod build_picker;
mod drawer;
mod explorer;
mod graph_canvas;
mod graph_controls;
pub(crate) mod pane_controls;
mod project_picker;
mod search_modal;
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
        .titlebar_inset(cx.titlebar_inset())
        .hide_sidebar_toggle()
        .frame_inset(0.0)
        .reduced_motion(cx.is_reduced_motion())
        .toolbar(workspace_toolbar::view(state, &k, &mut layer))
        .build(&k)
        .grow(1.0)
        .min_w(0.0);
    let body = col()
        .key("application")
        .on_layout(|s: &mut State, rect| s.source_popup.window = [rect[2], rect[3]])
        .w_full()
        .h_full()
        .fill(k.c.bg_base)
        .child(
            row()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .on_layout(|s: &mut State, rect| {
                    s.constrain_panes(rect[2]);
                })
                .child(split),
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
    if state.ui.overlay == Some(presentation::Overlay::Builds) {
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
    let controls = col()
        .key("floating-graph-controls")
        .radius(10.0)
        .border(1.0, k.c.border_base)
        .fill(k.c.bg_base)
        .child(graph_controls::footer(state, k, layer));
    let activity = activity::view(state, k)
        .max_w(480.0)
        .radius(10.0)
        .border(1.0, k.c.border_base);
    let bottom = row()
        .key("floating-controls")
        .on_layout(|state: &mut State, rect| state.set_canvas_controls_top(rect[1]))
        .gap(12.0)
        .flex_wrap()
        .items_end()
        .child(activity)
        .child(controls)
        .pin(Some(12.0), None, Some(12.0), Some(12.0));
    col()
        .key("canvas-workspace")
        .grow(1.0)
        .min_h(0.0)
        .min_w(0.0)
        .child(graph_canvas::graph_canvas(state, cx, k))
        .child(bottom)
}
