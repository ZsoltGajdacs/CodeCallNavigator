use crate::{
    app::{RESULT_PAGE_SIZE, State},
    view::{button, section_label},
};
use rune::tok::{self, TextStyle};
use rune::{El, ScrollSpec, col, row, text};

pub fn view(state: &State, dark: bool) -> El {
    let theme = tok::theme(dark);
    let mut content = col()
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .child(section_label("INSPECTOR", dark));
    content = selection(state, dark, content);
    content = content.child(section_label("RELATIONSHIP LEGEND", dark))
        .child(detail("Solid · direct target\nGold dashes · possible or uncertain target\nPurple dashes · callback / reference\nAmber node border · recursion or cycle", dark));
    if let Some(graph) = &state.graph {
        content = content.child(section_label("ACTIVE ANALYSIS INPUTS", dark));
        for root in &graph.data.roots {
            content = content.child(detail(root, dark));
        }
        content = content.child(provenance(state, dark));
        for note in &graph.data.diagnostics {
            content = content.child(detail(note, dark));
        }
    }
    if !state.warning.is_empty() {
        content = content
            .child(section_label("LAST ANALYSIS / LAYOUT MESSAGE", dark))
            .child(detail(&state.warning, dark));
    }
    col().w(290.0).h_full().min_h(0.0).fill(theme.card).child(
        col()
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(content),
    )
}

fn selection(state: &State, dark: bool, mut content: El) -> El {
    let theme = tok::theme(dark);
    if let Some(node) = state.selected().and_then(|id| {
        state
            .graph
            .as_ref()
            .and_then(|graph| graph.data.nodes.get(id as usize))
    }) {
        content = content
            .child(
                text(&node.label)
                    .font(TextStyle::new(19.0, 650.0))
                    .color(theme.text),
            )
            .child(detail(&node.signature, dark))
            .child(detail(&node.project, dark))
            .child(detail(format!("{} · {}", node.language, node.kind), dark))
            .child(detail(format!("{}:{}", node.file, node.line), dark))
            .child(
                button("Focus this method", "focus-selected", dark, |state| {
                    if let Some(selected) = state.selected() {
                        state.select(selected, true);
                    }
                })
                .w_full(),
            )
            .child(path_tools(state, dark))
            .child(crate::relationship::selected(state, dark))
            .child(crate::relationship::list(state, dark))
            .child(neighbor_list(state, true, dark))
            .child(neighbor_list(state, false, dark));
        if node.body_unavailable {
            content = content.child(detail(
                "The analyzer reports this target but no project method body is available.",
                dark,
            ));
        }
    } else {
        content = content.child(detail("Select a method to inspect its callers, callees and provenance. Click a group to expand it; double-click a method to focus its neighborhood.", dark));
    }
    content
}

fn provenance(state: &State, dark: bool) -> El {
    let mut panel = col()
        .w_full()
        .gap(6.0)
        .child(section_label("ANALYZER PROVENANCE", dark));
    let Some(graph) = &state.graph else {
        return panel;
    };
    for line in engine_summary(&graph.data.engine_versions) {
        panel = panel.child(detail(line, dark));
    }
    panel = panel.child(
        button(
            if state.show_provenance {
                "Hide full provenance"
            } else {
                "Show full provenance"
            },
            "toggle-provenance",
            dark,
            |state| state.show_provenance = !state.show_provenance,
        )
        .w_full(),
    );
    if state.show_provenance {
        panel = panel.child(detail(&graph.data.engine_versions, dark));
    }
    panel
}

pub(crate) fn engine_summary(raw: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return vec![raw.to_string()];
    };
    let mut lines = Vec::new();
    if let Some(revision) = value["callatlas"]["revision"].as_str() {
        lines.push(format!(
            "CallAtlas · {}",
            revision.chars().take(12).collect::<String>()
        ));
    }
    if let Some(version) = value["java-callgraph2"]["version"].as_str() {
        lines.push(format!(
            "java-callgraph2 · {version} · BCEL {}",
            value["java-callgraph2"]["bcel_version"]
                .as_str()
                .unwrap_or("not reported")
        ));
    }
    if lines.is_empty() {
        lines.push("Analyzer versions are available in full provenance.".into());
    }
    lines
}

