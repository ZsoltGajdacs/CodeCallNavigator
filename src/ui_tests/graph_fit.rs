//! Explicit Fit uses the full canvas with filters dismissed.
use super::click;
use crate::{app::State, jobs, projection, view};
use codecallnavigator::{
    graph::GraphIndex,
    model::{Edge, GraphData, Node},
};
use rune::{Env, Runtime};
use std::sync::Arc;

#[test]
fn fit_uses_the_full_canvas_with_filter_popover_closed() {
    let mut state = State::fixture();
    state.scene = true;
    state.jobs.boot.cancel();
    state.graph = Some(Arc::new(GraphIndex::new(Arc::new(data())).unwrap()));
    let graph = state.graph.as_ref().unwrap();
    let projects = projection::project(graph, &state.request().unwrap());
    state.nav.expanded.insert(projects.nodes[0].key.clone());
    let classes = projection::project(graph, &state.request().unwrap());
    let center = classes
        .nodes
        .iter()
        .find(|node| node.label == "Center")
        .unwrap();
    state.nav.focus_target = Some(projection::FocusTarget::Group(center.key.clone()));
    state.changed();
    let mut runtime = Runtime::new(state, view::view, (1050.0, 680.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    for _ in 0..4 {
        runtime.frame(1.0 / 60.0);
    }
    assert_eq!(runtime.state.graph_view.nodes.len(), 3);
    let elements = runtime.elements();
    let panel = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("graph-filters"))
        .unwrap();
    let canvas = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("graph-canvas"))
        .unwrap();
    assert!(
        panel.rect[1] + panel.rect[3] <= canvas.rect[1] + 0.5,
        "Filter trigger belongs above the canvas"
    );
    let camera = runtime.state.nav.camera.clone();
    click(&mut runtime, "graph-filters");
    runtime.frame(1.0 / 60.0);
    assert_eq!(runtime.state.nav.camera.pan, camera.pan);
    assert_eq!(runtime.state.nav.camera.zoom, camera.zoom);
    runtime.update(|state, _| state.close_overlay());
    runtime.frame(1.0 / 60.0);
    click(&mut runtime, "fit");
    for node in &runtime.state.graph_view.nodes {
        let top = runtime.state.nav.camera.screen(node.position);
        let bottom = top[1] + node.size[1] * runtime.state.nav.camera.zoom;
        assert!(top[1] >= 0.0, "{} extends above the canvas", node.label);
        assert!(
            bottom <= runtime.state.viewport[1],
            "{} extends below the canvas",
            node.label
        );
    }
}

fn data() -> GraphData {
    GraphData {
        nodes: ["Caller", "Center", "Sink"]
            .iter()
            .enumerate()
            .map(|(index, class)| Node {
                id: index.to_string(),
                build: "fixture".into(),
                project: "app".into(),
                container: (*class).into(),
                label: "run".into(),
                signature: "run()".into(),
                kind: "method".into(),
                ..Default::default()
            })
            .collect(),
        edges: [(0, 1), (1, 2)]
            .into_iter()
            .map(|(from, to)| Edge {
                evidence: Vec::new(),
                from,
                to,
                kind: "call".into(),
                certainty: "direct".into(),
                count: 1,
                provenance: String::new(),
            })
            .collect(),
        ..Default::default()
    }
}

/// Risk: Rune's implicit layout motion can lag the shared graph geometry during
/// direct pan. Only its real layout/presentation runtime can expose that second
/// animation; pure interpolation tests cannot observe presented card rectangles.
#[test]
fn presented_cards_follow_sampled_camera_without_a_second_layout_spring() {
    let mut runtime = super::runtime();
    runtime.settle_all();
    let node = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|node| runtime.state.nav.camera.shows(node, runtime.state.viewport))
        .unwrap()
        .clone();
    let key = format!("graph-card-{}", node.key);
    for frame in 1..8 {
        runtime.update(|state, _| {
            state.cancel_graph_camera();
            state.nav.camera.pan[0] += 7.0;
            state.nav.camera.pan[1] += 3.0;
        });
        runtime.frame(frame as f64 / 60.0);
        let (canvas, _) = runtime.presented("graph-canvas").unwrap();
        let (card, _) = runtime.presented(&key).unwrap();
        let point = runtime.state.nav.camera.screen(node.position);
        assert!(
            (card[0] - canvas[0] - point[0]).abs() < 0.1,
            "card X lags camera: card={:?} canvas={:?} point={:?} node={:?} camera={:?}",
            card,
            canvas,
            point,
            node.position,
            runtime.state.nav.camera
        );
        assert!(
            (card[1] - canvas[1] - point[1]).abs() < 0.1,
            "card Y lags camera: card={:?} canvas={:?} point={:?}",
            card,
            canvas,
            point
        );
    }
}
