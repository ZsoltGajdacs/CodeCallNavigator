//! Build picker presentation; discovery and selection lifecycle belong to build_discovery.
use crate::{app::State, build_discovery::Phase, presentation::Overlay, theme};
use rune::{Align, El, Side, col, row, text};
use rune_kit::{Button, Checkbox, IconButton, IconButtonVariant, Kit, ScrollBar, popover};

pub(super) fn button(state: &State, k: &Kit) -> El {
    let open = state.ui.overlay == Some(Overlay::Builds);
    let button = Button::new("Find builds")
        .on_press(State::open_builds)
        .build(k)
        .key("find-builds")
        .expanded(open)
        .w_full();
    if open {
        button
    } else {
        button.help(selected_summary(state))
    }
}

fn selected_summary(state: &State) -> String {
    let names: Vec<_> = state
        .targets
        .iter()
        .filter(|choice| choice.selected)
        .map(|choice| choice.target.label.as_str())
        .collect();
    if names.is_empty() {
        "No builds selected".into()
    } else {
        format!("Selected builds:\n{}", names.join("\n"))
    }
}

pub(super) fn layer(state: &State, k: &Kit, layers: &mut Vec<El>) {
    if state.ui.overlay != Some(Overlay::Builds) || !state.ui.sidebar.is_open() {
        return;
    }
    let size = state.source_popup.window;
    let width = 420.0_f32.min((size[0] - 16.0).max(1.0));
    let height = 480.0_f32.min((size[1] - 16.0).max(1.0));
    let panel = col()
        .key("build-picker")
        .role("dialog")
        .label("Builds")
        .w(width)
        .max_h(height)
        .min_h(0.0)
        .padding(12.0)
        .gap(theme::SPACE)
        .child(
            row()
                .w_full()
                .center_y()
                .justify_between()
                .child(text("Builds").font(k.ty.ui_medium).color(k.c.text_base))
                .child(
                    IconButton::new(k.glyph.close)
                        .variant(IconButtonVariant::Ghost)
                        .label("Close builds")
                        .on_press(State::close_overlay)
                        .build(k)
                        .key("close-builds"),
                ),
        )
        .child(
            col()
                .key("builds-scroll")
                .w_full()
                .shrink()
                .min_h(0.0)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .child(contents(state, k))
                .scroll_bar(k),
        )
        .focus_return("find-builds")
        .modal(true, State::close_overlay)
        .anchored_fit("find-builds", Side::Right, Align::Start, 8.0);
    layers.push(popover::raised(k, panel, k.radius.md));
}

fn contents(state: &State, k: &Kit) -> El {
    let message = match &state.build_discovery.phase {
        Phase::Loading => Some("Looking for build files…"),
        Phase::Failed(error) => Some(error.as_str()),
        _ if state.targets.is_empty() => Some(
            "No builds found. Choose a folder containing a solution, project, Java build or TypeScript configuration.",
        ),
        _ => None,
    };
    if let Some(message) = message {
        return text(message)
            .key("build-picker-message")
            .w_full()
            .wrap_anywhere()
            .font(k.ty.text_small)
            .color(k.c.text_muted);
    }
    col()
        .w_full()
        .gap(theme::SPACE)
        .children(state.targets.iter().enumerate().map(|(index, choice)| {
            Checkbox::new(choice.selected)
                .label(&choice.target.label)
                .hide_label()
                .on_toggle(move |s: &mut State| s.toggle_build(index))
                .build(k)
                .key(format!("build-{index}"))
                .w_full()
                .child(
                    text(&choice.target.label)
                        .a11y_hidden()
                        .font(k.ty.text_small)
                        .color(k.c.text_base)
                        .min_w(0.0)
                        .grow(1.0)
                        .wrap_anywhere(),
                )
        }))
}
