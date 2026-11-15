//! Native flow workspace: chart geometry and analyzed source have independent navigation.
pub(crate) mod address;
pub(crate) mod chart;
pub(crate) mod details;
mod expansion;
pub(crate) mod source;
use crate::{app::State, flow_workspace::Session, theme};
use rune::{Cx, El, Phase, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

pub fn view(state: &State, cx: &mut Cx<State>) -> El {
    let k = theme::kit(cx.system_dark());
    let session = state.flow_workspace.current.as_ref().unwrap();
    let name = state
        .graph
        .as_ref()
        .and_then(|g| {
            g.node_index(&session.symbol)
                .map(|i| g.data.nodes[i as usize].signature.as_str())
        })
        .unwrap_or(&session.symbol);
    let body = row()
        .motion(rune::Spring::NONE)
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .child(chart::view(state, session, address::Address::Single, &k))
        .child(divider(&k))
        .child(source_pane(state, session, &k));
    let content = col()
        .key("flow-workspace")
        .w_full()
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base)
        .on_layout(|s: &mut State, r| s.flow_workspace.window = [r[2], r[3]])
        .child(header(state, name, &k, cx.titlebar_inset()))
        .child(notice(state, session, &k))
        .child(body)
        .child(details::view(state, session, address::Address::Single, &k));
    crate::view::with_overlays(state, &k, content, Vec::new()).motion(rune::Spring::NONE)
}
fn header(state: &State, name: &str, k: &Kit, titlebar_inset: f32) -> El {
    col()
        .key("flow-toolbar")
        .w_full()
        .px(10.0)
        .py(8.0)
        .gap(6.0)
        .child(navigation(state, k, titlebar_inset))
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(
                    text(name)
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base)
                        .w(0.0)
                        .grow(1.0)
                        .min_w(0.0)
                        .truncate()
                        .help(name),
                )
                .child(expansion::controls(state, k))
                .child(
                    Button::new("Fit")
                        .on_press(State::fit_flow)
                        .build(k)
                        .key("flow-fit"),
                ),
        )
}
fn navigation(state: &State, k: &Kit, titlebar_inset: f32) -> El {
    row()
        .w_full()
        .pl(titlebar_inset)
        .gap(8.0)
        .center_y()
        .child(crate::view::search_button(k))
        .child(crate::view::comparison_button(k))
        .child(
            Button::new("Back to call graph")
                .variant(ButtonVariant::Outline)
                .on_press(State::close_flow)
                .build(k)
                .key("flow-close"),
        )
        .child(
            Button::new("Previous function")
                .disabled(state.flow_workspace.back.is_empty())
                .on_press(State::flow_back)
                .build(k)
                .key("flow-back"),
        )
}

fn notice(state: &State, session: &Session, k: &Kit) -> El {
    let mut content = col().w_full().px(12.0).gap(3.0);
    let explanation = "Analyzed source snapshot · source-derived paths; callbacks and exceptions may be conditional";
    if let Some(notice) = crate::view::freshness_notice(state, k) {
        content = content.child(notice);
    }
    content = content.child(
        text(explanation)
            .font(k.ty.small)
            .color(k.c.text_muted)
            .w_full()
            .wrap(),
    );
    if let Some(ready) = &session.ready {
        content = content.child(
            text(session.displayed_file().unwrap_or(&ready.chart.file))
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w_full()
                .truncate()
                .help(session.displayed_file().unwrap_or(&ready.chart.file)),
        );
        if ready.chart.partial {
            content = content.child(
                text("Partial flow · see diagnostics below")
                    .font(k.ty.small)
                    .color(k.c.warning.fg),
            );
        }
    }
    if session.expansion_mode() {
        content.child(expansion::notice(session, k))
    } else {
        content
    }
}
fn divider(k: &Kit) -> El {
    el().key("flow-divider")
        .w(6.0)
        .h_full()
        .fill(k.c.std_border.alpha(0.5))
        .cursor(rune::Cursor::ResizeColumn)
        .on_drag(|s: &mut State, drag, _| {
            if drag.phase == Phase::Down {
                s.flow_workspace.drag_width = source_width(s);
            }
            let max = (s.flow_workspace.window[0] * 0.7).max(300.0);
            s.flow_workspace.source_width =
                (s.flow_workspace.drag_width - drag.dx).clamp(300.0, max);
        })
}
fn source_width(state: &State) -> f32 {
    let window = state.flow_workspace.window[0].max(800.0);
    if state.flow_workspace.source_width > 0.0 {
        state.flow_workspace.source_width.clamp(300.0, window * 0.7)
    } else {
        window * 0.45
    }
}
fn source_pane(state: &State, session: &Session, k: &Kit) -> El {
    col()
        .w(source_width(state))
        .h_full()
        .min_h(0.0)
        .child(source::view(state, session, address::Address::Single, k))
}

pub fn key(state: &mut State, event: &rune::KeyEvent) -> bool {
    match &event.key {
        rune::Key::Escape => {
            state.close_flow();
            true
        }
        rune::Key::Left if event.mods.alt => {
            state.flow_back();
            true
        }
        _ => false,
    }
}
