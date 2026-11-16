//! Focused method inspection composed from Rune Kit controls and disclosures.
use crate::{
    app::{RESULT_PAGE_SIZE, State},
    presentation::Section,
    theme,
    view::pane_controls,
};
use rune::{El, ScrollSpec, TextStyle, col, row, text};
use rune_kit::{Button, ButtonVariant, Collapsible, Kit};

const BODY_UNAVAILABLE: &str =
    "The analyzer reports this target but no project method body is available.";

pub fn view(state: &State, k: &Kit) -> El {
    let content = col()
        .w(state.ui.inspector.width() - 32.0)
        .gap(theme::SPACE)
        .child(selection(state, k))
        .child(
            Collapsible::new("Analysis details", state.ui.section_open(Section::Analysis))
                .on_toggle(|s: &mut State| s.toggle_section(Section::Analysis))
                .child(analysis_details(state, k))
                .build(k)
                .key("analysis-section"),
        );
    col()
        .w_full()
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base)
        .child(pane_controls::header(
            state,
            k,
            pane_controls::Side::Inspector,
        ))
        .child(
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
            "Click an element to center its caller/callee tree. Double-click groups to reach classes, then click a method row or double-click the class header to explore methods.",
            k,
        );
    };
    let mut content = method_summary(node, k).child(primary_actions(state, node, k));
    if node.body_unavailable {
        content = content.child(detail(BODY_UNAVAILABLE, k));
    }
    let mut content = content
        .child(neighbors(state, true, k))
        .child(neighbors(state, false, k))
        .child(relationships(state, k))
        .child(
            Collapsible::new(
                "Shortest directed path",
                state.ui.section_open(Section::Path),
            )
            .on_toggle(|s: &mut State| s.toggle_section(Section::Path))
            .child(path_tools(state, k))
            .build(k)
            .key("path-section"),
        );
    if let Some(details) = architecture_details(state, node, k) {
        content = content.child(details);
    }
    if state
        .graph
        .as_ref()
        .and_then(|graph| graph.data.entry_points.as_ref())
        .is_some_and(|entries| entries.framework_entries.contains(&node.id))
    {
        content = content.child(detail("Framework entry · statically recognized", k));
    }
    content
}

fn architecture_details(
    state: &State,
    node: &codecallnavigator::model::Node,
    k: &Kit,
) -> Option<El> {
    let architecture = state.architecture.as_ref()?;
    let key = codecallnavigator::architecture::file_key(&node.build, &node.project, &node.file);
    let assignment = architecture.files.get(&key)?;
    Some(
        col()
            .w_full()
            .gap(4.0)
            .child(detail(format!("Layer: {}", assignment.layer), k))
            .child(detail(format!("Rule: {}", assignment.rule), k))
            .child(detail(format!("Configuration: {}", architecture.origin), k)),
    )
}

fn method_summary(node: &codecallnavigator::model::Node, k: &Kit) -> El {
    col()
        .w_full()
        .gap(8.0)
        .child(detail(format!("{} · {}", node.language, node.kind), k))
        .child(
            text(&node.label)
                .font(TextStyle::new(21.0, 650.0))
                .color(k.c.text_base)
                .w_full()
                .wrap_anywhere(),
        )
        .child(
            text(&node.signature)
                .font(theme::mono(14.0, 400.0))
                .color(k.c.text_base)
                .selectable(true)
                .selection_color(k.selection())
                .w_full()
                .wrap_anywhere(),
        )
        .child(source_location(node, k))
}

fn source_location(node: &codecallnavigator::model::Node, k: &Kit) -> El {
    let location = format!(
        "{}:{}",
        node.file.rsplit(['/', '\\']).next().unwrap_or(&node.file),
        node.line
    );
    text(location)
        .font(k.ty.ui)
        .color(k.c.text_accent)
        .w_full()
        .truncate()
        .key("selected-source-location")
        .role("button")
        .label(format!("View source · {}:{}", node.file, node.line))
        .help(&node.file)
        .focusable(true)
        .focus_ring(2.0, 2.5, k.c.border_focus)
        .on_press(State::open_selected_source)
}

fn primary_actions(state: &State, node: &codecallnavigator::model::Node, k: &Kit) -> El {
    let mut actions = row().w_full().gap(8.0).flex_wrap().child(
        Button::new("View source")
            .icon(&rune_kit::glyphs::FILE_CODE)
            .label_with(|_, ink| {
                text("View source")
                    .font(TextStyle::new(12.0, 400.0))
                    .color(ink)
            })
            .variant(ButtonVariant::Outline)
            .on_press(State::open_selected_source)
            .build(k)
            .key("view-selected-source"),
    );
    if state.flow_supported(&node.language) {
        actions = actions.child(flow_action(node.body_unavailable, k));
    }
    actions
}

