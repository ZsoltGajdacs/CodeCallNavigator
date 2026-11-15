//! Revision controls and saved endpoint flows; all blocking work belongs to the controller.
mod catalog;
mod panes;
mod provenance;
use crate::{app::State, theme};
use rune::{Cx, El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, TextInput};
pub fn view(state: &State, cx: &mut Cx<State>) -> El {
    let k = theme::kit(cx.system_dark());
    let body = col()
        .key("comparison-workspace")
        .w_full()
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base)
        .child(toolbar(state, cx.titlebar_inset(), &k))
        .child(revisions(state, &k))
        .child(provenance::view(state, &k))
        .child(status(state, &k))
        .child(counterpart(state, &k))
        .child(panes::view(state, &k));
    crate::view::with_overlays(state, &k, body, catalog::layers(state, &k))
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
            .variant(ButtonVariant::Outline)
            .on_press(|s: &mut State| s.flow_comparison.close())
            .build(k)
            .key("comparison-close"),
        )
        .child(
            text("Compare flows")
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .grow(1.0),
        )
        .child(
            Button::new("Functions")
                .disabled(state.flow_comparison.catalog.is_none())
                .on_press(|s: &mut State| s.flow_comparison.controls.catalog_open = true)
                .build(k)
                .key("comparison-catalog-open"),
        )
        .child(
            Button::new("Previous function")
                .disabled(state.flow_comparison.history.is_empty())
                .on_press(|s: &mut State| {
                    s.flow_comparison.back();
                })
                .build(k)
                .key("comparison-back"),
        )
        .child(fit_button(state, crate::flow_comparison::SideId::Before, k))
        .child(fit_button(state, crate::flow_comparison::SideId::After, k))
}
fn fit_button(state: &State, side: crate::flow_comparison::SideId, k: &Kit) -> El {
    let available = state
        .flow_comparison
        .current
        .as_ref()
        .is_some_and(|pair| matches!(pair.side(side), crate::flow_comparison::Side::Ready(_)));
    let (label, key) = match side {
        crate::flow_comparison::SideId::Before => ("Fit before", "comparison-fit-before"),
        crate::flow_comparison::SideId::After => ("Fit after", "comparison-fit-after"),
    };
    Button::new(label)
        .disabled(!available)
        .on_press(move |state: &mut State| state.flow_comparison.fit(side))
        .build(k)
        .key(key)
}
fn revisions(state: &State, k: &Kit) -> El {
    let controls = &state.flow_comparison.controls;
    row()
        .w_full()
        .px(10.0)
        .pb(8.0)
        .gap(8.0)
        .center_y()
        .child(text("Base").font(k.ty.small).color(k.c.text_muted))
        .child(
            TextInput::new(&controls.base)
                .width(None)
                .label("Base local revision")
                .on_edit(|s: &mut State| &mut s.flow_comparison.controls.base)
                .build(k)
                .key("comparison-base")
                .grow(1.0)
                .min_w(80.0),
        )
        .child(text("After").font(k.ty.small).color(k.c.text_muted))
        .child(
            TextInput::new(&controls.head)
                .width(None)
                .label("After local revision")
                .on_edit(|s: &mut State| &mut s.flow_comparison.controls.head)
                .build(k)
                .key("comparison-head")
                .grow(1.0)
                .min_w(80.0),
        )
        .child(
            Button::new("Load")
                .disabled(state.runtime.is_none())
                .on_press(State::load_flow_comparison)
                .build(k)
                .key("comparison-load"),
        )
        .child(
            Button::new("Reload")
                .disabled(state.flow_comparison.descriptor().is_none())
                .on_press(|s: &mut State| s.flow_comparison.reload_control())
                .build(k)
                .key("comparison-reload"),
        )
        .child(
            Button::new("Cancel")
                .disabled(!state.flow_comparison.pending())
                .on_press(|s: &mut State| s.flow_comparison.cancel_control())
                .build(k)
                .key("comparison-cancel"),
        )
}
fn counterpart(state: &State, k: &Kit) -> El {
    let pair = state.flow_comparison.current.as_ref();
    let manual = pair.is_some_and(|pair| {
        pair.correspondence.kind
            == crate::engine_client::flow_comparison::CorrespondenceKind::Manual
    });
    row()
        .key("comparison-counterpart")
        .w_full()
        .px(10.0)
        .py(3.0)
        .gap(8.0)
        .center_y()
        .child(
            text(if manual {
                "Manual counterpart · identities selected explicitly"
            } else {
                "Exact identity correspondence"
            })
            .key("comparison-correspondence")
            .font(k.ty.small)
            .color(if manual {
                k.c.warning.fg
            } else {
                k.c.text_muted
            })
            .grow(1.0)
            .min_w(0.0)
            .truncate(),
        )
        .child(
            Button::new("Choose after counterpart")
                .disabled(pair.is_none())
                .on_press(|s: &mut State| {
                    s.flow_comparison.controls.choosing_after = true;
                    s.flow_comparison.controls.catalog_open = true;
                })
                .build(k)
                .key("comparison-choose-after"),
        )
        .child(
            Button::new("Reset counterpart")
                .disabled(!manual)
                .on_press(|s: &mut State| s.flow_comparison.reset_control())
                .build(k)
                .key("comparison-reset-counterpart"),
        )
}
fn status(state: &State, k: &Kit) -> El {
    let workspace = &state.flow_comparison;
    let message = if !workspace.controls.message.is_empty() {
        workspace.controls.message.as_str()
    } else {
        match &workspace.phase {
            crate::flow_comparison::Phase::Closed => {
                "Load saved artifacts from local Git revisions"
            }
            crate::flow_comparison::Phase::Loading => "Loading saved revision pair…",
            crate::flow_comparison::Phase::Bound => "Saved revision pair loaded",
            crate::flow_comparison::Phase::ReloadRequired(message)
            | crate::flow_comparison::Phase::Failed(message) => message,
        }
    };
    let partial = provenance::partial(state);
    let message = partial.as_deref().unwrap_or(message);
    text(message)
        .key("comparison-status")
        .font(k.ty.small)
        .color(k.c.text_muted)
        .w_full()
        .px(10.0)
        .truncate()
        .help(message)
}
pub fn key(state: &mut State, event: &rune::KeyEvent) -> bool {
    if event.key == rune::Key::Escape {
        if state.flow_comparison.controls.catalog_open {
            state.flow_comparison.controls.catalog_open = false;
        } else {
            state.flow_comparison.close();
        }
        true
    } else {
        false
    }
}
