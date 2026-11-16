//! Compact lifecycle feedback; diagnostics stay in an anchored detail surface.
use super::automatic_refresh;
use crate::{app::State, icons, presentation::Overlay};
use rune::{El, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Loader, Placement, Popover, popover};

#[path = "activity_model.rs"]
mod model;
use model::{Input, Tone, Work};

pub(super) fn view(state: &State, k: &Kit, layer: &mut Vec<El>) -> El {
    let summary = presentation(state);
    let trigger = summary_trigger(&summary, k);
    let details = Popover::new("activity-popover")
        .title("Analysis details")
        .trigger(trigger)
        .open(state.ui.overlay == Some(Overlay::Activity))
        .placement(Placement::TopStart)
        .on_close(State::close_overlay)
        .child(popover::fit_scroll(details(state, k)))
        .build(k, layer)
        .grow(1.0)
        .min_w(0.0);
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
                        .font(k.ty.small)
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
    let automatic_enabled = state.query_context.as_ref().is_some_and(|context| {
        state
            .freshness
            .automatic
            .enabled(&context.descriptor.project)
    });
    let automatic = automatic_enabled
        .then(|| state.freshness.automatic.message())
        .flatten();
    let warning = if state.warning.is_empty() && automatic_enabled {
        state.freshness.automatic.details().unwrap_or_default()
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
    if state.reload.job.is_pending() {
        Work::Reload
    } else if state.jobs.layout.is_pending() || state.needs_layout {
        Work::Layout
    } else {
        Work::Idle
    }
}

fn details(state: &State, k: &Kit) -> El {
    let counts = format!(
        "Symbols: {} shown / {} omitted / {} filtered · Links: {} shown / {} inside groups / {} omitted / {} filtered",
        state.graph_view.nodes.len(),
        state.graph_view.omitted_nodes,
        state.graph_view.hidden_nodes,
        state.graph_view.edges.len(),
        state.graph_view.collapsed_edges,
        state.graph_view.omitted_edges,
        state.graph_view.hidden_edges
    );
    let mut body = col()
        .w_full()
        .gap(12.0)
        .child(
            text(&state.status)
                .key("active-analysis")
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap(),
        )
        .child(text(counts).wrap().font(k.ty.small).color(k.c.text_muted))
        .child(super::graph_controls::legend(k));
    if !state.warning.is_empty() {
        body = body.child(
            text(&state.warning)
                .key("activity-warning")
                .font(k.ty.small)
                .color(k.c.warning.fg)
                .wrap(),
        );
    }
    if let Some(notice) = freshness_notice(state, k) {
        body = body.child(notice);
    }
    body
}

pub(super) fn freshness_notice(state: &State, k: &Kit) -> Option<El> {
    let message = state.freshness.message(state.generation.as_deref())?;
    let bar = col().key("freshness-status").w_full().gap(8.0).child(
        text(&message)
            .key("freshness-message")
            .w_full()
            .help(message.clone())
            .font(k.ty.small)
            .color(k.c.text_muted)
            .wrap()
            .min_w(0.0),
    );
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
