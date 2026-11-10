//! Graph canvas and cards render projection metadata; interaction policy lives in its controller.
use crate::{
    app::State, graph_emphasis::GraphEmphasis, graph_interaction, projection::ViewNode, theme,
};
use rune::{Cx, Drag, El, MagnifyEvent, ScrollSpec, TextStyle, WheelEvent, canvas, col, el, text};
use rune_kit::{Kit, ScrollBar};
use std::time::Duration;

pub fn graph_canvas(state: &State, _: &mut Cx<State>, k: &Kit) -> El {
    el().key("graph-canvas")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .clip_soft(0.0)
        .fill(theme::canvas(k))
        .on_layout(|state: &mut State, rect| {
            state.resize_graph_viewport([rect[2], rect[3]]);
            state.source_popup.canvas = [rect[0], rect[1]];
        })
        .on_wheel(|state: &mut State, event: WheelEvent, _| {
            state.cancel_graph_camera();
            state
                .nav
                .camera
                .zoom_at([event.x, event.y], (-event.dy / 240.0).exp());
            state.dirty_session = true;
            true
        })
        .on_magnify(|state: &mut State, event: MagnifyEvent, _| {
            state.cancel_graph_camera();
            state
                .nav
                .camera
                .zoom_at([event.x, event.y], 1.0 + event.delta);
            state.dirty_session = true;
            true
        })
        .on_drag(|state: &mut State, drag: &mut Drag, _| {
            state.cancel_graph_camera();
            state.nav.camera.drag(drag);
            state.dirty_session = true;
        })
        .child(graph_content(state, k))
}

fn graph_content(state: &State, k: &Kit) -> El {
    let emphasis = GraphEmphasis::new(&state.graph_view, state.visual_selection.as_deref());
    let mut content = el().key("graph-world").w_full().h_full();
    for (index, band) in state.graph_view.layer_bands.iter().enumerate() {
        content = content.child(layer_band(state, index, band, k));
    }
    let shapes = crate::edges::render(state, k, &emphasis);
    content = content.child(canvas(shapes).w_full().h_full().inert());
    if !state.view_is_current() {
        content = content.inert().opacity(0.45);
    }
    for (index, node) in state.graph_view.nodes.iter().enumerate() {
        if state.nav.camera.shows(node, state.viewport) {
            let card = node_card(state, index, k, &emphasis);
            content = content.child(if state.view_is_current() {
                card
            } else {
                card.inert()
            });
        }
    }
    if state.graph_view.nodes.is_empty() {
        content = content.child(empty_canvas(state, k));
    }
    content
}

fn layer_band(state: &State, index: usize, band: &crate::projection::LayerBand, k: &Kit) -> El {
    let point = state.nav.camera.screen(band.position);
    let scale = state.nav.camera.zoom;
    col()
        .key(format!("layer-band-{index}"))
        .absolute(point[0], point[1])
        .size(band.size[0] * scale, band.size[1] * scale)
        .radius(8.0)
        .border(1.0, k.c.text_muted.alpha(0.18))
        .fill(k.c.text_accent.alpha(0.035))
        .padding(6.0 * scale)
        .child(
            text(&band.name)
                .font(theme::mono(11.0 * scale, 600.0))
                .color(k.c.text_muted),
        )
        .inert()
}

fn empty_canvas(state: &State, k: &Kit) -> El {
    let (title, description) = empty_message(state);
    col()
        .absolute(0.0, 0.0)
        .w_full()
        .h_full()
        .center()
        .padding(24.0)
        .gap(12.0)
        .child(
            text(title)
                .font(TextStyle::new(20.0, 600.0))
                .color(k.c.text_base)
                .max_w(580.0)
                .wrap(),
        )
        .child(
            text(description)
                .font(k.ty.ui)
                .color(k.c.text_muted)
                .max_w(580.0)
                .wrap(),
        )
}

fn empty_message(state: &State) -> (&str, &str) {
    if state.graph.is_none() {
        return (
            "See how your application connects",
            "Choose a folder, select its builds, then generate a call graph.",
        );
    }
    if state.jobs.layout.is_pending() || state.needs_layout {
        return (
            "Preparing graph view…",
            "The complete analysis is available while the visible layout is prepared.",
        );
    }
    if !state.graph_view.message.is_empty() {
        return ("Architectural layers", &state.graph_view.message);
    }
    if !state.warning.is_empty() {
        return (
            "Graph view unavailable",
            "Your last successful analysis is preserved. Read the message in the inspector, or change the graph filters.",
        );
    }
    (
        "No symbols in this view",
        "Adjust the filters, search for a method, or return to Overview.",
    )
}

fn node_card(state: &State, index: usize, k: &Kit, emphasis: &GraphEmphasis) -> El {
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
        .radius(9.0)
        .border(if selected { 2.0 } else { 1.0 }, border)
        .fill(if selected {
            accent.alpha(0.16)
        } else if node.application_start {
            accent.alpha(0.10)
        } else {
            k.c.layer_01
        })
        .padding(10.0 * scale)
        .gap(8.0 * scale)
        .clip_rounded(9.0)
        .child(node_header(node, index, scale, k));
    if !node.methods.is_empty() {
        card = card.child(method_list(node, scale, k));
    }
    card.opacity(emphasis.card_opacity(index))
}

fn node_header(node: &ViewNode, index: usize, scale: f32, k: &Kit) -> El {
    let detail = node_detail(node);
    let header = col()
        .key(format!("graph-node-{index}"))
        .w_full()
        .h(41.2 * scale)
        .min_h(0.0)
        .gap(3.0 * scale)
        .child(
            text(&node.label)
                .font(theme::mono((14.0 * scale).max(8.0), 600.0))
                .color(k.c.text_base)
                .truncate(),
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
                .h(24.0 * scale)
                .min_h(24.0 * scale)
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
        .h(24.0 * node.methods.len().min(12) as f32 * scale)
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
