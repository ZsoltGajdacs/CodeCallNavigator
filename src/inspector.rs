//! Focused method inspection composed from Rune Kit controls and disclosures.
use crate::{
    app::{RESULT_PAGE_SIZE, State},
    presentation::Section,
    theme,
};
use rune::{El, ScrollSpec, col, row, text};
use rune_kit::{Button, ButtonVariant, Collapsible, Kit, List, ListItem};

pub fn view(state: &State, k: &Kit) -> El {
    let content = col()
        .w(state.ui.inspector.width() - 32.0)
        .gap(theme::SPACE)
        .child(text("Inspector").font(k.ty.ui_medium).color(k.c.text_muted))
        .child(selection(state, k))
        .child(
            Collapsible::new("Analysis details", state.ui.section_open(Section::Analysis))
                .on_toggle(|s: &mut State| s.toggle_section(Section::Analysis))
                .child(analysis_details(state, k))
                .build(k)
                .key("analysis-section"),
        );
    col().w_full().h_full().min_h(0.0).fill(k.c.bg_base).child(
        col()
            .key("inspector-scroll")
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(col().w_full().padding(16.0).child(content)),
    )
}

fn selection(state: &State, k: &Kit) -> El {
    let Some(node) = state
        .selected()
        .and_then(|id| state.graph.as_ref()?.data.nodes.get(id as usize))
    else {
        return detail(
            "Select a method to inspect its callers, callees and provenance. Click a group to expand it; double-click a method to focus its neighborhood.",
            k,
        );
    };
    let mut content = method_summary(node, k)
        .child(
            Collapsible::new(
                "Shortest directed path",
                state.ui.section_open(Section::Path),
            )
            .on_toggle(|s: &mut State| s.toggle_section(Section::Path))
            .child(path_tools(state, k))
            .build(k)
            .key("path-section"),
        )
        .child(
            Collapsible::new(
                format!("Relationships · {}", state.relationships.len()),
                state.ui.section_open(Section::Relationships),
            )
            .on_toggle(|s: &mut State| s.toggle_section(Section::Relationships))
            .child(crate::relationship::list(state, k))
            .build(k)
            .key("relationships-section"),
        )
        .child(neighbors(state, true, k))
        .child(neighbors(state, false, k));
    if node.body_unavailable {
        content = content.child(detail(
            "The analyzer reports this target but no project method body is available.",
            k,
        ));
    }
    content
}

fn method_summary(node: &codegraph::model::Node, k: &Kit) -> El {
    col()
        .w_full()
        .gap(theme::SPACE)
        .child(
            text(&node.label)
                .font(theme::mono(20.0, 650.0))
                .color(k.c.text_base)
                .w_full()
                .wrap_anywhere(),
        )
        .child(
            text(&node.signature)
                .font(theme::mono(12.0, 400.0))
                .color(k.c.text_base)
                .selectable(true)
                .selection_color(k.selection())
                .w_full()
                .wrap_anywhere(),
        )
        .child(detail(
            format!("{} · {} · {}", node.project, node.language, node.kind),
            k,
        ))
        .child(detail(format!("{}:{}", node.file, node.line), k))
        .child(
            Button::new("Focus this method")
                .variant(ButtonVariant::Outline)
                .on_press(|state: &mut State| {
                    if let Some(selected) = state.selected() {
                        state.select(selected, true);
                    }
                })
                .build(k)
                .key("focus-selected")
                .w_full(),
        )
}

fn analysis_details(state: &State, k: &Kit) -> El {
    let mut content = col().w_full().gap(8.0);
    if let Some(graph) = &state.graph {
        content = content.child(
            text("Active analysis inputs")
                .font(k.ty.small_medium)
                .color(k.c.text_muted),
        );
        for root in &graph.data.roots {
            content = content.child(detail(root, k));
        }
        content = content.child(provenance(state, k));
        for note in &graph.data.diagnostics {
            content = content.child(detail(note, k));
        }
    }
    if !state.warning.is_empty() {
        content = content
            .child(
                text("Last analysis / layout message")
                    .font(k.ty.small_medium)
                    .color(k.c.warning.fg),
            )
            .child(detail(&state.warning, k));
    }
    content
}

