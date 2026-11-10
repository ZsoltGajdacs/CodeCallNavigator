//! Reproducible CPU frame preparation benchmark; excludes GPU presentation.
use crate::{
    app::State,
    projection::{ViewEdge, ViewNode},
    scenes, view,
};
use rune::{Env, Runtime, render::KIND_SEGMENT};
use std::time::Instant;

const NODES: usize = 300;
const EDGES: usize = 6_000;
const FRAMES: usize = 60;

#[test]
#[ignore = "release-mode canvas benchmark; run with --ignored --nocapture"]
fn dense_canvas_frame_preparation() {
    assert!(!cfg!(debug_assertions), "Run this benchmark with --release");
    let mut runtime = Runtime::new(fixture(), view::view, (1900.0, 1100.0), Env::new(true, 1.0));
    runtime.settle_all();
    runtime.frame(0.0);
    for zoom in [0.16, 0.20, 0.24] {
        measure(&mut runtime, zoom);
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
        "canvas nodes={NODES} edges={EDGES} zoom={zoom:.2} frames={FRAMES} median_ms={:.3} p95_ms={:.3} mean_instances={} mean_segments={} layout_nodes={}",
        elapsed[FRAMES / 2],
        elapsed[FRAMES * 95 / 100],
        instances / FRAMES,
        segments / FRAMES,
        runtime.stats.layout_nodes
    );
}

fn fixture() -> State {
    let mut state = State::new();
    scenes::install(&mut state);
    state.deterministic = true;
    state.graph_view.nodes = (0..NODES)
        .map(|index| ViewNode {
            key: format!("canvas-benchmark-{index}"),
            label: format!("Method{index}"),
            detail: "Benchmark fixture".into(),
            position: [(index % 20) as f32 * 160.0, (index / 20) as f32 * 110.0],
            size: [150.0, 90.0],
            count: 1,
            ..Default::default()
        })
        .collect();
    state.graph_view.edges = (0..EDGES)
        .map(|index| {
            let from = index % NODES;
            let to = (from + 1 + (index / NODES) * 13) % NODES;
            let start = state.graph_view.nodes[from].position;
            let end = state.graph_view.nodes[to].position;
            let bend = match index % 3 {
                0 => 0.0,
                1 => 1.0,
                _ => 180.0,
            };
            ViewEdge {
                from,
                to,
                kind: "call".into(),
                certainty: if index % 4 == 0 { "dispatch" } else { "direct" }.into(),
                count: 1,
                points: vec![
                    start,
                    [
                        start[0] + (end[0] - start[0]) / 3.0,
                        start[1] + (end[1] - start[1]) / 3.0 + bend,
                    ],
                    [
                        start[0] + (end[0] - start[0]) * 2.0 / 3.0,
                        start[1] + (end[1] - start[1]) * 2.0 / 3.0 + bend,
                    ],
                    end,
                ],
            }
        })
        .collect();
    state.graph_view.size = [3200.0, 1650.0];
    state
}
