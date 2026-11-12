//! Explicit Fit uses the full canvas beneath the floating filter controls.
use super::click;
use crate::{app::State, jobs, projection, view};
use codecallnavigator::{
    graph::GraphIndex,
    model::{Edge, GraphData, Node},
};
use rune::{Env, Runtime};
use std::sync::Arc;

#[test]
fn fit_uses_the_full_canvas_beneath_wrapped_filters() {
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
    assert!(panel.rect[3] > 60.0, "Filters should wrap at minimum width");
    assert!(
        (canvas.rect[1] - panel.rect[1]).abs() < 0.5,
        "Filters must float over the top of the canvas"
    );
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