fn focus_action(k: &Kit) -> El {
    Button::new("Focus this method")
        .variant(ButtonVariant::Ghost)
        .on_press(|state: &mut State| {
            if let Some(selected) = state.selected() {
                state.select_method(selected, crate::graph_interaction::SelectionAction::Focus);
            }
        })
        .build(k)
        .key("focus-selected")
}

fn relationships(state: &State, k: &Kit) -> El {
    Collapsible::new(
        format!("Relationships · {}", state.relationships.len()),
        state.ui.section_open(Section::Relationships),
    )
    .on_toggle(|s: &mut State| s.toggle_section(Section::Relationships))
    .child(crate::relationship::list(state, k))
    .build(k)
    .key("relationships-section")
}

fn analysis_details(state: &State, k: &Kit) -> El {
    let mut content = col().w_full().gap(8.0).child(focus_action(k));
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
    if raw.trim().is_empty() {
        return vec![
            "Analyzer provenance is unavailable in this saved graph; refresh to collect it.".into(),
        ];
    }
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
        format!("{title} ({})", ids.len()),
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
    let mut rows = col()
        .w_full()
        .min_w(0.0)
        .gap(0.0)
        .role("listbox")
        .label(if incoming { "Callers" } else { "Callees" })
        .focus_group();
    if let Some(graph) = &state.graph {
        for (position, &id) in ids
            .iter()
            .enumerate()
            .skip(page * RESULT_PAGE_SIZE)
            .take(RESULT_PAGE_SIZE)
        {
            let selected = state.selected() == Some(id);
            let mut item = neighbor_row(id, &graph.data.nodes[id as usize], k)
                .key(format!("neighbor-{incoming}-{id}"))
                .focus_selected(selected)
                .position_in_set(position + 1, ids.len());
            if selected {
                item = item.fill(k.c.text_accent.alpha(0.10)).child(
                    rune::icon(k.glyph.check_small, 16.0)
                        .color(k.c.text_accent)
                        .a11y_hidden(),
                );
            }
            rows = rows.child(item);
        }
    }
    let mut content = col().w_full().gap(6.0);
    if ids.len() > RESULT_PAGE_SIZE {
        content = content.child(neighbor_pager(state, incoming, ids.len(), k));
    }
    content.child(rows.key(format!("neighbor-list-{incoming}")))
}

fn neighbor_row(id: u32, node: &codecallnavigator::model::Node, k: &Kit) -> El {
    let location = format!(
        "{}:{}",
        node.file.rsplit(['/', '\\']).next().unwrap_or(&node.file),
        node.line
    );
    row()
        .role("option")
        .label(format!("{} · {}:{}", node.label, node.file, node.line))
        .help(format!("{} · {}:{}", node.signature, node.file, node.line))
        .focusable(true)
        .focus_ring(2.0, 2.5, k.c.border_focus)
        .focus_highlight(k.c.surface_raised_hover)
        .w_full()
        .min_w(0.0)
        .h(48.0)
        .px(8.0)
        .gap(6.0)
        .center_y()
        .hover(k.c.surface_raised_hover)
        .child(
            col()
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(&node.label)
                        .font(TextStyle::new(14.0, 400.0))
                        .color(k.c.text_accent)
                        .w_full()
                        .truncate()
                        .a11y_hidden(),
                )
                .child(
                    text(location)
                        .font(TextStyle::new(12.0, 400.0))
                        .color(k.c.text_muted)
                        .w_full()
                        .truncate()
                        .a11y_hidden(),
                ),
        )
        .child(
            rune::el()
                .h(1.0)
                .pin(Some(8.0), None, Some(8.0), Some(0.0))
                .fill(k.c.border_muted)
                .a11y_hidden(),
        )
        .on_press(move |state: &mut State| {
            state.select_method(id, crate::graph_interaction::SelectionAction::Focus);
        })
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

fn flow_action(body_unavailable: bool, k: &Kit) -> El {
    Button::new("Open flowchart")
        .label_with(|_, ink| {
            text("Open flowchart")
                .font(TextStyle::new(12.0, 400.0))
                .color(ink)
        })
        .disabled(body_unavailable)
        .help(if body_unavailable {
            BODY_UNAVAILABLE
        } else {
            "Open this method's analyzed source and function flow"
        })
        .variant(ButtonVariant::Outline)
        .on_press(State::open_selected_flow)
        .build(k)
        .key("open-flowchart")
}

#[cfg(test)]
mod tests {
    #[test]
    fn legacy_empty_analyzer_provenance_requests_refresh_explicitly() {
        assert_eq!(
            super::engine_summary(""),
            vec!["Analyzer provenance is unavailable in this saved graph; refresh to collect it."]
        );
    }
}
