//! Native flow workspace: chart geometry and analyzed source have independent navigation.
mod details;
use crate::{
    app::State,
    flow_workspace::{Ready, Session},
    theme,
};
use codegraph::flow::FlowStep;
use rune::{Cx, El, GrowthSpec, Phase, Shape, canvas, col, el, row, text};
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
        .child(chart(state, session, &k))
        .child(divider(&k))
        .child(source_pane(state, session, &k));
    let content = col()
        .key("flow-workspace")
        .w_full()
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base)
        .on_layout(|s: &mut State, r| s.flow_workspace.window = [r[2], r[3]])
        .child(header(state, name, &k))
        .child(notice(state, session, &k))
        .child(body)
        .child(details::view(state, session, &k));
    rune_kit::root(content, Vec::new())
}
fn header(state: &State, name: &str, k: &Kit) -> El {
    row()
        .key("flow-toolbar")
        .w_full()
        .padding(10.0)
        .gap(10.0)
        .center_y()
        .flex_wrap()
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
        .child(
            text(name)
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .grow(1.0)
                .min_w(0.0)
                .truncate()
                .help(name),
        )
        .child(
            Button::new("Fit")
                .on_press(State::fit_flow)
                .build(k)
                .key("flow-fit"),
        )
}
fn notice(state: &State, session: &Session, k: &Kit) -> El {
    let mut content = col().w_full().px(12.0).gap(3.0);
    let explanation = if state.stale {
        "Analyzed source snapshot · disk has changed; refresh the call graph to update"
    } else {
        "Analyzed source snapshot · source-derived paths; callbacks and exceptions may be conditional"
    };
    content = content.child(
        text(explanation)
            .font(k.ty.small)
            .color(k.c.text_muted)
            .w_full()
            .wrap(),
    );
    if let Some(ready) = &session.ready {
        content = content.child(
            text(&ready.chart.file)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w_full()
                .truncate()
                .help(&ready.chart.file),
        );
        if ready.chart.partial {
            content = content.child(
                text("Partial flow · see diagnostics below")
                    .font(k.ty.small)
                    .color(k.c.warning.fg),
            );
        }
    }
    content
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
fn chart(state: &State, session: &Session, k: &Kit) -> El {
    let mut content = el()
        .motion(rune::Spring::NONE)
        .key("flow-canvas")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .clip_soft(0.0)
        .fill(theme::canvas(k))
        .on_layout(|s: &mut State, r| {
            s.flow_workspace.viewport = [r[2], r[3]];
            if s.flow_workspace.fit_pending {
                s.fit_flow();
                s.flow_workspace.fit_pending = false;
            }
        })
        .on_wheel(|s: &mut State, event, _| {
            if let Some(session) = &mut s.flow_workspace.current {
                session
                    .camera
                    .zoom_at([event.x, event.y], (-event.dy / 240.0).exp());
            }
            true
        })
        .on_magnify(|s: &mut State, event, _| {
            if let Some(session) = &mut s.flow_workspace.current {
                session
                    .camera
                    .zoom_at([event.x, event.y], 1.0 + event.delta);
            }
            true
        })
        .on_drag(|s: &mut State, drag, _| {
            if let Some(session) = &mut s.flow_workspace.current {
                session.camera.drag(drag);
            }
        });
    let Some(ready) = &session.ready else {
        return content.child(
            col().w_full().h_full().center().padding(20.0).child(
                text(&session.message)
                    .font(k.ty.ui)
                    .color(k.c.text_muted)
                    .wrap(),
            ),
        );
    };
    content = content.child(connectors(session, ready, state.flow_workspace.viewport, k));
    for (index, step) in ready.chart.steps.iter().enumerate() {
        if session
            .camera
            .shows_box(step.position, step.size, state.flow_workspace.viewport)
        {
            content = content.child(step_card(session, step, index, k));
        }
    }
    content
}
fn connectors(session: &Session, ready: &Ready, viewport: [f32; 2], k: &Kit) -> El {
    let mut shape = Shape::new();
    let mut content = el().w_full().h_full().inert();
    for link in &ready.chart.links {
        let points = link
            .points
            .iter()
            .map(|&p| session.camera.screen(p))
            .collect::<Vec<_>>();
        if !crate::edges::visible(&points, viewport) {
            continue;
        }
        let uncertain = matches!(link.kind.as_str(), "exception" | "deferred" | "unknown");
        let color = if uncertain {
            theme::uncertain(k)
        } else {
            k.c.text_muted.alpha(0.7)
        };
        shape = crate::edges::route(shape, &points, color, uncertain);
        if !link.label.is_empty() && !points.is_empty() {
            let p = points[points.len() / 2];
            content = content.child(
                text(&link.label)
                    .absolute(p[0] + 4.0, p[1] - 14.0)
                    .font(k.ty.small)
                    .color(color)
                    .fill(theme::canvas(k))
                    .px(3.0),
            );
        }
    }
    content.child(canvas(shape).w_full().h_full())
}
fn step_card(session: &Session, step: &FlowStep, index: usize, k: &Kit) -> El {
    let p = session.camera.screen(step.position);
    let scale = session.camera.zoom;
    let selected = session.selected == Some(index);
    let semantic = matches!(step.kind.as_str(), "decision" | "switch" | "loop");
    let border = if selected {
        k.c.text_accent
    } else if semantic {
        theme::uncertain(k)
    } else {
        k.c.text_muted.alpha(0.4)
    };
    col()
        .key(format!("flow-step-{index}"))
        .absolute(p[0], p[1])
        .size(step.size[0] * scale, step.size[1] * scale)
        .radius(if semantic { 3.0 } else { 8.0 })
        .border(if selected { 2.0 } else { 1.0 }, border)
        .fill(if selected {
            k.c.text_accent.alpha(0.15)
        } else {
            k.c.layer_01
        })
        .padding(7.0 * scale)
        .clip_rounded(5.0)
        .child(
            text(&step.kind)
                .font(theme::mono((10.0 * scale).max(7.0), 500.0))
                .color(k.c.text_muted)
                .truncate(),
        )
        .child(
            text(&step.label)
                .font(theme::mono((12.0 * scale).max(8.0), 450.0))
                .color(k.c.text_base)
                .w_full()
                .wrap_anywhere(),
        )
        .help(&step.label)
        .role("button")
        .label(&step.label)
        .focusable(true)
        .on_press_tx(move |s: &mut State, tx| s.select_flow_step(index, tx))
}
fn source_pane(state: &State, session: &Session, k: &Kit) -> El {
    let pane = col()
        .key("flow-source-pane")
        .w(source_width(state))
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base);
    let Some(ready) = &session.ready else {
        return pane;
    };
    let kit = k.clone();
    let offset = session.scroll.clone();
    let width = (ready.source.max_columns as f32 * 8.0 + 90.0).max(300.0);
    pane.child(
        col()
            .key(session.scroll_key.clone())
            .w_full()
            .grow(1.0)
            .min_h(0.0)
            .scroll_xy()
            .selection_group(true)
            .on_scroll_offset(move |at| offset.set(at.offset))
            .child(
                col()
                    .w(width)
                    .growth(GrowthSpec::default())
                    .virtual_rows_version(state.flow_workspace_revision())
                    .virtual_rows(
                        ready.source.lines.len(),
                        20.0,
                        |_: &State, i| format!("flow-source-line-{i}").into(),
                        move |s: &State, i| source_line(s, i, &kit),
                    ),
            ),
    )
}
fn source_line(state: &State, index: usize, k: &Kit) -> El {
    let session = state.flow_workspace.current.as_ref().unwrap();
    let ready = session.ready.as_ref().unwrap();
    let line = &ready.source.lines[index];
    let line_end = ready
        .source
        .lines
        .get(index + 1)
        .map(|next| next.range.start)
        .unwrap_or(line.range.end);
    let selected = session
        .selected
        .and_then(|i| ready.chart.steps.get(i))
        .is_some_and(|step| line.range.start < step.span.end && line_end > step.span.start);
    crate::source_view::line(
        &ready.source,
        crate::source_view::SourceRow {
            index,
            selected,
            key: format!("flow-source-text-{index}"),
        },
        k,
    )
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
