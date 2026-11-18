//! Graph canvas and cards render projection metadata; interaction policy lives in its controller.
use crate::{app::State, theme};
use rune::{Cx, Drag, El, MagnifyEvent, TextStyle, WheelEvent, canvas, col, el, text};
use rune_kit::Kit;
mod cards;

pub fn graph_canvas(state: &State, cx: &mut Cx<State>, k: &Kit) -> El {
    state.graph_motion.reduced.set(cx.is_reduced_motion());
    let canvas = el()
        .key("graph-canvas")
        .focusable(true)
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
                .zoom_at([event.x, event.y], (event.dy / 240.0).exp());
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
        .child(graph_content(state, k));
    if state.graph_view.nodes.is_empty() {
        // Actions in the empty state stay interactive even before the first
        // layout, when the graph world deliberately ignores pointer input.
        canvas.child(empty_canvas(state, k))
    } else {
        canvas
    }
}

fn graph_content(state: &State, k: &Kit) -> El {
    let emphasis = state
        .graph_emphasis
        .borrow_mut()
        .get(&state.graph_view, state.visual_selection.as_deref());
    // Displayed geometry is already sampled by graph_motion. A second inherited
    // Rune layout spring would detach cards/hit targets from canvas edge shapes.
    let mut content = el()
        .key("graph-world")
        .motion(rune::Spring::NONE)
        .w_full()
        .h_full();
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
            let card = cards::node_card(state, index, k, &emphasis);
            content = content.child(if state.view_is_current() {
                card
            } else {
                card.inert()
            });
        }
    }
    for (node, opacity) in state.graph_motion.ghosts() {
        content = content.child(cards::ghost(state, node, opacity, k));
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
    let feedback = state.graph.is_none().then(|| state.feedback()).flatten();
    let description = feedback.as_ref().map(|failure| failure.summary());
    let (title, description) = description.as_deref().map_or_else(
        || empty_message(state),
        |description| ("Analysis failed", description),
    );
    let panel = col()
        .key("empty-canvas")
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
        );
    if let Some(failure) = feedback {
        panel
            .child(
                text(failure.excerpt())
                    .key("analysis-failure-excerpt")
                    .font(k.ty.small)
                    .color(k.c.warning.fg)
                    .max_w(580.0)
                    .wrap_anywhere(),
            )
            .child(super::analysis_failure::actions(state, k, true))
    } else {
        panel
    }
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
