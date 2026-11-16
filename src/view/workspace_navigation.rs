//! Navigation chrome consumes existing graph actions without owning graph state.
use crate::{app::State, icons, presentation::Overlay};
use rune::{El, col, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, Placement, Popover,
};

pub(super) fn view(state: &State, k: &Kit, layers: &mut Vec<El>) -> El {
    row()
        .key("graph-toolbar")
        .center_y()
        .gap(4.0)
        .min_w(0.0)
        .child(history(state, false, k))
        .child(history(state, true, k))
        .child(breadcrumb(state, k))
        .child(overflow(state, k, layers))
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
                    .max_w(160.0),
            )
            .child(text("/").font(k.ty.ui).color(k.c.text_faint))
            .child(
                text(&node.label)
                    .font(k.ty.ui_medium)
                    .color(k.c.text_base)
                    .truncate()
                    .max_w(200.0),
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

fn overflow(state: &State, k: &Kit, layers: &mut Vec<El>) -> El {
    let trigger = Button::new("More")
        .variant(ButtonVariant::GhostMuted)
        .control_size(ControlSize::ExtraLarge)
        .expanded(state.ui.overlay == Some(Overlay::Navigation))
        .on_press(|state: &mut State| {
            state.ui.overlay = if state.ui.overlay == Some(Overlay::Navigation) {
                None
            } else {
                Some(Overlay::Navigation)
            };
        })
        .build(k)
        .key("navigation-more")
        .help("Graph navigation actions");
    let items = col()
        .w(180.0)
        .gap(6.0)
        .child(
            Button::new("Overview")
                .variant(ButtonVariant::Ghost)
                .on_press(|state: &mut State| {
                    state.close_overlay();
                    state.overview();
                })
                .build(k)
                .key("overview"),
        )
        .child(
            Button::new("Collapse all")
                .variant(ButtonVariant::Ghost)
                .on_press(|state: &mut State| {
                    state.close_overlay();
                    state.collapse_all();
                })
                .build(k)
                .key("collapse-all"),
        );
    Popover::new("navigation-popover")
        .label("Graph navigation actions")
        .trigger(trigger)
        .open(state.ui.overlay == Some(Overlay::Navigation))
        .placement(Placement::BottomStart)
        .on_close(State::close_overlay)
        .child(items)
        .build(k, layers)
}
