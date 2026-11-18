//! Compact lifecycle feedback; diagnostics stay in an anchored detail surface.
use super::automatic_refresh;
use crate::{app::State, icons, presentation::Overlay};
use rune::{El, col, el, icon, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Loader, Placement,
    popover,
};

#[path = "activity_model.rs"]
mod model;
use model::{Input, Tone, Work};

pub(super) fn view(state: &State, k: &Kit, layer: &mut Vec<El>) -> El {
    let summary = presentation(state);
    let trigger =
        summary_trigger(&summary, k).expanded(state.ui.overlay == Some(Overlay::Activity));
    if state.ui.overlay == Some(Overlay::Activity) {
        details_layer(state, k, layer, &summary);
    }
    let details = row()
        .key("activity-popover-trigger")
        .grow(1.0)
        .min_w(0.0)
        .child(trigger);
    let mut bar = row()
        .key("status-bar")
        .w(360.0_f32.min((state.viewport[0] - 24.0).max(1.0)))
        .min_w(0.0)
        .padding(10.0)
        .gap(10.0)
        .center_y()
        .child(details);
    if state.analyzing || state.freshness.manual_pending() {
        bar = bar
            .child(el().w(1.0).h(30.0).fill(k.c.border_base).a11y_hidden())
            .child(
                Button::new("Cancel")
                    .variant(ButtonVariant::Outline)
                    .on_press(State::cancel_analysis)
                    .build(k)
                    .key("activity-cancel")
                    .label("Cancel analysis"),
            );
    }
    bar
}

fn summary_trigger(summary: &model::Presentation, k: &Kit) -> El {
    row()
        .key("activity-details")
        .center_y()
        .gap(10.0)
        .grow(1.0)
        .min_w(0.0)
        .role("button")
        .label("Analysis activity details")
        .help("Show analysis progress, graph counts, freshness and diagnostics")
        .focusable(true)
        .on_press(|state: &mut State| {
            state.ui.overlay = if state.ui.overlay == Some(Overlay::Activity) {
                None
            } else {
                Some(Overlay::Activity)
            };
        })
        .child(indicator(summary, k))
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(1.0)
                .child(
                    text(summary.title)
                        .key("activity-title")
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base)
                        .truncate(),
                )
                .child(
                    text(&summary.detail)
                        .key("status-message")
                        .font(k.ty.ui)
                        .color(if summary.tone == Tone::Attention {
                            k.c.warning.fg
                        } else {
                            k.c.text_muted
                        })
                        .help(&summary.detail)
                        .truncate(),
                ),
        )
}

fn indicator(summary: &model::Presentation, k: &Kit) -> El {
    if summary.busy {
        Loader::new()
            .size(28.0)
            .color(k.c.text_accent)
            .build(k)
            .key("activity-indicator")
    } else {
        icon(&icons::REFRESH, 24.0)
            .key("activity-indicator")
            .color(if summary.tone == Tone::Attention {
                k.c.warning.fg
            } else {
                k.c.text_accent
            })
            .a11y_hidden()
    }
}

fn presentation(state: &State) -> model::Presentation {
    let freshness = state.freshness.message(state.generation.as_deref());
    let automatic_enabled = state.automatic_refresh_enabled();
    let automatic = automatic_enabled
        .then(|| state.freshness.automatic.message())
        .flatten();
    let automatic_warning = automatic_enabled
        .then(|| state.freshness.automatic.details())
        .flatten();
    let analysis_warning = state.feedback().map(|failure| failure.summary());
    let warning = if state.warning.is_empty() {
        analysis_warning
            .as_deref()
            .or(automatic_warning.as_deref())
            .unwrap_or_default()
    } else {
        &state.warning
    };
    let work = work(state, automatic_enabled);
    let status = match work {
        Work::Layout => "Arranging graph view…",
        Work::Reload => "Loading the saved graph…",
        Work::Automatic => automatic.as_deref().unwrap_or(&state.status),
        _ => &state.status,
    };
    model::present(Input {
        work,
        has_graph: state.graph.is_some(),
        warning,
        status,
        freshness: &state.freshness.status,
        freshness_message: freshness.as_deref(),
        newer_graph: state
            .freshness
            .new_generation(state.generation.as_deref())
            .is_some(),
    })
}

