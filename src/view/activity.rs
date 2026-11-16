//! Analysis activity, freshness and diagnostic presentation.
use super::automatic_refresh;
use crate::{app::State, presentation::Section, theme};
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Collapsible, Kit, Spinner};
fn active_analysis(state: &State, k: &Kit) -> El {
    let names = state
        .graph
        .as_ref()
        .map(|graph| {
            graph
                .data
                .roots
                .iter()
                .map(|root| {
                    std::path::Path::new(root)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                })
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_default();
    let message = if state.jobs.layout.is_pending() {
        "Updating graph view…".into()
    } else if names.is_empty() {
        state.graph_view.message.clone()
    } else {
        format!("Last successful analysis · {names}")
    };
    text(message)
        .key("active-analysis")
        .w_full()
        .h(24.0)
        .px(theme::SPACE)
        .font(k.ty.small)
        .color(k.c.text_muted)
        .truncate()
}

pub(super) fn view(state: &State, k: &Kit) -> El {
    let status = if !state.warning.is_empty() {
        &state.warning
    } else {
        &state.status
    };
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
    let mut summary = row().w_full().center_y().gap(8.0).child(
        text(status)
            .key("status-message")
            .wrap()
            .max_lines(2)
            .font(k.ty.small)
            .color(if state.warning.is_empty() {
                k.c.text_muted
            } else {
                k.c.warning.fg
            }),
    );
    if updating(state) {
        summary = summary.child(Spinner::new().build(k));
    }
    let mut details = col()
        .w_full()
        .gap(6.0)
        .child(active_analysis(state, k))
        .child(text(counts).wrap().font(k.ty.small).color(k.c.text_muted))
        .child(super::graph_controls::legend(k));
    if let Some(notice) = freshness_notice(state, k) {
        details = details.child(notice);
    }
    let mut bar = col()
        .key("status-bar")
        .w_full()
        .padding(10.0)
        .gap(4.0)
        .fill(k.c.bg_base)
        .child(summary)
        .child(
            Collapsible::new("Details", state.ui.section_open(Section::Activity))
                .on_toggle(|state: &mut State| state.toggle_section(Section::Activity))
                .child(details)
                .build(k)
                .key("activity-details"),
        );
    if state.analyzing {
        bar = bar.child(
            Button::new("Cancel analysis")
                .variant(ButtonVariant::Warning)
                .on_press(State::cancel_analysis)
                .build(k)
                .key("activity-cancel"),
        );
    }
    bar
}

pub(super) fn freshness_notice(state: &State, k: &Kit) -> Option<El> {
    let message = state.freshness.message(state.generation.as_deref())?;
    let mut bar = row()
        .key("freshness-status")
        .w_full()
        .gap(8.0)
        .center_y()
        .flex_wrap()
        .child(
            text(&message)
                .key("freshness-message")
                .max_lines(2)
                .truncate()
                .help(message.clone())
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap()
                .grow(1.0)
                .min_w(0.0),
        );
    if state
        .freshness
        .new_generation(state.generation.as_deref())
        .is_some()
    {
        bar = bar.child(
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
        bar = bar.child(
            Button::new("Retry freshness")
                .on_press(State::retry_freshness)
                .build(k)
                .key("freshness-retry"),
        );
    }
    Some(automatic_refresh::append(state, k, bar))
}

fn updating(state: &State) -> bool {
    state.analyzing || state.jobs.layout.is_pending() || state.reload.job.is_pending()
}
