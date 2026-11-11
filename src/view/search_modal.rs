//! Search presentation only; lifecycle and matching have separate owners.
use crate::{app::State, search::Results};
use codegraph::search::SearchCategory;
use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, IconButton, IconButtonVariant, Kit, TextInput};

pub(super) fn button(k: &Kit) -> El {
    IconButton::new(k.glyph.search)
        .label("Search")
        .help("Search classes and methods · Double Shift")
        .variant(IconButtonVariant::GhostMuted)
        .on_press(State::open_search)
        .build(k)
        .key("open-search")
}

pub(super) fn layer(state: &State, k: &Kit, layers: &mut Vec<El>) {
    if !state.search.open {
        return;
    }
    let panel = col()
        .key("search-dialog")
        .role("dialog")
        .label("Search classes and methods")
        .w_full()
        .max_w(760.0)
        .h((state.search_window_height() - 120.0).clamp(240.0, 540.0))
        .padding(16.0)
        .gap(10.0)
        .fill(k.c.layer_01)
        .radius(8.0)
        .child(header(k))
        .child(tabs(state, k))
        .child(input(state, k))
        .child(
            col()
                .key("search-results-scroll")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .child(results(state, k)),
        )
        .child(pager(state, k))
        .modal(true, State::close_search)
        .modal_click_outside(true);
    layers.push(
        el().key("search-scrim")
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .fill(k.c.scrim)
            .a11y_hidden(),
    );
    layers.push(
        col()
            .key("search-layer")
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
            .px(24.0)
            .pt(64.0)
            .items_center()
            .child(panel),
    );
}

fn input(state: &State, k: &Kit) -> El {
    TextInput::new(&state.search.input)
        .width(None)
        .label("Search classes and methods")
        .placeholder("Search classes and methods…")
        .leading_icon(k.glyph.search)
        .on_edit(|s: &mut State| &mut s.search.input)
        .on_change(|s: &mut State, _| s.request_search())
        .on_key(|s: &mut State, e, tx| {
            !matches!(e.key, rune::Key::Modifier { .. }) && crate::search::key(s, e, tx)
        })
        .build(k)
        .key("search")
}

fn tabs(state: &State, k: &Kit) -> El {
    let mut tabs = row().w_full().gap(8.0);
    for (category, title, key) in [
        (SearchCategory::All, "All", "search-all"),
        (SearchCategory::Classes, "Classes", "search-classes"),
        (SearchCategory::Methods, "Methods", "search-methods"),
    ] {
        tabs = tabs.child(
            Button::new(title)
                .variant(if state.search.category == category {
                    ButtonVariant::Outline
                } else {
                    ButtonVariant::Ghost
                })
                .on_press(move |s: &mut State| s.set_search_category(category))
                .build(k)
                .preserve_focus_on_press(true)
                .key(key),
        );
    }
    tabs
}

fn header(k: &Kit) -> El {
    row()
        .w_full()
        .center_y()
        .gap(12.0)
        .child(
            text("Search")
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .grow(1.0),
        )
        .child(text("Double Shift").font(k.ty.small).color(k.c.text_muted))
        .child(
            IconButton::new(k.glyph.close)
                .label("Close search")
                .variant(IconButtonVariant::Ghost)
                .on_press(State::close_search)
                .build(k)
                .key("close-search"),
        )
}

fn results(state: &State, k: &Kit) -> El {
    if let Some(note) = status(state) {
        return text(note)
            .font(k.ty.small)
            .color(k.c.text_muted)
            .w_full()
            .wrap();
    }
    let mut list = col()
        .key("search-results")
        .role("listbox")
        .w_full()
        .gap(4.0);
    for (index, hit) in state.search.hits().iter().enumerate() {
        list = list.child(result_row(hit, index, state.search.selected == index, k));
    }
    list
}

fn status(state: &State) -> Option<&str> {
    match &state.search.results {
        Results::Idle => Some(if state.graph.is_some() {
            "Search the complete graph, including hidden classes and methods."
        } else {
            "Analyze a project to search classes and methods."
        }),
        Results::Loading => Some("Searching…"),
        Results::Failed(error) => Some(error.as_str()),
        Results::Ready(page) if page.hits.is_empty() => Some("No matching classes or methods"),
        _ => None,
    }
}

fn result_row(hit: &codegraph::search::SearchHit, index: usize, selected: bool, k: &Kit) -> El {
    let label = format!("{} · {}", hit.kind_label(), hit.name);

    col()
        .key(format!("search-result-{index}"))
        .role("option")
        .focus_selected(selected)
        .label(&label)
        .description(&hit.detail)
        .w_full()
        .padding(10.0)
        .gap(3.0)
        .radius(4.0)
        .fill(if selected { k.c.bg_deep } else { k.c.layer_01 })
        .on_press(move |s: &mut State| s.activate_search(index))
        .child(
            text(label)
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .w_full()
                .wrap(),
        )
        .child(
            text(&hit.detail)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w_full()
                .wrap_anywhere(),
        )
}

fn pager(state: &State, k: &Kit) -> El {
    row()
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            Button::new("Previous")
                .disabled(state.search.page == 0)
                .on_press(|s: &mut State| s.step_search(false))
                .build(k)
                .key("search-prev"),
        )
        .child(
            text(format!("{}", state.search.page + 1))
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            Button::new("Next")
                .disabled(!state.search.more())
                .on_press(|s: &mut State| s.step_search(true))
                .build(k)
                .key("search-next"),
        )
        .child(
            text("↑ ↓ select · Enter open · Esc close")
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
}
