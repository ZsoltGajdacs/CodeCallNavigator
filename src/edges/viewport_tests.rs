//! Production shape emission, without a GPU, for routes crossing a small viewport.
use super::*;
use crate::projection::{GraphView, ViewNode};
use rune::{Cx, El, Env, Runtime, canvas, render::KIND_SEGMENT};

fn draw(shape: Shape) -> Vec<rune::render::Instance> {
    fn view(shape: &Shape, _: &mut Cx<Shape>) -> El {
        canvas(shape.clone()).size(100.0, 100.0)
    }
    let mut runtime = Runtime::new(shape, view, (100.0, 100.0), Env::new(true, 1.0));
    runtime.settle_all();
    runtime.frame(0.0);
    runtime.instances
}

fn graph(points: Vec<[f32; 2]>, dashed: bool) -> State {
    let mut state = State::new();
    state.viewport = [100.0, 100.0];
    state.nav.camera.pan = [0.0, 0.0];
    state.graph_view = GraphView {
        nodes: vec![ViewNode::default(); 2],
        edges: vec![ViewEdge {
            from: 0,
            to: 1,
            points,
            kind: "call".into(),
            certainty: if dashed { "dispatch" } else { "direct" }.into(),
            ..Default::default()
        }],
        ..Default::default()
    };
    state
}

#[test]
fn long_crossing_dash_routes_emit_only_viewport_pieces() {
    let state = graph(vec![[-10_000.0, 50.0], [10_000.0, 50.0]], true);
    let emphasis = GraphEmphasis::new(&state.graph_view, None);
    let instances = draw(render(&state, &theme::kit(true), &emphasis));
    let segments: Vec<_> = instances
        .iter()
        .filter(|i| i.params[1] == KIND_SEGMENT)
        .collect();
    assert!(!segments.is_empty(), "Crossing route must remain visible");
    assert!(
        segments.len() < 20,
        "Emitted {} dashes for a 100-point viewport",
        segments.len()
    );
    assert!(
        segments
            .iter()
            .all(|i| i.rect[0] > -20.0 && i.rect[0] < 120.0)
    );
}

fn segments(shape: Shape) -> Vec<[[f32; 2]; 2]> {
    draw(shape)
        .iter()
        .filter(|i| i.params[1] == KIND_SEGMENT)
        .map(|i| [[i.radii[0], i.radii[1]], [i.radii[2], i.radii[3]]])
        .collect()
}

fn distance(point: [f32; 2], segments: &[[[f32; 2]; 2]]) -> f32 {
    segments
        .iter()
        .map(|segment| {
            let delta = [segment[1][0] - segment[0][0], segment[1][1] - segment[0][1]];
            let offset = [point[0] - segment[0][0], point[1] - segment[0][1]];
            let squared = delta[0] * delta[0] + delta[1] * delta[1];
            let t = if squared == 0.0 {
                0.0
            } else {
                ((offset[0] * delta[0] + offset[1] * delta[1]) / squared).clamp(0.0, 1.0)
            };
            (offset[0] - t * delta[0]).hypot(offset[1] - t * delta[1])
        })
        .fold(f32::INFINITY, f32::min)
}

#[test]
fn clipping_preserves_visible_strokes_through_joins_reentry_pan_and_zoom() {
    let routes = [
        vec![[-200.0, 25.0], [300.0, 75.0]],
        vec![
            [-200.0, 25.0],
            [300.0, 25.0],
            [-200.0, 75.0],
            [300.0, 75.0],
            [300.0, 75.0],
        ],
        vec![[-100.0, 50.0], [50.0, -100.0], [50.0, 200.0], [200.0, 50.0]],
    ];
    for points in routes {
        for dashed in [false, true] {
            check_route_crops(&points, dashed);
        }
    }
}

fn check_route_crops(points: &[[f32; 2]], dashed: bool) {
    let mut state = graph(points.to_vec(), dashed);
    for zoom in [0.5, 1.0, 2.0] {
        state.nav.camera.zoom = zoom;
        for pan in [[0.0, 0.0], [13.25, -8.5]] {
            state.nav.camera.pan = pan;
            assert_visible_strokes(&state, points);
        }
    }
}

