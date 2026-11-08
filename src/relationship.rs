//! Relationship details and paged, selectable Kit rows.
use crate::app::{RESULT_PAGE_SIZE, State};
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, List, ListItem};

pub fn list(state: &State, k: &Kit) -> El {
    let mut rows = List::new()
        .active(None)
        .divider()
        .empty_message("No relationships reported.");
    if let Some(graph) = &state.graph {
        let selected = state.selected();
        for &id in state
            .relationships
            .iter()
            .skip(state.relationship_page * RESULT_PAGE_SIZE)
            .take(RESULT_PAGE_SIZE)
        {
            let edge = &graph.data.edges[id as usize];
            let other = if selected == Some(edge.from) {
                edge.to
            } else {
                edge.from
            };
            let label = format!(
                "{} · {} ×{} · {}",
                edge.kind, edge.certainty, edge.count, graph.data.nodes[other as usize].label
            );
            rows = rows.item(
                ListItem::new(label)
                    .selected(state.selected_edge == Some(id))
                    .on_select(move |state: &mut State, _| state.selected_edge = Some(id)),
            );
        }
    }
    col()
        .w_full()
        .gap(8.0)
        .child(pager(state, k))
        .child(rows.build(k).key("relationship-list"))
        .child(selected(state, k))
}

pub fn selected(state: &State, k: &Kit) -> El {
    let mut panel = col().w_full().gap(6.0);
    let Some(graph) = &state.graph else {
        return panel;
    };
    let Some(edge) = state
        .selected_edge
        .and_then(|id| graph.data.edges.get(id as usize))
    else {
        return panel;
    };
    let from = &graph.data.nodes[edge.from as usize];
    let to = &graph.data.nodes[edge.to as usize];
    panel = panel.child(
        text("Selected relationship")
            .font(k.ty.ui_medium)
            .color(k.c.text_base),
    );
    for line in [
        format!("From: {}", from.signature),
        format!("To: {}", to.signature),
        format!("Kind: {}", edge.kind),
        format!("Certainty: {}", edge.certainty),
        format!("Occurrences: {}", edge.count),
        format!("Build: {}", from.build),
        format!("Analyzer: {}", graph.data.engine_versions),
    ] {
        panel = panel.child(
            text(line)
                .w_full()
                .wrap_anywhere()
                .selectable(true)
                .font(k.ty.text_xsmall)
                .color(k.c.text_muted),
        );
    }
    panel
}

fn pager(state: &State, k: &Kit) -> El {
    let pages = state.relationships.len().div_ceil(RESULT_PAGE_SIZE).max(1);
    row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            Button::new("Previous")
                .variant(ButtonVariant::Ghost)
                .disabled(state.relationship_page == 0)
                .on_press(|state: &mut State| state.step_relationships(false))
                .build(k)
                .key("relationships-prev"),
        )
        .child(
            text(format!("{} / {pages}", state.relationship_page + 1))
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            Button::new("Next")
                .variant(ButtonVariant::Ghost)
                .disabled(state.relationship_page + 1 >= pages)
                .on_press(|state: &mut State| state.step_relationships(true))
                .build(k)
                .key("relationships-next"),
        )
}
