use crate::{app::State, camera, inspector, projection::Grouping};
use codegraph::graph::Direction;
use rune::tok::{self, TextStyle};
use rune::{Cx, El, Key, KeyEvent, ScrollSpec, Tx, col, el, row, text, text_input};

pub fn button(
    label: impl Into<String>,
    key: impl Into<String>,
    dark: bool,
    action: impl Fn(&mut State) + 'static,
) -> El {
    button_base(label, key, dark).on_press(action)
}

fn button_base(label: impl Into<String>, key: impl Into<String>, dark: bool) -> El {
    let label = label.into();
    let theme = tok::theme(dark);
    el().key(key.into())
        .h(30.0)
        .px(10.0)
        .radius(6.0)
        .clip_rounded(6.0)
        .min_w(0.0)
        .fill(theme.control_off)
        .role("button")
        .label(&label)
        .focusable(true)
        .center()
        .child(
            text(label)
                .truncate()
                .max_w(218.0)
                .font(TextStyle::new(12.0, 500.0))
                .color(theme.text),
        )
}

struct ViewContext<'a> {
    state: &'a State,
    dark: bool,
}

pub fn view(state: &State, cx: &mut Cx<State>) -> El {
    let dark = cx.system_dark();
    let ui = &ViewContext { state, dark };
    let theme = tok::theme(dark);
    let center = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .child(toolbar(ui))
        .child(active_analysis(ui))
        .child(camera::graph_canvas(state, cx, dark))
        .child(canvas_footer(ui));
    col()
        .key("application")
        .w_full()
        .h_full()
        .fill(theme.bg)
        .child(header(ui))
        .child(
            row()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .child(sidebar(ui))
                .child(center)
                .child(inspector::view(state, dark)),
        )
        .child(status_bar(ui))
}

fn active_analysis(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
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
                        .into_owned()
                })
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_default();
    let message = if state.jobs.layout.is_pending() {
        "Updating graph view…".to_string()
    } else if names.is_empty() {
        state.graph_view.message.clone()
    } else {
        format!("Last successful analysis · {names}")
    };
    text(message)
        .w_full()
        .h(24.0)
        .px(12.0)
        .font(TextStyle::new(10.0, 400.0))
        .color(tok::theme(dark).text_secondary)
        .truncate()
}

fn header(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let theme = tok::theme(dark);
    row()
        .h(64.0)
        .w_full()
        .px(24.0)
        .gap(12.0)
        .center_y()
        .fill(theme.card)
        .child(el().w(62.0))
        .child(
            text("CodeGraph")
                .font(TextStyle::new(21.0, 700.0))
                .color(theme.text),
        )
        .child(
            text("Application call explorer")
                .font(tok::type_::BODY)
                .color(theme.text_secondary),
        )
        .child(el().grow(1.0))
        .child(
            text(if state.scene {
                "TEST FIXTURE"
            } else {
                "LOCAL WORKSPACE"
            })
            .font(TextStyle::new(10.0, 600.0))
            .color(theme.text_secondary),
        )
}

