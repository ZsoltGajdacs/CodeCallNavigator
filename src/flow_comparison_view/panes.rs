//! Two independent charts and one explicitly selected source/details endpoint.
use crate::{
    app::State,
    engine_client::flow_comparison::Symbol,
    flow_comparison::{Side, SideId},
    flow_view::{self, address::Address},
    projection::labels::{self, Label},
};
use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};
pub(super) fn view(state: &State, k: &Kit, layer: &mut Vec<El>) -> El {
    let workspace = &state.flow_comparison;
    let show_methods_source = workspace.class_selection().is_none()
        && (!workspace.changes.loaded || workspace.changes.expert);
    col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(if workspace.class_selection().is_some() {
            class_depth(state, k, layer)
        } else {
            el()
        })
        .child(
            row()
                .key("comparison-charts")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .child(chart(state, SideId::Before, k))
                .child(el().w(1.0).h_full().fill(k.c.std_border))
                .child(chart(state, SideId::After, k)),
        )
        .child(if show_methods_source {
            bottom(state, k)
        } else {
            el()
        })
}
fn class_depth(state: &State, k: &Kit, layer: &mut Vec<El>) -> El {
    let workspace = &state.flow_comparison;
    let select = crate::view::depth_select(
        "comparison-depth",
        workspace.class_depth(),
        workspace.maximum_class_depth(),
    )
    .open(workspace.controls.class_depth_open)
    .on_toggle(|s: &mut State| {
        s.flow_comparison.previews.dismiss();
        s.flow_comparison.controls.class_depth_open = !s.flow_comparison.controls.class_depth_open;
    })
    .on_close(|s: &mut State| s.flow_comparison.controls.class_depth_open = false)
    .on_choose(|s: &mut State, value, _| {
        if let Ok(depth) = value.parse() {
            s.flow_comparison.set_class_depth(depth);
        }
    })
    .build(k, layer);
    row()
        .w_full()
        .px(8.0)
        .gap(8.0)
        .center_y()
        .child(text("Depth").font(k.ty.small).color(k.c.text_muted))
        .child(select)
}

fn address(side: SideId) -> Address {
    match side {
        SideId::Before => Address::Before,
        SideId::After => Address::After,
    }
}
fn title(side: SideId) -> &'static str {
    match side {
        SideId::Before => "Before",
        SideId::After => "After",
    }
}
fn chart(state: &State, side: SideId, k: &Kit) -> El {
    let address = address(side);
    let content = col()
        .key(address.key("pane"))
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .min_h(0.0);
    if state.flow_comparison.class_selection().is_some() {
        return super::class_canvas::pane(content, state, side, k);
    }
    let Some(pair) = &state.flow_comparison.current else {
        return content.child(heading(title(side), None, k)).child(empty(
            address,
            if state.flow_comparison.detail_pending() {
                "Building before and after charts…"
            } else if state.flow_comparison.changes.loaded {
                "Select a changed class or method"
            } else {
                "Choose a saved function identity"
            },
            k,
        ));
    };
    let side_data = pair.side(side);
    let label = method_label(side_data);
    let content = content.child(heading(title(side), label.as_ref(), k));
    match side_data {
        Side::Ready(ready) => content
            .child(
                if ready
                    .session
                    .ready
                    .as_ref()
                    .is_some_and(|ready| ready.chart.partial)
                {
                    text("Partial flow · see diagnostics")
                        .key(address.key("partial"))
                        .font(k.ty.small)
                        .color(k.c.warning.fg)
                        .px(8.0)
                } else {
                    el()
                },
            )
            .child(flow_view::chart::view(state, &ready.session, address, k)),
        other => content.child(empty(address, &explanation(other), k)),
    }
}

fn method_label(side: &Side) -> Option<Label> {
    let node: &Symbol = match side {
        Side::Ready(ready) => &ready.node,
        Side::Unavailable(unavailable) => unavailable.node.as_ref()?,
        Side::Missing => return None,
    };
    Some(labels::named(&node.label, &node.signature, node.visibility))
}

