//! Graph Studio shell. Content and application actions live in focused modules.
mod drawer;
mod graph_controls;

use crate::{
    app::State,
    camera, icons, inspector,
    presentation::{self, Drawer},
    theme,
};
use rune::{Cx, El, Key, KeyEvent, Tx, col, el, row, text};
use rune_kit::{
    Button, ButtonVariant, IconButton, IconButtonVariant, Kit,
    shell::{SplitView, Toolbar},
};

pub fn view(state: &State, cx: &mut Cx<State>) -> El {
    let k = theme::kit(cx.system_dark());
    let mut layer = Vec::new();
    let main = col()
        .w_full()
        .h_full()
        .min_w(0.0)
        .min_h(0.0)
        .child(graph_controls::toolbar(state, &k))
        .child(active_analysis(state, &k))
        .child(graph_with_filters(state, cx, &k))
        .child(graph_controls::legend(&k))
        .child(graph_controls::footer(state, &k, &mut layer));
    let split = SplitView::new(main)
        .sidebar(
            &state.ui.sidebar,
            presentation::sidebar,
            drawer::view(state, &k),
        )
        .inspector(
            &state.ui.inspector,
            presentation::inspector,
            inspector::view(state, &k),
        )
        .titlebar_inset((cx.titlebar_inset() - theme::RAIL_WIDTH).max(0.0))
        .hide_sidebar_toggle()
        .frame_inset(0.0)
        .reduced_motion(cx.is_reduced_motion())
        .toolbar(
            Toolbar::new()
                .leading(text("CodeGraph").font(k.ty.ui_medium).color(k.c.text_base))
                .leading(workspace_identity(state, &k))
                .trailing(analysis_action(state, &k))
                .trailing(
                    IconButton::new(&icons::LAYOUT_RIGHT)
                        .label("Inspector")
                        .help("Show or hide inspector")
                        .variant(IconButtonVariant::GhostMuted)
                        .expanded(state.ui.inspector.is_open())
                        .on_press(State::toggle_inspector)
                        .build(&k)
                        .key("toggle-inspector"),
                ),
        )
        .build(&k)
        .grow(1.0)
        .min_w(0.0);
    let body = col()
        .key("application")
        .w_full()
        .h_full()
        .fill(k.c.bg_base)
        .child(
            row()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .on_layout(|s: &mut State, rect| {
                    s.constrain_panes(rect[2]);
                })
                .child(activity_rail(state, &k))
                .child(split),
        )
        .child(status_bar(state, &k));
    rune_kit::root(body, layer)
}

fn graph_with_filters(state: &State, cx: &mut Cx<State>, k: &Kit) -> El {
    el().key("graph-canvas-wrapper")
        .grow(1.0)
        .min_w(0.0)
        .min_h(0.0)
        .child(camera::graph_canvas(state, cx, k))
        .child(graph_controls::filters(state, k))
}

fn activity_rail(state: &State, k: &Kit) -> El {
    let width = theme::RAIL_WIDTH;
    let mut rail = col()
        .key("activity-rail")
        .w(width)
        .h_full()
        .pt(56.0)
        .gap(theme::SPACE)
        .items_center()
        .fill(k.c.bg_deep);
    for (section, glyph, label, key) in [
        (
            Drawer::Repository,
            &icons::FOLDER,
            "Repository",
            "drawer-repository",
        ),
        (Drawer::Search, k.glyph.search, "Search", "drawer-search"),
    ] {
        rail = rail.child(
            IconButton::new(glyph)
                .label(label)
                .help(label)
                .variant(IconButtonVariant::GhostMuted)
                .expanded(state.ui.sidebar.is_open() && state.ui.drawer == section)
                .on_press(move |s: &mut State| s.toggle_drawer(section))
                .build(k)
                .key(key),
        );
    }
    rail
}

fn workspace_identity(state: &State, k: &Kit) -> El {
    let path = if state.root.text.is_empty() {
        state
            .graph
            .as_ref()
            .and_then(|graph| graph.data.roots.first())
            .map(String::as_str)
            .unwrap_or("")
    } else {
        &state.root.text
    };
    let label = std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "Local workspace".into());
    text(label)
        .key("workspace-identity")
        .font(k.ty.small)
        .color(k.c.text_muted)
        .min_w(0.0)
        .max_w(200.0)
        .truncate()
        .help(path)
}

fn analysis_action(state: &State, k: &Kit) -> El {
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
        .disabled(state.store.is_none() || state.targets.iter().all(|t| !t.selected))
        .on_press(State::analyze_selected)
        .build(k)
        .key("analyze")
    }
}

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

fn status_bar(state: &State, k: &Kit) -> El {
    let status = if !state.warning.is_empty() {
        &state.warning
    } else if state.stale {
        "Source files changed · refresh to update this graph"
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
    col()
        .key("status-bar")
        .w_full()
        .px(theme::SPACE)
        .py(6.0)
        .gap(3.0)
        .fill(k.c.bg_base)
        .child(
            text(status)
                .key("status-message")
                .w_full()
                .wrap()
                .max_lines(2)
                .font(k.ty.small)
                .color(if state.warning.is_empty() {
                    k.c.text_muted
                } else {
                    k.c.warning.fg
                }),
        )
        .child(
            text(counts)
                .w_full()
                .wrap()
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
}

pub fn key(state: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    if event.is_command('f') {
        state.open_search();
        return true;
    }
    if event.is_command('o') {
        state.choose_folder(tx);
        return true;
    }
    if rune_kit::shell::is_sidebar_toggle(event) {
        state.toggle_drawer(state.ui.drawer);
        return true;
    }
    match &event.key {
        Key::Left if event.mods.alt => state.history(false),
        Key::Right if event.mods.alt => state.history(true),
        Key::Escape if state.ui.overlay.is_some() => state.close_overlay(),
        Key::Escape => state.overview(),
        _ => return false,
    }
    true
}
