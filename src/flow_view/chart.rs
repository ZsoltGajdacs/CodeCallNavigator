//! Chart geometry is shared; all interactions resolve through a concrete pane address.
use super::address::Address;
use crate::{
    app::State,
    flow_workspace::{Ready, Session},
    theme,
};
use codecallnavigator::flow::FlowStep;
use rune::{El, Shape, canvas, col, el, text};
use rune_kit::Kit;
pub(crate) fn view(state: &State, session: &Session, address: Address, k: &Kit) -> El {
    let mut content = el()
        .motion(rune::Spring::NONE)
        .key(address.key("canvas"))
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .clip_soft(0.0)
        .fill(theme::canvas(k))
        .on_layout(move |s: &mut State, r| address.layout(s, [r[2], r[3]]))
        .on_wheel(move |s: &mut State, event, _| {
            if let Some(session) = address.session_mut(s) {
                session
                    .camera
                    .zoom_at([event.x, event.y], (event.dy / 240.0).exp());
            }
            true
        })
        .on_magnify(move |s: &mut State, event, _| {
            if let Some(session) = address.session_mut(s) {
                session
                    .camera
                    .zoom_at([event.x, event.y], 1.0 + event.delta);
            }
            true
        })
        .on_drag(move |s: &mut State, drag, _| {
            if let Some(session) = address.session_mut(s) {
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
    if address.side().is_some() {
        let key = session.scroll_key.clone();
        content = content.child(
            el().key(format!("{key}-measure"))
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .inert()
                .on_layout(move |state: &mut State, rect| {
                    address.measured(state, &key, [rect[2], rect[3]])
                }),
        );
    }
    content = content.child(connectors(session, ready, address.viewport(state), k));
    for (index, step) in ready.chart.steps.iter().enumerate() {
        if session
            .camera
            .shows_box(step.position, step.size, address.viewport(state))
        {
            content = content.child(step_card(session, Card { step, index }, address, k));
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
struct Card<'a> {
    step: &'a FlowStep,
    index: usize,
}
fn step_card(session: &Session, card: Card<'_>, address: Address, k: &Kit) -> El {
    let Card { step, index } = card;
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
        .key(address.key(&format!("step-{index}")))
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
        .on_press_tx(move |s: &mut State, tx| address.select(s, index, tx))
}