fn detail(value: impl Into<String>, dark: bool) -> El {
    text(value)
        .font(TextStyle::new(11.0, 400.0))
        .color(tok::theme(dark).text_secondary)
        .selectable(true)
        .w_full()
        .wrap_anywhere()
}

fn path_tools(state: &State, dark: bool) -> El {
    let from = state
        .nav
        .path_from
        .as_ref()
        .and_then(|id| state.graph.as_ref()?.node_index(id))
        .map(|id| {
            state.graph.as_ref().unwrap().data.nodes[id as usize]
                .label
                .as_str()
        })
        .unwrap_or("not selected");
    let to = state
        .nav
        .path_to
        .as_ref()
        .and_then(|id| state.graph.as_ref()?.node_index(id))
        .map(|id| {
            state.graph.as_ref().unwrap().data.nodes[id as usize]
                .label
                .as_str()
        })
        .unwrap_or("not selected");
    col().w_full().gap(5.0).child(section_label("SHORTEST DIRECTED PATH", dark))
        .child(button(format!("Set start · {from}"), "path-from", dark, |state| { state.nav.path_from = state.nav.selected.clone(); state.dirty_session = true; }).w_full())
        .child(button(format!("Set end · {to}"), "path-to", dark, |state| { state.nav.path_to = state.nav.selected.clone(); state.dirty_session = true; }).w_full())
        .child(button("Find path", "find-path", dark, |state| { state.checkpoint(); state.nav.show_path = true; state.auto_fit = true; state.changed(); })
            .w_full().disabled(state.nav.path_from.is_none() || state.nav.path_to.is_none()))
        .child(detail("Uses the current possible-target and callback filters. An empty result means no path in this analyzed graph.", dark))
}

fn neighbor_list(state: &State, incoming: bool, dark: bool) -> El {
    let ids = if incoming {
        &state.neighbors.0
    } else {
        &state.neighbors.1
    };
    let heading = if incoming { "CALLERS" } else { "CALLEES" };
    let page = state.neighbor_pages[usize::from(!incoming)];
    let mut list = col()
        .w_full()
        .gap(4.0)
        .child(section_label(format!("{heading} · {}", ids.len()), dark));
    if ids.is_empty() {
        return list.child(detail("None reported with the current filters.", dark));
    }
    list = list.child(neighbor_pager(state, incoming, ids.len(), dark));
    let Some(graph) = &state.graph else {
        return list;
    };
    for &id in ids
        .iter()
        .skip(page * RESULT_PAGE_SIZE)
        .take(RESULT_PAGE_SIZE)
    {
        let label = graph.data.nodes[id as usize].signature.clone();
        list = list.child(
            button(
                label,
                format!("neighbor-{incoming}-{id}"),
                dark,
                move |state| state.select(id, true),
            )
            .w_full(),
        );
    }
    list
}

fn neighbor_pager(state: &State, incoming: bool, count: usize, dark: bool) -> El {
    let page = state.neighbor_pages[usize::from(!incoming)];
    let pages = count.div_ceil(RESULT_PAGE_SIZE).max(1);
    row()
        .w_full()
        .gap(5.0)
        .child(
            button(
                "←",
                format!("neighbors-prev-{incoming}"),
                dark,
                move |state| state.step_neighbors(incoming, false),
            )
            .disabled(page == 0),
        )
        .child(
            text(format!("{} / {pages}", page + 1))
                .font(tok::type_::LABEL)
                .color(tok::theme(dark).text_secondary),
        )
        .child(
            button(
                "→",
                format!("neighbors-next-{incoming}"),
                dark,
                move |state| state.step_neighbors(incoming, true),
            )
            .disabled(page + 1 >= pages),
        )
}