fn provenance(state: &State, k: &Kit) -> El {
    let mut panel = col().w_full().gap(6.0).child(
        text("Analyzer provenance")
            .font(k.ty.small_medium)
            .color(k.c.text_muted),
    );
    let Some(graph) = &state.graph else {
        return panel;
    };
    for line in engine_summary(&graph.data.engine_versions) {
        panel = panel.child(detail(line, k));
    }
    panel = panel.child(
        Button::new(if state.show_provenance {
            "Hide full provenance"
        } else {
            "Show full provenance"
        })
        .variant(ButtonVariant::Ghost)
        .on_press(|state: &mut State| state.show_provenance = !state.show_provenance)
        .build(k)
        .key("toggle-provenance")
        .w_full(),
    );
    if state.show_provenance {
        panel = panel.child(detail(&graph.data.engine_versions, k));
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

fn detail(value: impl Into<String>, k: &Kit) -> El {
    text(value)
        .font(k.ty.text_xsmall)
        .color(k.c.text_muted)
        .selectable(true)
        .selection_color(k.selection())
        .w_full()
        .wrap_anywhere()
}

fn endpoint_label<'a>(state: &'a State, endpoint: Option<&str>) -> &'a str {
    endpoint
        .and_then(|id| state.graph.as_ref()?.node_index(id))
        .map(|id| {
            state.graph.as_ref().unwrap().data.nodes[id as usize]
                .label
                .as_str()
        })
        .unwrap_or("not selected")
}

fn path_tools(state: &State, k: &Kit) -> El {
    let from = endpoint_label(state, state.nav.path_from.as_deref());
    let to = endpoint_label(state, state.nav.path_to.as_deref());
    col().w_full().gap(6.0)
        .child(Button::new(format!("Set start · {from}")).variant(ButtonVariant::Ghost)
            .on_press(|state: &mut State| { state.nav.path_from = state.nav.selected.clone(); state.dirty_session = true; })
            .build(k).key("path-from").w_full())
        .child(Button::new(format!("Set end · {to}")).variant(ButtonVariant::Ghost)
            .on_press(|state: &mut State| { state.nav.path_to = state.nav.selected.clone(); state.dirty_session = true; })
            .build(k).key("path-to").w_full())
        .child(Button::new("Find path").variant(ButtonVariant::Outline)
            .disabled(state.nav.path_from.is_none() || state.nav.path_to.is_none())
            .on_press(|state: &mut State| { state.checkpoint(); state.nav.show_path = true; state.auto_fit = true; state.changed(); })
            .build(k).key("find-path").w_full())
        .child(detail("Uses the current possible-target and callback filters. An empty result means no path in this analyzed graph.", k))
}

fn neighbors(state: &State, incoming: bool, k: &Kit) -> El {
    let ids = if incoming {
        &state.neighbors.0
    } else {
        &state.neighbors.1
    };
    let (title, section, key) = if incoming {
        ("Callers", Section::Callers, "callers-section")
    } else {
        ("Callees", Section::Callees, "callees-section")
    };
    let content = neighbor_list(state, incoming, ids, k);
    Collapsible::new(
        format!("{title} · {}", ids.len()),
        state.ui.section_open(section),
    )
    .on_toggle(move |s: &mut State| s.toggle_section(section))
    .child(content)
    .build(k)
    .key(key)
}

fn neighbor_list(state: &State, incoming: bool, ids: &[u32], k: &Kit) -> El {
    if ids.is_empty() {
        return detail("None reported with the current filters.", k);
    }
    let page = state.neighbor_pages[usize::from(!incoming)];
    let mut rows = List::new().active(None).divider();
    if let Some(graph) = &state.graph {
        for &id in ids
            .iter()
            .skip(page * RESULT_PAGE_SIZE)
            .take(RESULT_PAGE_SIZE)
        {
            rows = rows.item(
                ListItem::new(graph.data.nodes[id as usize].signature.clone())
                    .selected(state.selected() == Some(id))
                    .on_select(move |state: &mut State, _| state.select(id, true)),
            );
        }
    }
    col()
        .w_full()
        .gap(6.0)
        .child(neighbor_pager(state, incoming, ids.len(), k))
        .child(rows.build(k).key(format!("neighbor-list-{incoming}")))
}

fn neighbor_pager(state: &State, incoming: bool, count: usize, k: &Kit) -> El {
    let page = state.neighbor_pages[usize::from(!incoming)];
    let pages = count.div_ceil(RESULT_PAGE_SIZE).max(1);
    row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            Button::new("Previous")
                .variant(ButtonVariant::Ghost)
                .disabled(page == 0)
                .on_press(move |state: &mut State| state.step_neighbors(incoming, false))
                .build(k)
                .key(format!("neighbors-prev-{incoming}")),
        )
        .child(
            text(format!("{} / {pages}", page + 1))
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            Button::new("Next")
                .variant(ButtonVariant::Ghost)
                .disabled(page + 1 >= pages)
                .on_press(move |state: &mut State| state.step_neighbors(incoming, true))
                .build(k)
                .key(format!("neighbors-next-{incoming}")),
        )
}
