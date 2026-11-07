use crate::{
    app::{RESULT_PAGE_SIZE, State},
    view::{button, section_label},
};
use rune::tok::{self, TextStyle};
use rune::{El, col, row, text};

pub fn list(state: &State, dark: bool) -> El {
    let mut list = col().w_full().gap(5.0).child(section_label(
        format!("RELATIONSHIPS · {}", state.relationships.len()),
        dark,
    ));
    list = list.child(pager(state, dark));
    let Some(graph) = &state.graph else {
        return list;
    };
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
        list = list.child(
            button(label, format!("relationship-{id}"), dark, move |state| {
                state.selected_edge = Some(id)
            })
            .w_full(),
        );
    }
    list
}

pub fn selected(state: &State, dark: bool) -> El {
    let mut panel = col().w_full().gap(5.0);
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
    panel = panel.child(section_label("SELECTED RELATIONSHIP", dark));
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
                .font(TextStyle::new(11.0, 400.0))
                .color(tok::theme(dark).text_secondary),
        );
    }
    panel
}

fn pager(state: &State, dark: bool) -> El {
    let pages = state.relationships.len().div_ceil(RESULT_PAGE_SIZE).max(1);
    row()
        .w_full()
        .gap(5.0)
        .child(
            button("←", "relationships-prev", dark, |state| {
                state.step_relationships(false)
            })
            .disabled(state.relationship_page == 0),
        )
        .child(
            text(format!("{} / {pages}", state.relationship_page + 1))
                .font(tok::type_::LABEL)
                .color(tok::theme(dark).text_secondary),
        )
        .child(
            button("→", "relationships-next", dark, |state| {
                state.step_relationships(true)
            })
            .disabled(state.relationship_page + 1 >= pages),
        )
}
