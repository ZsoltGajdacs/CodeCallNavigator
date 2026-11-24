//! Compare composes the change outline and the two revision panes.
mod changes;
mod class_canvas;
mod panes;
mod revisions;
mod snapshots;
mod source_diff;

use crate::{
    app::State,
    flow_comparison::{SideId, SnapshotSide},
    theme,
};
use rune::{Cx, El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub fn view(state: &State, cx: &mut Cx<State>) -> El {
    let k = theme::kit(cx.system_dark());
    let mut overlays = Vec::new();
    let body = col()
        .key("comparison-workspace")
        .on_layout(|s: &mut State, rect| {
            let size = [rect[2], rect[3]];
            s.flow_comparison.previews.interaction.window = size;
            s.source_popup.interaction.window = size;
        })
        .w_full()
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base)
        .child(toolbar(state, cx.titlebar_inset(), &k))
        .child(
            row()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .child(changes::view(state, &k))
                .child(el().w(1.0).h_full().fill(k.c.std_border))
                .child(panes::view(state, &k, &mut overlays)),
        );
    overlays.extend(source_diff::layer(state, cx, &k));
    overlays.extend(snapshots::layers(state, &k));
    overlays.extend(revisions::layers(state, &k));
    crate::view::with_overlays(state, &k, body, overlays)
}

fn toolbar(state: &State, inset: f32, k: &Kit) -> El {
    row()
        .key("comparison-toolbar")
        .w_full()
        .padding(8.0)
        .pl(8.0 + inset)
        .gap(8.0)
        .center_y()
        .child(
            Button::new(if state.flow_workspace.current.is_some() {
                "Back to function flow"
            } else {
                "Back to call graph"
            })
            .variant(ButtonVariant::Ghost)
            .on_press(|s: &mut State| s.flow_comparison.close())
            .build(k)
            .key("comparison-close"),
        )
        .child(
            text("Compare flows")
                .font(k.ty.ui_medium)
                .color(k.c.text_base),
        )
}

fn endpoint_header(state: &State, side: SideId, k: &Kit) -> El {
    let (label, side) = match side {
        SideId::Before => ("Before", SnapshotSide::Base),
        SideId::After => ("After", SnapshotSide::Head),
    };
    col()
        .w_full()
        .min_w(0.0)
        .padding(10.0)
        .gap(6.0)
        .fill(k.c.bg_base)
        .child(text(label).font(k.ty.ui_medium).color(k.c.text_base))
        .child(revisions::picker_button(state, side, k))
        .child(snapshots::view(state, side, k))
}

pub fn key(state: &mut State, event: &rune::KeyEvent) -> bool {
    if event.key != rune::Key::Escape {
        return false;
    }
    let workspace = &mut state.flow_comparison;
    if workspace.controls.class_depth_open.take().is_some() {
        return true;
    }
    if workspace.revisions.picker.is_some() {
        workspace.revisions.close_picker();
    } else if workspace.snapshots.variant_picker.take().is_some() {
        return true;
    } else if workspace.previews.active.is_some() {
        workspace.previews.dismiss();
    } else {
        workspace.close();
    }
    true
}
