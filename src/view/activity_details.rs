//! Readable analysis diagnostics; lifecycle and freshness actions keep their owners.
use crate::app::State;
use rune::{El, col, el, row, text};
use rune_kit::Kit;

pub(super) fn view(
    state: &State,
    k: &Kit,
    phase: &str,
    elapsed: Option<std::time::Duration>,
) -> El {
    let mut body = col()
        .w_full()
        .min_w(0.0)
        .gap(12.0)
        .child(heading("Current activity", k))
        .child(
            text(phase)
                .key("active-analysis")
                .w_full()
                .font(k.ty.ui)
                .color(k.c.text_base)
                .wrap_anywhere(),
        );
    if let Some(elapsed) = elapsed {
        let seconds = elapsed.as_secs();
        body = body.child(
            text(format!(
                "Stage shown for {}:{:02}",
                seconds / 60,
                seconds % 60
            ))
            .key("analysis-stage-elapsed")
            .font(k.ty.ui)
            .color(k.c.text_muted),
        );
    }
    body = body
        .child(divider(k))
        .child(summary(state, k))
        .child(divider(k))
        .child(heading("Relationship legend", k))
        .child(super::graph_controls::legend(k));
    if let Some(failure) = super::analysis_failure::details(state, k) {
        body = body
            .child(divider(k))
            .child(heading("Analysis failure", k))
            .child(failure);
    }
    let notice = super::activity::freshness_notice(state, k);
    if !state.warning.is_empty() || notice.is_some() {
        body = body.child(divider(k)).child(heading("Freshness", k));
        if !state.warning.is_empty() {
            body = body.child(
                text(&state.warning)
                    .key("activity-warning")
                    .w_full()
                    .font(k.ty.ui)
                    .color(k.c.warning.fg)
                    .wrap_anywhere(),
            );
        }
        if let Some(notice) = notice {
            body = body.child(notice);
        }
    }
    body
}

fn summary(state: &State, k: &Kit) -> El {
    let graph = &state.graph_view;
    let body = col()
        .key("activity-graph-summary")
        .w_full()
        .gap(6.0)
        .child(heading("Graph summary", k))
        .child(count_row(
            "Symbols",
            format!(
                "{} shown · {} omitted · {} filtered",
                graph.nodes.len(),
                graph.omitted_nodes,
                graph.hidden_nodes
            ),
            k,
        ))
        .child(count_row(
            "Links",
            format!(
                "{} shown · {} inside groups · {} omitted · {} filtered",
                graph.edges.len(),
                graph.collapsed_edges,
                graph.omitted_edges,
                graph.hidden_edges
            ),
            k,
        ));
    if let Some(retention) = state
        .analysis_feedback
        .retention
        .as_ref()
        .or_else(|| state.freshness.retention(state.generation.as_deref()))
    {
        return body.child(
            text(crate::analysis_feedback::retention_message(retention))
                .key("analysis-graph-retention")
                .w_full()
                .font(k.ty.ui)
                .color(k.c.text_base)
                .wrap_anywhere(),
        );
    }
    body
}

fn count_row(label: &str, counts: String, k: &Kit) -> El {
    row()
        .w_full()
        .min_w(0.0)
        .gap(12.0)
        .child(
            text(label)
                .w(64.0)
                .font(k.ty.ui_medium)
                .color(k.c.text_base),
        )
        .child(
            text(counts)
                .grow(1.0)
                .min_w(0.0)
                .font(k.ty.ui)
                .color(k.c.text_base)
                .wrap(),
        )
}

fn heading(label: &str, k: &Kit) -> El {
    text(label).font(k.ty.ui_medium).color(k.c.text_muted)
}

fn divider(k: &Kit) -> El {
    el().w_full().h(1.0).fill(k.c.border_muted).a11y_hidden()
}