pub(super) fn heading(title: &str, label: Option<&Label>, k: &Kit) -> El {
    row()
        .w_full()
        .padding(6.0)
        .gap(8.0)
        .center_y()
        .child(text(title).font(k.ty.small_medium).color(k.c.text_base))
        .child(
            text(label.map_or("", |label| &label.text))
                .key(format!("comparison-pane-name-{title}"))
                .font(k.ty.small)
                .color(k.c.text_muted)
                .grow(1.0)
                .min_w(0.0)
                .truncate()
                .help(label.map_or("", |label| &label.description)),
        )
}
pub(super) fn empty(address: Address, message: &str, k: &Kit) -> El {
    col()
        .key(address.key("canvas"))
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .clip_soft(0.0)
        .padding(20.0)
        .center()
        .on_layout(move |s: &mut State, r| address.layout(s, [r[2], r[3]]))
        .child(
            text(message)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w_full()
                .wrap(),
        )
}
fn bottom(state: &State, k: &Kit) -> El {
    let active = state
        .flow_comparison
        .current
        .as_ref()
        .map(|pair| pair.active_source)
        .unwrap_or(SideId::Before);
    let area = col()
        .key("comparison-source-area")
        .w_full()
        .h(230.0)
        .min_h(0.0)
        .border(1.0, k.c.std_border)
        .child(tabs(active, k));
    let Some(pair) = &state.flow_comparison.current else {
        return area.child(
            text(&state.flow_comparison.detail_message)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .padding(12.0),
        );
    };
    let address = address(active);
    match pair.side(active) {
        Side::Ready(ready) => area.child(source_details(state, ready, address, k)),
        side => area.child(
            text(explanation(side))
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w_full()
                .padding(12.0)
                .wrap(),
        ),
    }
}
fn source_details(
    state: &State,
    ready: &crate::flow_comparison::ReadySide,
    address: Address,
    k: &Kit,
) -> El {
    row()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .h_full()
                .child(
                    text(format!(
                        "Analyzed snapshot · {}:{} · {}",
                        ready.node.owner.file, ready.node.line, ready.node.owner.language
                    ))
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .w_full()
                    .truncate()
                    .help(format!(
                        "Project: {}\nBuild: {}",
                        ready.node.owner.project, ready.node.owner.build
                    )),
                )
                .child(
                    flow_view::source::view(state, &ready.session, address, k)
                        .grow(1.0)
                        .min_h(0.0),
                ),
        )
        .child(
            flow_view::details::view(state, &ready.session, address, k)
                .w(340.0)
                .h_full(),
        )
}
fn tabs(active: SideId, k: &Kit) -> El {
    let mut tabs = row().w_full().gap(8.0).px(8.0).py(4.0).center_y();
    for side in [SideId::Before, SideId::After] {
        tabs = tabs.child(
            Button::new(format!("{} source", title(side)))
                .variant(if active == side {
                    ButtonVariant::Outline
                } else {
                    ButtonVariant::Ghost
                })
                .on_press(move |s: &mut State| s.flow_comparison.activate_source(side))
                .build(k)
                .key(match side {
                    SideId::Before => "comparison-source-before",
                    SideId::After => "comparison-source-after",
                }),
        );
    }
    tabs
}
fn explanation(side: &Side) -> String {
    match side {
        Side::Missing => "Missing in this revision".into(),
        Side::Unavailable(side) => format!(
            "{} · {}\n{}",
            match side.membership {
                crate::engine_client::flow_comparison::Presence::Present =>
                    "Present, flow unavailable",
                crate::engine_client::flow_comparison::Presence::Unknown => "Membership unknown",
            },
            side.failure.reason,
            side.failure.message
        ),
        Side::Ready(_) => String::new(),
    }
}
