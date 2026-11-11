//! Repository discovery drawer.
use crate::{app::State, theme};
use rune::{El, col, text};
use rune_kit::{Button, Checkbox, Kit, ScrollBar, ScrollBarStyle, TextInput};

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
                        .grow(1.0)
                        .min_h(0.0),
                ),
        )
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

fn note(value: &str, k: &Kit) -> El {
    text(value)
        .font(k.ty.text_xsmall)
        .color(k.c.text_muted)
        .w_full()
        .wrap_anywhere()
}