fn work(state: &State, automatic_enabled: bool) -> Work {
    if state.jobs.discovery.is_pending() {
        Work::Discovery
    } else if state.analyzing || state.freshness.manual_pending() {
        Work::Analysis
    } else if graph_work(state) != Work::Idle {
        graph_work(state)
    } else if automatic_enabled && state.freshness.automatic.can_cancel() {
        Work::Automatic
    } else {
        Work::Idle
    }
}

fn graph_work(state: &State) -> Work {
    if state.project_open.loading() || state.reload.job.is_pending() {
        Work::Reload
    } else if state.jobs.layout.is_pending() || state.needs_layout {
        Work::Layout
    } else {
        Work::Idle
    }
}

fn details_layer(state: &State, k: &Kit, layers: &mut Vec<El>, summary: &model::Presentation) {
    let close = IconButton::new(k.glyph.close)
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Regular)
        .label("Close")
        .on_press(State::close_overlay)
        .build(k)
        .key("close");
    let body =
        super::activity_details::view(state, k, summary.phase.as_deref().unwrap_or(&state.status));
    let content = col()
        .key("activity-popover-content")
        .role("dialog")
        .label("Analysis details")
        .focus_return("activity-details")
        .modal(true, State::close_overlay)
        .w(480.0_f32.min((state.source_popup.window[0] - 24.0).max(1.0)))
        .min_h(0.0)
        .child(
            row()
                .key("header")
                .padding(16.0)
                .pb(0.0)
                .gap(8.0)
                .center_y()
                .child(
                    text("Analysis details")
                        .font(k.ty.body_medium)
                        .color(k.c.text_base)
                        .grow(1.0),
                )
                .child(close),
        )
        .child(popover::fit_scroll(body.padding(16.0)));
    layers.push(rune_kit::scrim(
        "activity-popover-scrim",
        State::close_overlay,
    ));
    layers.push(popover::anchored(
        k,
        popover::raised(k, content, k.radius.md),
        "activity-popover-trigger",
        Placement::TopStart,
        8.0,
    ));
}

/// Actual work phase is independent of idle warning/freshness emphasis.
pub(super) fn titlebar(state: &State, k: &Kit) -> El {
    let summary = presentation(state);
    let Some(phase) = summary.phase else {
        return el().grow(1.0);
    };
    row()
        .key("titlebar-activity")
        .grow(1.0)
        .min_w(0.0)
        .center_y()
        .gap(10.0)
        .child(el().w(1.0).h(20.0).fill(k.c.border_strong).a11y_hidden())
        .child(Loader::new().size(18.0).color(k.c.text_accent).build(k))
        .child(
            text(&phase)
                .key("titlebar-progress")
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .grow(1.0)
                .min_w(0.0)
                .truncate()
                .help(&phase),
        )
}

pub(super) fn freshness_notice(state: &State, k: &Kit) -> Option<El> {
    let mut bar = col().key("freshness-status").w_full().gap(8.0);
    if let Some(message) = state.freshness.message(state.generation.as_deref()) {
        bar = bar.child(
            text(&message)
                .key("freshness-message")
                .w_full()
                .help(message.clone())
                .font(k.ty.ui)
                .color(k.c.text_muted)
                .wrap()
                .min_w(0.0),
        );
    } else if !state.can_toggle_automatic_refresh() {
        return None;
    }
    let mut actions = row().w_full().gap(8.0).center_y().flex_wrap();
    if state
        .freshness
        .new_generation(state.generation.as_deref())
        .is_some()
    {
        actions = actions.child(
            Button::new(if state.reload.job.is_pending() {
                "Loading saved graph…"
            } else {
                "Reload saved graph"
            })
            .disabled(state.project_job_busy())
            .on_press(State::reload_generation)
            .build(k)
            .key("freshness-reload"),
        );
    }
    if state.freshness.retry_available() {
        actions = actions.child(
            Button::new("Retry freshness")
                .on_press(State::retry_freshness)
                .build(k)
                .key("freshness-retry"),
        );
    }
    Some(automatic_refresh::append(state, k, bar, actions))
}
