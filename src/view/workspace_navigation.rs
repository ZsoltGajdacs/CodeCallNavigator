//! Navigation chrome consumes existing graph actions without owning graph state.
use crate::{app::State, icons};
use rune::{El, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit};

pub(super) fn view(state: &State, k: &Kit) -> El {
    row()
        .key("graph-toolbar")
        .center_y()
        .gap(4.0)
        .min_w(0.0)
        .child(history(state, false, k))
        .child(history(state, true, k))
        .child(breadcrumb(state, k).grow(1.0).shrink())
        .child(action("Overview", "overview", State::overview, k))
        .child(super::comparison_button(k))
        .child(action(
            "Collapse all",
            "collapse-all",
            State::collapse_all,
            k,
        ))
}

fn history(state: &State, forward: bool, k: &Kit) -> El {
    IconButton::new(if forward {
        &icons::OUTLINE_ARROW_RIGHT
    } else {
        &icons::OUTLINE_ARROW_LEFT
    })
    .label(if forward { "Forward" } else { "Back" })
    .help(if forward { "Forward" } else { "Back" })
    .variant(IconButtonVariant::GhostMuted)
    .control_size(ControlSize::ExtraLarge)
    .disabled(if forward {
        state.forward.is_empty()
    } else {
        state.back.is_empty()
    })
    .on_press(move |state: &mut State| state.history(forward))
    .build(k)
    .key(if forward { "forward" } else { "back" })
}

fn breadcrumb(state: &State, k: &Kit) -> El {
    let selected = state
        .selected()
        .and_then(|id| state.graph.as_ref()?.data.nodes.get(id as usize));
    let mut path = row()
        .key("workspace-breadcrumb")
        .center_y()
        .gap(10.0)
        .min_w(0.0);
    if let Some(node) = selected {
        path = path
            .child(
                text(&node.container)
                    .font(k.ty.ui)
                    .color(k.c.text_muted)
                    .truncate()
                    .max_w(160.0)
                    .shrink(),
            )
            .child(text("/").font(k.ty.ui).color(k.c.text_faint))
            .child(
                text(&node.label)
                    .font(k.ty.ui_medium)
                    .color(k.c.text_base)
                    .truncate()
                    .max_w(200.0)
                    .shrink(),
            );
    } else {
        path = path.child(
            text("Overview")
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .truncate(),
        );
    }
    path
}

fn action(label: &str, key: &'static str, on_press: fn(&mut State), k: &Kit) -> El {
    Button::new(label)
        .variant(ButtonVariant::Ghost)
        .on_press(on_press)
        .build(k)
        .key(key)
}