fn assert_visible_strokes(state: &State, points: &[[f32; 2]]) {
    let emphasis = GraphEmphasis::new(&state.graph_view, None);
    let kit = theme::kit(true);
    let clipped = segments(render(state, &kit, &emphasis));
    let controls: Vec<_> = points.iter().map(|&p| state.nav.camera.screen(p)).collect();
    let reference = segments(stroke(
        Shape::new(),
        &controls,
        style(&state.graph_view.edges[0], &kit, 1.0),
    ));
    for x in (0..=100).step_by(5) {
        for y in (0..=100).step_by(5) {
            let point = [x as f32, y as f32];
            let expected = distance(point, &reference);
            if expected < 2.0 {
                assert!(
                    (distance(point, &clipped) - expected).abs() < 0.02,
                    "Lost visible geometry at {point:?}, camera={:?}",
                    state.nav.camera
                );
            }
        }
    }
}

#[test]
fn hidden_prefix_keeps_original_dash_phase() {
    let state = graph(vec![[-10_000.0, 50.0], [10_000.0, 50.0]], true);
    let emphasis = GraphEmphasis::new(&state.graph_view, None);
    let segments = segments(render(&state, &theme::kit(true), &emphasis));
    // 10,000 points into a 7-on/4-off pattern is phase 1: this dash ends at x=6.
    assert!(
        segments
            .iter()
            .any(|p| (p[0][0] + 1.0).abs() < 0.001 && (p[1][0] - 6.0).abs() < 0.001)
    );
}

#[test]
fn arrowheads_remain_when_tips_are_just_outside_each_viewport_edge() {
    for tip in [[-5.0, 50.0], [105.0, 50.0], [50.0, -5.0], [50.0, 105.0]] {
        let state = graph(vec![[50.0, 50.0], tip], false);
        let emphasis = GraphEmphasis::new(&state.graph_view, None);
        let instances = draw(render(&state, &theme::kit(true), &emphasis));
        assert!(
            instances
                .iter()
                .any(|i| i.params[1] == rune::render::KIND_TRIANGLE)
        );
    }
    let state = graph(vec![[-100.0, -100.0], [-50.0, -50.0]], true);
    assert!(
        render(
            &state,
            &theme::kit(true),
            &GraphEmphasis::new(&state.graph_view, None)
        )
        .is_empty()
    );
}

#[test]
fn pan_reuses_geometry_but_zoom_and_replacement_refresh_it() {
    let mut state = graph(vec![[-100.0, 50.0], [200.0, 50.0]], true);
    let kit = theme::kit(true);
    let render = |s: &State| {
        let emphasis = s
            .graph_emphasis
            .borrow_mut()
            .get(&s.graph_view, s.visual_selection.as_deref());
        super::render(s, &kit, &emphasis)
    };
    let first = render(&state);
    let emphasis = state
        .graph_emphasis
        .borrow_mut()
        .get(&state.graph_view, None);
    for pan in [1.0, 4.0, 9.0] {
        state.nav.camera.pan[0] = pan;
        render(&state);
    }
    assert_eq!(state.edge_geometry.borrow().builds, 1);
    let reused = state
        .graph_emphasis
        .borrow_mut()
        .get(&state.graph_view, None);
    assert!(std::rc::Rc::ptr_eq(&emphasis, &reused));
    state.nav.camera.zoom = 2.0;
    render(&state);
    assert_eq!(state.edge_geometry.borrow().builds, 2);
    let mut replacement = state.graph_view.clone();
    replacement.edges[0].points = vec![[0.0, 10.0], [100.0, 10.0]];
    state.replace_graph_view(replacement);
    let replaced = render(&state);
    assert_ne!(
        first, replaced,
        "Same edge count must not reuse the old layout"
    );
    assert_eq!(state.edge_geometry.borrow().builds, 1);
    let replaced_emphasis = state
        .graph_emphasis
        .borrow_mut()
        .get(&state.graph_view, None);
    assert!(!std::rc::Rc::ptr_eq(&emphasis, &replaced_emphasis));
}
