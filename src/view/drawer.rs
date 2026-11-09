//! Repository and paginated symbol search drawer content.
use crate::{app::State, presentation::Drawer, theme};
use rune::{El, ScrollSpec, col, text};
use rune_kit::{Button, Checkbox, Kit, List, ListItem, ScrollBar, ScrollBarStyle, TextInput};

pub fn view(state: &State, k: &Kit) -> El {
    let (title, content) = match state.ui.drawer {
        Drawer::Repository => ("Repository", repository(state, k)),
        Drawer::Search => ("Search", search(state, k)),
    };
    let content = content
        .w(state.ui.sidebar.width() - 2.0 * theme::SPACE)
        .gap(theme::SPACE);
    let body = col().key("drawer-scroll").w_full().grow(1.0).min_h(0.0);
    let body = if state.ui.drawer == Drawer::Repository {
        body.padding(theme::SPACE)
            .child(content.grow(1.0).min_h(0.0))
    } else {
        body.scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(col().w_full().padding(theme::SPACE).child(content))
    };
    col()
        .key("drawer")
        .w_full()
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base)
        .child(
            text(title)
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .px(theme::SPACE)
                .h(40.0),
        )
        .child(body)
}

fn repository(state: &State, k: &Kit) -> El {
    let mut content = col()
        .child(
            Button::new("Choose folder…")
                .on_press_tx(|s: &mut State, tx| s.choose_folder(tx))
                .build(k)
                .key("choose-folder")
                .w_full(),
        )
        .child(
            TextInput::new(&state.root)
                .width(None)
                .label("Project folder")
                .placeholder("Project folder")
                .on_edit(|s: &mut State| &mut s.root)
                .on_change(|s: &mut State, _| s.root_edited())
                .on_submit(|s: &mut State, _| s.discover_root())
                .build(k)
                .key("root-path"),
        )
        .child(
            Button::new("Find builds")
                .on_press(State::discover_root)
                .build(k)
                .key("find-builds")
                .w_full(),
        )
        .child(text("Builds").font(k.ty.ui_medium).color(k.c.text_base));
    if state.targets.is_empty() {
        content = content.child(note(
            if state.jobs.discovery.is_pending() {
                "Looking for build files…"
            } else {
                "Solutions, C# projects, Java builds and TypeScript configurations appear here."
            },
            k,
        ));
    }
    let mut builds = col().w_full().gap(theme::SPACE);
    for (index, choice) in state.targets.iter().enumerate() {
        builds = builds.child(
            Checkbox::new(choice.selected)
                .label(&choice.target.label)
                .on_toggle(move |s: &mut State| s.toggle_build(index))
                .build(k)
                .key(format!("build-{index}"))
                .w_full(),
        );
    }
    if !state.targets.is_empty() {
        content = content.child(
            col()
                .key("builds-scroll")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll_xy()
                .child(builds)
                .scroll_bar_styled(ScrollBarStyle::from_kit(k).horizontal())
                .pb(8.0),
        );
    }
    content.child(note(analysis_help(state), k))
}

fn analysis_help(state: &State) -> &'static str {
    if state.jobs.boot.is_pending() {
        "Opening the saved workspace…"
    } else if state.analyzing {
        "Analyzing selected builds; the previous graph remains available."
    } else if state.targets.iter().all(|t| !t.selected) {
        "Select one or more builds, then generate or refresh the graph."
    } else {
        "Analysis runs locally and may restore dependencies or compile project code."
    }
}

fn search(state: &State, k: &Kit) -> El {
    let mut content = col()
        .child(
            TextInput::new(&state.query)
                .width(None)
                .label("Search all symbols")
                .placeholder("Search all symbols…")
                .leading_icon(k.glyph.search)
                .on_edit(|s: &mut State| &mut s.query)
                .build(k)
                .key("search"),
        )
        .child(search_pager(state, k));
    if state.jobs.search.is_pending() {
        content = content.child(note("Searching…", k));
    } else if state.hits.is_empty() {
        content = content.child(note(
            if state.query.text.is_empty() {
                "Search the complete index, including symbols hidden by graph filters."
            } else {
                "No matching symbols"
            },
            k,
        ));
    } else {
        content = content.child(search_results(state, k));
    }
    content
}

fn search_results(state: &State, k: &Kit) -> El {
    let mut results = List::new().active(None).divider();
    let Some(graph) = &state.graph else {
        return results.build(k);
    };
    for &index in &state.hits {
        let node = &graph.data.nodes[index as usize];
        let suffix = if state.nav.filter.accepts_node(node) {
            ""
        } else {
            " · hidden, click to reveal"
        };
        results = results.item(
            ListItem::new(format!("{}{suffix}", node.signature))
                .selected(state.selected() == Some(index))
                .on_select(move |s: &mut State, _| s.reveal(index)),
        );
    }
    results.build(k).key("search-results")
}

fn search_pager(state: &State, k: &Kit) -> El {
    rune::row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            Button::new("Previous")
                .disabled(state.search_page == 0)
                .on_press(|s: &mut State| s.step_search(false))
                .build(k)
                .key("search-prev"),
        )
        .child(
            text(format!("{}", state.search_page + 1))
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            Button::new("Next")
                .disabled(!state.search_more)
                .on_press(|s: &mut State| s.step_search(true))
                .build(k)
                .key("search-next"),
        )
}

fn note(value: &str, k: &Kit) -> El {
    text(value)
        .font(k.ty.text_xsmall)
        .color(k.c.text_muted)
        .w_full()
        .wrap_anywhere()
}
