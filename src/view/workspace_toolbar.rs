//! Workspace identity and action-band composition; operations stay in State.
use super::{graph_controls, pane_controls, project_picker, search_modal, workspace_navigation};
use crate::app::State;
use rune::{El, TextStyle, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, Kit, glyphs};

pub(super) fn identity(k: &Kit, titlebar_inset: f32) -> El {
    row()
        .key("workspace-identity-band")
        .w_full()
        .h(42.0)
        .pl((titlebar_inset + 12.0).max(20.0))
        .pr(20.0)
        .gap(12.0)
        .center_y()
        .window_drag()
        .border(1.0, k.c.border_muted)
        .child(
            icon(&glyphs::FILE_CODE, 18.0)
                .color(k.c.text_base)
                .a11y_hidden(),
        )
        .child(
            text("CodeCallNavigator")
                .font(TextStyle::new(15.0, 650.0))
                .color(k.c.text_base),
        )
        .child(el().w(1.0).h(20.0).fill(k.c.border_strong).a11y_hidden())
        .child(text("Open Canvas").font(k.ty.ui).color(k.c.text_muted))
}

pub(super) fn view(state: &State, k: &Kit, layers: &mut Vec<El>) -> El {
    row()
        .key("workspace-action-band")
        .w_full()
        .h(48.0)
        .px(16.0)
        .gap(8.0)
        .center_y()
        .window_drag()
        .fill(k.c.bg_base)
        .border(1.0, k.c.border_muted)
        .child(project_picker::button(state, k))
        .child(pane_controls::toggle(state, k, pane_controls::Side::Files))
        .child(workspace_navigation::view(state, k, layers).grow(1.0))
        .child(search_modal::button(k))
        .child(graph_controls::filters(state, k, layers))
        .child(pane_controls::toggle(
            state,
            k,
            pane_controls::Side::Inspector,
        ))
}

pub(super) fn clear_cache_action(state: &State, k: &Kit) -> El {
    Button::new(if state.clearing_cache {
        "Clearing…"
    } else {
        "Clear cache"
    })
    .disabled(!state.can_clear_cache())
    .on_press(|state: &mut State| {
        state.close_overlay();
        state.clear_cache();
    })
    .build(k)
    .key("clear-cache")
    .help("Clear this project's analysis cache before starting a full analysis")
}

pub(super) fn analysis_action(state: &State, k: &Kit) -> El {
    if state.analyzing {
        Button::new("Cancel analysis")
            .variant(ButtonVariant::Warning)
            .on_press(State::cancel_analysis)
            .build(k)
            .key("cancel-analysis")
    } else {
        Button::new(if state.graph.is_some() {
            "Refresh"
        } else {
            "Generate graph"
        })
        .disabled(
            state.project_job_busy()
                || state.store.is_none()
                || state.targets.iter().all(|t| !t.selected),
        )
        .on_press(|state: &mut State| {
            state.close_overlay();
            state.analyze_selected();
        })
        .build(k)
        .key("analyze")
    }
}
