//! Revision controls and saved endpoint flows; all blocking work belongs to the controller.
mod catalog;
mod changes;
mod class_canvas;
mod impact;
mod panes;
mod provenance;
mod revisions;
mod snapshots;
mod source_diff;
use crate::{app::State, flow_comparison::Panel, theme};
use rune::{Cx, El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit};
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
        .child(snapshots::controls(state, &k));
    let body = match state.flow_comparison.controls.panel {
        Panel::Graph => body.child(snapshots::graph(state, &k)),
        Panel::Impact => body.child(impact::view(state, &k)),
        Panel::Flows => body
            .child(provenance::view(state, &k))
            .child(status(state, &k))
            .child(if state.flow_comparison.changes.show_outline() {
                changes::view(state, &k)
            } else {
                counterpart(state, &k)
            })
            .child(changes::divider(state, &k))
            .child(panes::view(state, &k)),
    };
    let mut overlays = Vec::new();
    if state.flow_comparison.controls.panel == Panel::Flows {
        overlays.extend(source_diff::layer(state, &k));
    }
    overlays.extend(catalog::layers(state, &k));
    overlays.extend(revisions::layers(state, &k));
    crate::view::with_overlays(state, &k, body, overlays)
}
fn toolbar(state: &State, inset: f32, k: &Kit) -> El {
    let panel = state.flow_comparison.controls.panel;
    let toolbar = row()
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
            text(match panel {
                Panel::Flows => "Compare flows",
                Panel::Graph => "Compare graphs",
                Panel::Impact => "Changed-symbol impact",
            })
            .font(k.ty.ui_medium)
            .color(k.c.text_base)
            .grow(1.0),
        );
    if panel != Panel::Flows {
        return toolbar;
    }
    flow_actions(state, toolbar, k)
}
fn flow_actions(state: &State, toolbar: El, k: &Kit) -> El {
    let toolbar = if state.flow_comparison.changes.loaded && state.flow_comparison.changes.expert {
        toolbar.child(
            Button::new("Changes")
                .on_press(|s: &mut State| {
                    s.flow_comparison.changes.expert = false;
                    s.flow_comparison.controls.catalog_open = false;
                })
                .build(k)
                .key("comparison-show-changes"),
        )
    } else {
        toolbar
    };
    toolbar
        .child(
            Button::new("Functions")
                .disabled(state.flow_comparison.catalog.is_none())
                .on_press(|s: &mut State| {
                    s.flow_comparison.previews.close();
                    s.flow_comparison.changes.expert = true;
                    s.flow_comparison.controls.catalog_open = true;
                })
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
    let (label, key) = match side {
        crate::flow_comparison::SideId::Before => ("Fit before", "comparison-fit-before"),
        crate::flow_comparison::SideId::After => ("Fit after", "comparison-fit-after"),
    };
    Button::new(label)
        .disabled(!state.flow_comparison.can_fit(side))
        .on_press(move |state: &mut State| state.flow_comparison.fit(side))
        .build(k)
        .key(key)
}
fn revisions(state: &State, k: &Kit) -> El {
    let controls = &state.flow_comparison.controls;
    let snapshots = &state.flow_comparison.snapshots;
    row()
        .w_full()
        .px(10.0)
        .pb(8.0)
        .gap(8.0)
        .center_y()
        .child(revisions::picker_button(
            state,
            crate::flow_comparison::SnapshotSide::Base,
            k,
        ))
        .child(revisions::picker_button(
            state,
            crate::flow_comparison::SnapshotSide::Head,
            k,
        ))
        .child(
            Button::new(if controls.panel == Panel::Flows {
                "Load"
            } else {
                "Compare flows"
            })
            .disabled(
                state.runtime.is_none()
                    || snapshots.pending()
                    || state.flow_comparison.revisions.base().is_none()
                    || state.flow_comparison.worktree_refresh.pending(),
            )
            .on_press(State::load_flow_comparison)
            .build(k)
            .key("comparison-load"),
        )
        .child(reload_button(state, k))
        .child(
            Button::new("Cancel")
                .disabled(
                    !state.flow_comparison.comparison_pending()
                        && !state.flow_comparison.worktree_refresh.pending(),
                )
                .on_press(State::cancel_comparison_action)
                .build(k)
                .key("comparison-cancel"),
        )
}
fn reload_button(state: &State, k: &Kit) -> El {
    let workspace = &state.flow_comparison;
    let snapshots = &workspace.snapshots;
    let can_reload = match workspace.controls.panel {
        Panel::Flows => workspace.descriptor().is_some(),
        Panel::Graph => snapshots.can_reload_graph(),
        Panel::Impact => workspace.can_preview_impact(),
    };
    Button::new("Reload")
        .disabled(snapshots.pending() || workspace.worktree_refresh.pending() || !can_reload)
        .on_press(
            |state: &mut State| match state.flow_comparison.controls.panel {
                Panel::Flows => state.flow_comparison.reload_control(),
                Panel::Graph => {
                    state.compare_snapshot_graphs(crate::flow_comparison::GraphNavigation::Reload)
                }
                Panel::Impact => state.preview_changed_symbol_impact(),
            },
        )
        .build(k)
        .key("comparison-reload")
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
        if state.flow_comparison.revisions.picker.is_some() {
            state.flow_comparison.revisions.close_picker();
            return true;
        } else if state.flow_comparison.previews.active.is_some() {
            state.flow_comparison.previews.dismiss();
            return true;
        } else if state.flow_comparison.controls.catalog_open {
            state.flow_comparison.controls.catalog_open = false;
        } else if state.flow_comparison.controls.panel == Panel::Graph
            && state
                .flow_comparison
                .snapshots
                .selected_change
                .take()
                .is_some()
        {
            return true;
        } else {
            state.flow_comparison.close();
        }
        true
    } else {
        false
    }
}
