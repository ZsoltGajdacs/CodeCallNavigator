//! Chart geometry is shared; all interactions resolve through a concrete pane address.
use super::address::Address;
use crate::{
    app::State,
    flow_comparison::StepChange,
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
    content
        .child(connectors(session, ready, address.viewport(state), k))
        .children(step_cards(state, session, address, k))
}

fn step_cards<'a>(
    state: &'a State,
    session: &'a Session,
    address: Address,
    k: &'a Kit,
) -> impl Iterator<Item = El> + 'a {
    let outline_side = address
        .side()
        .filter(|_| state.flow_comparison.changes.show_outline());
    let viewport = address.viewport(state);
    session
        .ready
        .iter()
        .flat_map(|ready| &ready.chart.steps)
        .enumerate()
        .filter(move |(_, step)| session.camera.shows_box(step.position, step.size, viewport))
        .map(move |(index, step)| {
            let change =
                outline_side.and_then(|side| state.flow_comparison.step_change(side, index));
            let card = step_card(
                session,
                Card {
                    step,
                    index,
                    change,
                },
                address,
                k,
            );
            preview_on_hover(card, outline_side, address, index)
        })
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
    change: Option<StepChange>,
}
fn step_card(session: &Session, card: Card<'_>, address: Address, k: &Kit) -> El {
    let Card {
        step,
        index,
        change,
    } = card;
    let p = session.camera.screen(step.position);
    let scale = session.camera.zoom;
    let selected = session.selected == Some(index);
    let badge = session.marker_badge(index);
    let paint = StepPaint::new(change, selected, step, k);
    let kind = step_label(step, badge, paint.mark);
    let control = col()
        .key(address.key(&format!("step-{index}")))
        .absolute(p[0], p[1])
        .size(step.size[0] * scale, step.size[1] * scale)
        .radius(paint.radius)
        .border(if selected { 2.0 } else { 1.0 }, paint.border)
        .fill(paint.fill)
        .padding(7.0 * scale)
        .clip_rounded(5.0)
        .child(
            text(kind.as_str())
                .key(address.key(&format!("step-kind-{index}")))
                .font(theme::mono((10.0 * scale).max(7.0), 500.0))
                .color(
                    paint
                        .change
                        .unwrap_or_else(|| badge.map_or(k.c.text_muted, |_| k.c.warning.fg)),
                )
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
        .on_press_tx(move |s: &mut State, tx| address.select(s, index, tx));
    if address.side().is_none() {
        return control;
    }
    // Rune includes controls with keyboard handlers in the macOS Tab order.
    control.on_key(move |state: &mut State, event, tx| {
        if event.key != rune::Key::Enter {
            return false;
        }
        address.select(state, index, tx);
        true
    })
}

struct StepPaint {
    mark: &'static str,
    change: Option<rune::Color>,
    radius: f32,
    border: rune::Color,
    fill: rune::Color,
}

impl StepPaint {
    fn new(change: Option<StepChange>, selected: bool, step: &FlowStep, k: &Kit) -> Self {
        let semantic = matches!(step.kind.as_str(), "decision" | "switch" | "loop");
        let (mark, color) = match change {
            Some(StepChange::Added) => ("+ ", Some(k.c.success.fg)),
            Some(StepChange::Removed) => ("− ", Some(k.c.danger.fg)),
            Some(StepChange::Modified) => ("± ", Some(k.c.text_accent)),
            None => ("", None),
        };
        let (border, fill) = match (color, selected, semantic) {
            (Some(color), _, _) => (color, color.alpha(0.12)),
            (None, true, _) => (k.c.text_accent, k.c.text_accent.alpha(0.15)),
            (None, false, true) => (theme::uncertain(k), k.c.layer_01),
            (None, false, false) => (k.c.text_muted.alpha(0.4), k.c.layer_01),
        };
        Self {
            mark,
            change: color,
            radius: if semantic { 3.0 } else { 8.0 },
            border,
            fill,
        }
    }
}

fn step_label(step: &FlowStep, badge: Option<&str>, mark: &str) -> String {
    let mut label = format!("{mark}{}", step.kind);
    if let Some(badge) = badge {
        label.push_str(" · ");
        label.push_str(badge);
    }
    label
}

fn preview_on_hover(
    card: El,
    side: Option<crate::flow_comparison::SideId>,
    address: Address,
    index: usize,
) -> El {
    let Some(side) = side else {
        return card;
    };
    let key = address.key(&format!("step-{index}"));
    card.hover_delays(
        std::time::Duration::from_millis(160),
        std::time::Duration::ZERO,
    )
    .on_hover_change(move |s: &mut State, inside, _| {
        if inside {
            s.flow_comparison
                .show_step_preview(side, index, key.clone());
        } else {
            s.flow_comparison.end_step_hover(&key);
        }
    })
}
