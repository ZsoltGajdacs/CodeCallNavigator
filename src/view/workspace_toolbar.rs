//! Workspace identity and actions; graph navigation stays in graph_controls.
use super::{graph_controls, pane_controls, project_picker, search_modal};
use crate::app::State;
use rune::{El, text};
use rune_kit::{Button, ButtonVariant, Kit, shell::Toolbar};

pub(super) fn view(state: &State, k: &Kit, layer: &mut Vec<El>) -> Toolbar {
    Toolbar::new()
        .leading(
            text("CodeCallNavigator")
                .font(k.ty.ui_medium)
                .color(k.c.text_base),
        )
        .leading(project_picker::button(state, k))
        .leading(pane_controls::toggle(state, k, pane_controls::Side::Files))
        .trailing(search_modal::button(k))
        .trailing(graph_controls::filters(state, k, layer))
        .trailing(pane_controls::toggle(
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
