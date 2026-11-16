//! Native card presentation; interaction policy remains in graph_interaction.
use crate::{
    app::State, graph_emphasis::GraphEmphasis, graph_interaction, projection::ViewNode,
    projection::card_metrics as metrics, theme,
};
use rune::{Drag, El, ScrollSpec, TextStyle, col, el, text};
use rune_kit::{Kit, ScrollBar};
use std::time::Duration;

pub(super) fn node_card(state: &State, index: usize, k: &Kit, emphasis: &GraphEmphasis) -> El {
    let node = &state.graph_view.nodes[index];
    let selected = emphasis.is_anchor(index);
    let accent = k.c.text_accent;
    let border = if node.cyclic {
        theme::uncertain(k)
    } else if selected {
        accent
    } else {
        k.c.text_muted.alpha(0.3)
    };
    let point = state.nav.camera.screen(node.position);
    let scale = state.nav.camera.zoom;
    let mut card = col()
        .key(format!("graph-card-{}", node.key))
        .absolute(point[0], point[1])
        .size(node.size[0] * scale, node.size[1] * scale)
        .radius(metrics::RADIUS)
        .border(if selected { 2.0 } else { 1.0 }, border)
        .fill(theme::card_surface(k, selected, node.application_start))
        .padding(metrics::PADDING * scale)
        .gap(8.0 * scale)
        .clip_rounded(metrics::RADIUS)
        .child(node_header(node, index, CardStyle::new(selected, scale), k));
    if !node.methods.is_empty() {
        card = card.child(method_list(node, scale, k));
    }
    card.opacity(emphasis.card_opacity(index) * state.graph_motion.opacity(index))
}

struct CardStyle {
    scale: f32,
    title: TextStyle,
}

impl CardStyle {
    fn new(selected: bool, scale: f32) -> Self {
        Self {
            scale,
            title: TextStyle::new(
                ((if selected { 18.0 } else { 14.0 }) * scale).max(8.0),
                600.0,
            ),
        }
    }
}

fn node_header(node: &ViewNode, index: usize, style: CardStyle, k: &Kit) -> El {
    let scale = style.scale;
    let detail = node_detail(node);
    let header = col()
        .key(format!("graph-node-{index}"))
        .w_full()
        .h(metrics::HEADER_HEIGHT * scale)
        .min_h(0.0)
        .justify_center()
        .gap(3.0 * scale)
        .child(
            text(&node.label)
                .font(style.title)
                .color(k.c.text_base)
                .wrap_anywhere()
                .max_lines(2),
        )
        .child(
            text(detail)
                .font(TextStyle::new((11.0 * scale).max(7.0), 400.0))
                .color(k.c.text_muted)
                .truncate(),
        );
    node_actions(header, node)
}

fn node_actions(header: El, node: &ViewNode) -> El {
    let key = node.key.clone();
    let drill_key = key.clone();
    let member = node.member;
    let hover_node = node.clone();
    let detail = node_detail(node);
    header
        .on_hover_change(move |s: &mut State, inside, tx| {
            s.hover_source(hover_node.clone(), inside, tx)
        })
        .role("button")
        .label(format!(
            "{} · {} · {}",
            node.label, node.description, detail
        ))
        .help(&node.description)
        .focusable(true)
        .on_press_tx(move |state: &mut State, tx| {
            let interval = graph_interaction::double_click_interval();
            state.press_graph_node(key.clone(), member, interval);
            tx.wake_after(Duration::from_secs_f64(interval));
        })
        .on_double_press(move |state: &mut State| {
            state.drill_graph_node(drill_key.clone(), member);
        })
}

fn method_list(node: &ViewNode, scale: f32, k: &Kit) -> El {
    let mut rows = col().w_full().gap(0.0);
    for method in &node.methods {
        let member = method.member;
        rows = rows.child(
            el().key(format!("graph-method-{member}"))
                .w_full()
                .h(metrics::MEMBER_HEIGHT * scale)
                .min_h(metrics::MEMBER_HEIGHT * scale)
                .center_y()
                .px(3.0 * scale)
                .radius(3.0)
                .hover(k.c.text_accent.alpha(0.12))
                .role("button")
                .label(format!("{} · {}", method.label, method.description))
                .help(&method.description)
                .focusable(true)
                .on_press(move |state: &mut State| state.focus_graph_method(member))
                .child(
                    text(&method.label)
                        .font(theme::mono((11.0 * scale).max(7.0), 400.0))
                        .color(k.c.text_base)
                        .truncate(),
                ),
        );
    }
    col()
        .key(format!("graph-methods-{}", node.key))
        .w_full()
        .h(metrics::MEMBER_HEIGHT * node.methods.len().min(metrics::MAX_MEMBERS) as f32 * scale)
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        // A native scroll viewport stops wheel routing before the canvas. Its own
        // hit target also keeps drags in blank row space from panning the graph.
        .on_drag(|_: &mut State, _: &mut Drag, _| {})
        .on_magnify(|_: &mut State, _, _| true)
        .child(rows)
        .scroll_bar(k)
}

fn node_detail(node: &ViewNode) -> String {
    if node.member.is_none() {
        format!(
            "{} symbols · {} links inside",
            node.count, node.internal_edges
        )
    } else if node.application_start {
        format!("Application start · {}", node.detail)
    } else {
        node.detail.clone()
    }
}

pub(super) fn ghost(state: &State, node: &ViewNode, opacity: f32, k: &Kit) -> El {
    let point = state.nav.camera.screen(node.position);
    let scale = state.nav.camera.zoom;
    col()
        .key(format!("departing-{}", node.key))
        .absolute(point[0], point[1])
        .size(node.size[0] * scale, node.size[1] * scale)
        .radius(metrics::RADIUS)
        .fill(theme::card_surface(k, false, false))
        .border(1.0, k.c.text_muted.alpha(0.3))
        .padding(metrics::PADDING * scale)
        .opacity(opacity)
        .child(
            text(&node.label)
                .font(TextStyle::new((14.0 * scale).max(8.0), 600.0))
                .color(k.c.text_base)
                .truncate(),
        )
        .inert()
}