fn sidebar(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let theme = tok::theme(dark);
    let mut content = col()
        .w_full()
        .padding(16.0)
        .gap(12.0)
        .child(root_controls(ui))
        .child(targets(ui))
        .child(analyze_button(ui))
        .child(analysis_help(ui))
        .child(filters(ui))
        .child(search_controls(ui))
        .child(search_results(ui));
    if !state.nav.expanded.is_empty() {
        content = content.child(section_label("EXPANDED GROUPS", dark));
        for key in state.nav.expanded.iter().take(20) {
            let label = serde_json::from_str::<Vec<String>>(key)
                .ok()
                .and_then(|parts| parts.last().cloned())
                .unwrap_or_else(|| "Group".into());
            let key = key.clone();
            content = content.child(
                button(
                    format!("− {label}"),
                    format!("collapse-{key}"),
                    dark,
                    move |state| {
                        state.checkpoint();
                        state.nav.expanded.remove(&key);
                        state.auto_fit = true;
                        state.changed();
                    },
                )
                .w_full(),
            );
        }
    }
    col().w(270.0).h_full().min_h(0.0).fill(theme.card).child(
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

fn root_controls(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let theme = tok::theme(dark);
    col()
        .w_full()
        .gap(10.0)
        .child(section_label("PROJECT FOLDER", dark))
        .child(
            button_base("Choose folder…", "choose-folder", dark)
                .on_press_tx(|state: &mut State, tx| state.choose_folder(tx))
                .w_full(),
        )
        .child(
            text_input(&state.root, TextStyle::new(11.0, 400.0))
                .key("root-path")
                .single_line()
                .w_full()
                .h(32.0)
                .padding(6.0)
                .fill(theme.bg)
                .radius(5.0)
                .color(theme.text)
                .on_edit(|state: &mut State| &mut state.root)
                .on_input_change(|state: &mut State, _| state.root_edited())
                .on_submit(|state: &mut State, _| state.discover_root()),
        )
        .child(button("Find builds", "find-builds", dark, State::discover_root).w_full())
}

fn analyze_button(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let label = if state.graph.is_some() {
        "Refresh selected builds"
    } else {
        "Generate graph"
    };
    button(label, "analyze", dark, State::analyze_selected)
        .w_full()
        .disabled(
            state.analyzing
                || state.store.is_none()
                || state.targets.iter().all(|choice| !choice.selected),
        )
}

fn analysis_help(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let message = if state.jobs.boot.is_pending() {
        "Opening the saved workspace…"
    } else if state.analyzing {
        "Analyzing selected builds; the previous graph remains available."
    } else if state.targets.iter().all(|choice| !choice.selected) {
        "Select one or more builds to generate or refresh the graph."
    } else {
        "Analysis runs locally and may restore dependencies or compile project code."
    };
    text(message)
        .w_full()
        .wrap()
        .font(TextStyle::new(10.0, 400.0))
        .color(tok::theme(dark).text_secondary)
}

fn search_controls(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let theme = tok::theme(dark);
    col()
        .w_full()
        .gap(8.0)
        .child(section_label("FIND A METHOD", dark))
        .child(
            text_input(&state.query, tok::type_::BODY)
                .key("search")
                .single_line()
                .w_full()
                .h(34.0)
                .padding(7.0)
                .fill(theme.bg)
                .radius(6.0)
                .color(theme.text)
                .placeholder("Search all symbols…", theme.text_secondary)
                .on_edit(|state: &mut State| &mut state.query),
        )
}

pub fn section_label(label: impl Into<String>, dark: bool) -> El {
    text(label)
        .font(TextStyle::new(10.0, 650.0))
        .color(tok::theme(dark).text_secondary)
}

fn targets(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let theme = tok::theme(dark);
    let mut list = col().w_full().gap(4.0).child(section_label("BUILDS", dark));
    if state.targets.is_empty() {
        return list.child(
            text(if state.jobs.discovery.is_pending() {
                "Looking for build files…"
            } else {
                "Solutions, C# projects and Gradle builds appear here."
            })
            .font(TextStyle::new(11.0, 400.0))
            .color(theme.text_secondary)
            .w_full()
            .wrap_anywhere(),
        );
    }
    for (index, choice) in state.targets.iter().enumerate() {
        let label = format!(
            "{} {}",
            checkbox_marker(choice.selected),
            choice.target.label
        );
        list = list.child(
            button(label, format!("build-{index}"), dark, move |state| {
                state.targets[index].selected = !state.targets[index].selected;
                state.dirty_session = true;
            })
            .w_full(),
        );
    }
    list
}

type FilterOption = (&'static str, &'static str, bool, fn(&mut State));

fn filters(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let filter = &state.nav.filter;
    let items: [FilterOption; 5] = [
        ("Test code", "filter-tests", filter.include_tests, |state| {
            state.nav.filter.include_tests ^= true
        }),
        (
            "Generated code",
            "filter-generated",
            filter.include_generated,
            |state| state.nav.filter.include_generated ^= true,
        ),
        (
            "Unavailable bodies",
            "filter-unavailable",
            filter.include_unavailable,
            |state| state.nav.filter.include_unavailable ^= true,
        ),
        (
            "Possible / uncertain targets",
            "filter-uncertain",
            filter.include_uncertain,
            |state| state.nav.filter.include_uncertain ^= true,
        ),
        (
            "Callbacks / references",
            "filter-references",
            filter.include_references,
            |state| state.nav.filter.include_references ^= true,
        ),
    ];
    items.into_iter().fold(
        col()
            .w_full()
            .gap(4.0)
            .child(section_label("SHOW IN GRAPH", dark)),
        |list, (label, key, checked, action)| {
            list.child(
                button(
                    format!("{} {label}", if checked { "☑" } else { "☐" }),
                    key,
                    dark,
                    move |state| {
                        state.checkpoint();
                        action(state);
                        state.changed();
                    },
                )
                .w_full()
                .checked(checked)
                .role("checkbox"),
            )
        },
    )
}

fn search_results(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let mut list = state
        .hits
        .iter()
        .filter_map(|&index| search_hit(ui, index))
        .fold(
            col().w_full().gap(4.0).child(search_pager(ui)),
            |list, hit| list.child(hit),
        );
    if state.jobs.search.is_pending() {
        list = list.child(
            text("Searching…")
                .font(tok::type_::LABEL)
                .color(tok::theme(dark).text_secondary),
        );
    }
    if search_is_empty(state) {
        list = list.child(
            text("No matching symbols")
                .font(tok::type_::LABEL)
                .color(tok::theme(dark).text_secondary),
        );
    }
    list
}

fn search_pager(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    row()
        .w_full()
        .gap(4.0)
        .child(
            button("←", "search-prev", dark, |state| state.step_search(false))
                .disabled(state.search_page == 0),
        )
        .child(
            text(format!("Page {}", state.search_page + 1))
                .font(tok::type_::LABEL)
                .color(tok::theme(dark).text_secondary),
        )
        .child(
            button("→", "search-next", dark, |state| state.step_search(true))
                .disabled(!state.search_more),
        )
}

fn search_hit(ui: &ViewContext<'_>, index: u32) -> Option<El> {
    let (state, dark) = (ui.state, ui.dark);
    let node = state.graph.as_ref()?.data.nodes.get(index as usize)?;
    let suffix = if state.nav.filter.accepts_node(node) {
        ""
    } else {
        " · hidden, click to reveal"
    };
    Some(
        button(
            format!("{}{suffix}", node.signature),
            format!("search-hit-{index}"),
            dark,
            move |state| state.reveal(index),
        )
        .w_full(),
    )
}

fn search_is_empty(state: &State) -> bool {
    !state.query.text.is_empty() && state.hits.is_empty() && !state.jobs.search.is_pending()
}

fn checkbox_marker(checked: bool) -> &'static str {
    if checked { "☑" } else { "☐" }
}

fn toolbar(ui: &ViewContext<'_>) -> El {
    let tools = navigation_tools(ui);
    let tools = grouping_tools(ui, tools);
    let tools = zoom_tools(ui, tools);
    el().key("toolbar-scroll")
        .w_full()
        .h(44.0)
        .scroll_x()
        .child(tools.min_w(620.0))
}

fn navigation_tools(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    row()
        .w_full()
        .h(44.0)
        .px(10.0)
        .gap(5.0)
        .center_y()
        .fill(tok::theme(dark).card)
        .child(
            button("←", "back", dark, |state| state.history(false)).disabled(state.back.is_empty()),
        )
        .child(
            button("→", "forward", dark, |state| state.history(true))
                .disabled(state.forward.is_empty()),
        )
        .child(button("Overview", "overview", dark, State::overview))
        .child(button("Collapse", "collapse-all", dark, |state| {
            state.checkpoint();
            state.nav.expanded.clear();
            state.nav.focus = None;
            state.nav.show_path = false;
            state.nav.page = 0;
            state.auto_fit = true;
            state.changed();
        }))
}

fn grouping_tools(ui: &ViewContext<'_>, mut tools: El) -> El {
    let (state, dark) = (ui.state, ui.dark);
    for (label, grouping) in [
        ("Project", Grouping::Project),
        ("Namespace", Grouping::Namespace),
        ("Folder", Grouping::Folder),
    ] {
        tools = tools.child(
            button(label, format!("group-{label}"), dark, move |state| {
                state.checkpoint();
                state.nav.grouping = grouping;
                state.nav.expanded.clear();
                state.nav.focus = None;
                state.nav.show_path = false;
                state.nav.page = 0;
                state.auto_fit = true;
                state.changed();
            })
            .checked(state.nav.grouping == grouping)
            .fill(if state.nav.grouping == grouping {
                tok::accent(0, dark).alpha(0.22)
            } else {
                tok::theme(dark).control_off
            }),
        );
    }
    tools
}

fn zoom_tools(ui: &ViewContext<'_>, tools: El) -> El {
    let dark = ui.dark;
    tools
        .child(el().grow(1.0))
        .child(button("−", "zoom-out", dark, |state| {
            state
                .nav
                .camera
                .zoom_at([state.viewport[0] / 2.0, state.viewport[1] / 2.0], 0.8);
            state.dirty_session = true;
        }))
        .child(button("Fit", "fit", dark, |state| {
            state.nav.camera.fit(state.graph_view.size, state.viewport);
            state.dirty_session = true;
        }))
        .child(button("+", "zoom-in", dark, |state| {
            state
                .nav
                .camera
                .zoom_at([state.viewport[0] / 2.0, state.viewport[1] / 2.0], 1.25);
            state.dirty_session = true;
        }))
}

fn canvas_footer(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let mut footer = row()
        .w_full()
        .h(42.0)
        .px(12.0)
        .gap(8.0)
        .center_y()
        .fill(tok::theme(dark).card);
    for (label, direction) in [
        ("Callers", Direction::Incoming),
        ("Callees", Direction::Outgoing),
        ("Both", Direction::Both),
    ] {
        footer = footer.child(
            button(label, format!("direction-{label}"), dark, move |state| {
                state.checkpoint();
                state.nav.direction = direction;
                state.nav.show_path = false;
                state.changed();
            })
            .checked(state.nav.direction == direction)
            .fill(if state.nav.direction == direction {
                tok::accent(0, dark).alpha(0.22)
            } else {
                tok::theme(dark).control_off
            }),
        );
    }
    footer = footer.child(button(
        format!("Depth {}", state.nav.depth),
        "depth",
        dark,
        |state| {
            state.checkpoint();
            state.nav.depth = if state.nav.depth >= 5 {
                1
            } else {
                state.nav.depth + 1
            };
            state.changed();
        },
    ));
    if state.graph_view.pages > 1 {
        footer = footer.child(button(
            format!("Page {}/{} →", state.nav.page + 1, state.graph_view.pages),
            "next-page",
            dark,
            |state| {
                state.nav.page = (state.nav.page + 1) % state.graph_view.pages;
                state.auto_fit = true;
                state.changed();
            },
        ));
    }
    footer.child(el().grow(1.0)).child(
        text(format!("{:.0}%", state.nav.camera.zoom * 100.0))
            .font(tok::type_::LABEL)
            .color(tok::theme(dark).text_secondary),
    )
}

fn status_bar(ui: &ViewContext<'_>) -> El {
    let (state, dark) = (ui.state, ui.dark);
    let theme = tok::theme(dark);
    let status = if !state.warning.is_empty() {
        state.warning.as_str()
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
    let mut bar = row()
        .w_full()
        .h(40.0)
        .px(14.0)
        .gap(12.0)
        .center_y()
        .fill(theme.card)
        .child(
            text(status)
                .font(TextStyle::new(11.0, 400.0))
                .color(theme.text_secondary)
                .grow(1.0)
                .truncate(),
        )
        .child(
            text(counts)
                .w(650.0)
                .wrap()
                .max_lines(2)
                .font(TextStyle::new(10.0, 400.0))
                .color(theme.text_secondary),
        );
    if state.analyzing {
        bar = bar.child(button(
            "Cancel",
            "cancel-analysis",
            dark,
            State::cancel_analysis,
        ));
    }
    bar
}

pub fn key(state: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    if event.is_command('f') {
        state.query.focused = true;
        return true;
    }
    if event.is_command('o') {
        state.choose_folder(tx);
        return true;
    }
    match &event.key {
        Key::Left if event.mods.alt => state.history(false),
        Key::Right if event.mods.alt => state.history(true),
        Key::Escape => state.overview(),
        _ => return false,
    }
    true
}
