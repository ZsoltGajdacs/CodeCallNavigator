//! Repository discovery and project explorer drawer.
use super::{build_picker, explorer};
use crate::{app::State, theme};
use rune::{El, col, text};
use rune_kit::{Button, Kit, TextInput};

pub fn view(state: &State, k: &Kit) -> El {
    col()
        .key("drawer")
        .w_full()
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base)
        .child(
            text("Repository")
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .px(theme::SPACE)
                .h(40.0),
        )
        .child(
            col()
                .key("drawer-scroll")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .padding(theme::SPACE)
                .child(
                    repository(state, k)
                        .w(state.ui.sidebar.width() - 2.0 * theme::SPACE)
                        .gap(theme::SPACE)
                        .min_h(0.0),
                )
                .child(explorer::view(state, k).w(state.ui.sidebar.width() - 2.0 * theme::SPACE)),
        )
}

fn repository(state: &State, k: &Kit) -> El {
    col()
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
        .child(build_picker::button(state, k))
        .child(note(analysis_help(state), k))
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

fn note(value: &str, k: &Kit) -> El {
    text(value)
        .font(k.ty.text_xsmall)
        .color(k.c.text_muted)
        .w_full()
        .wrap_anywhere()
}
