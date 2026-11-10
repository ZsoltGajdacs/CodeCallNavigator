//! Reproducible CPU frame preparation benchmark; excludes GPU presentation.
use crate::{app::State, view};
use rune::{Env, Runtime, render::KIND_SEGMENT};
use std::time::Instant;

const NODES: usize = 300;
const EDGES: usize = 6_000;
const FRAMES: usize = 60;

#[test]
#[ignore = "release-mode canvas benchmark; run with --ignored --nocapture"]
fn dense_canvas_frame_preparation() {
    assert!(!cfg!(debug_assertions), "Run this benchmark with --release");
    for scale in [1.0, 2.0] {
        let mut runtime = Runtime::new(
            fixture(),
            view::view,
            (1900.0, 1100.0),
            Env::new(true, scale),
        );
        runtime.settle_all();
        runtime.frame(0.0);
        for zoom in [0.16, 0.20, 0.24] {
            measure(&mut runtime, zoom);
        }
    }
}

fn measure(runtime: &mut Runtime<State>, zoom: f32) {
    let mut elapsed = Vec::with_capacity(FRAMES);
    let mut segments = 0;
    let mut instances = 0;
    for frame in 0..FRAMES + 12 {
        runtime.state.nav.camera.zoom = zoom;
        runtime.state.nav.camera.pan = [24.0 + (frame % 12) as f32, 100.0];
        runtime.invalidate();
        let start = Instant::now();
        runtime.frame(1.0 / 60.0);
        let milliseconds = start.elapsed().as_secs_f64() * 1000.0;
        if frame >= 12 {
            elapsed.push(milliseconds);
            segments += runtime
                .instances
                .iter()
                .filter(|i| i.params[1] == KIND_SEGMENT)
                .count();
            instances += runtime.instances.len();
        }
    }
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .all(|node| { runtime.state.nav.camera.shows(node, runtime.state.viewport) }),
        "Every benchmark node must remain visible"
    );
    elapsed.sort_by(f64::total_cmp);
    println!(
        "canvas scale={} nodes={NODES} edges={EDGES} zoom={zoom:.2} frames={FRAMES} median_ms={:.3} p95_ms={:.3} mean_instances={} mean_segments={} layout_nodes={}",
        runtime.env.scale,
        elapsed[FRAMES / 2],
        elapsed[FRAMES * 95 / 100],
        instances / FRAMES,
        segments / FRAMES,
        runtime.stats.layout_nodes
    );
}

fn fixture() -> State {
    let mut state = State::new();
    crate::canvas_fixture::install(&mut state);
    state
}

#[test]
#[ignore = "requires an explicit copied CODEGRAPH_DATA_DIR; release CPU benchmark only"]
fn saved_canvas_frame_preparation() {
    assert!(!cfg!(debug_assertions), "Run this benchmark with --release");
    assert!(
        std::env::var_os("CODEGRAPH_DATA_DIR").is_some(),
        "Use a copied profiling workspace"
    );
    for scale in [1.0, 2.0] {
        let mut state = State::new();
        crate::canvas_fixture::saved(&mut state);
        assert!(state.warning.is_empty(), "{}", state.warning);
        assert!(!state.graph_view.edges.is_empty());
        let mut runtime = Runtime::new(state, view::view, (1900.0, 1100.0), Env::new(true, scale));
        runtime.poll = Some(crate::jobs::poll);
        runtime.settle_all();
        runtime.frame(0.0);
        measure_pointer_drag(&mut runtime);
    }
}

fn measure_pointer_drag(runtime: &mut Runtime<State>) {
    let canvas = runtime
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("graph-canvas"))
        .unwrap()
        .rect;
    let point = [canvas[0] + canvas[2] - 8.0, canvas[1] + canvas[3] - 8.0];
    let initial_pan = runtime.state.nav.camera.pan;
    runtime.pointer_down(point[0], point[1]);
    runtime.frame(1.0 / 60.0);
    let mut elapsed = Vec::new();
    for frame in 0..132 {
        runtime.pointer_move(
            point[0] + (frame as f32 * 0.2).sin() * 180.0,
            point[1] + (frame as f32 * 0.2).cos() * 90.0,
        );
        let start = Instant::now();
        runtime.frame(1.0 / 60.0);
        if frame >= 12 {
            elapsed.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    assert_ne!(
        initial_pan, runtime.state.nav.camera.pan,
        "Pointer must pan the graph"
    );
    elapsed.sort_by(f64::total_cmp);
    let segments: Vec<_> = runtime
        .instances
        .iter()
        .filter(|i| i.params[1] == KIND_SEGMENT)
        .collect();
    let offscreen = segments
        .iter()
        .filter(|i| {
            let [x, y, w, h] = i.rect;
            let [l, t, r, b] = i.clip;
            x + w + 1.0 < l || y + h + 1.0 < t || x - 1.0 > r || y - 1.0 > b
        })
        .count();
    println!(
        "saved scale={} nodes={} edges={} median_ms={:.3} p95_ms={:.3} instances={} segments={} offscreen_segments={offscreen} geometry_builds={}",
        runtime.env.scale,
        runtime.state.graph_view.nodes.len(),
        runtime.state.graph_view.edges.len(),
        elapsed[60],
        elapsed[114],
        runtime.instances.len(),
        segments.len(),
        runtime.state.edge_geometry.borrow().builds
    );
    runtime.pointer_up(point[0], point[1]);
    runtime.frame(1.0 / 60.0);
}
