//! Explicit class and method fixtures exercising production traversal and Graphviz geometry.
use crate::{
    app::{Navigation, State},
    projection::{self, FocusTarget, Grouping},
};
use codegraph::{
    analyzers::AnalyzerRuntime,
    graph::GraphIndex,
    model::{Edge, GraphData, Node},
};
use std::sync::Arc;

pub fn classes(state: &mut State) {
    install(state, false, Grouping::Project);
}

pub fn methods(state: &mut State) {
    install(state, true, Grouping::Project);
}

pub fn aligned_classes(state: &mut State) {
    install(state, false, Grouping::Folder);
}

fn install(state: &mut State, method: bool, grouping: Grouping) {
    state.deterministic = true;
    state.jobs.boot.cancel();
    state.root = rune::TextEdit::new("Centered traversal fixture · no project scanned");
    state.nav = Navigation::default();
    state.nav.grouping = grouping;
    state.nav.depth = 1;
    let result = (|| {
        let runtime = Arc::new(AnalyzerRuntime::discover()?);
        let graph = Arc::new(GraphIndex::new(Arc::new(data()))?);
        state.graph = Some(graph.clone());
        let project_view = projection::project(&graph, &state.request().unwrap());
        let project = project_view.nodes.first().expect("fixture project");
        state.nav.expanded.insert(project.key.clone());
        // Only the center's branch is expanded; connected callers and callees
        // live under collapsed folders, while a caller class was expanded before.
        if grouping == Grouping::Folder {
            state.nav.expanded.insert(
                serde_json::to_string(&["folder", "fixture", "Order application", "services"])
                    .unwrap(),
            );
            state.nav.expanded.insert(
                serde_json::to_string(&[
                    "class",
                    "fixture",
                    "Order application",
                    "OrderController",
                ])
                .unwrap(),
            );
        }
        state.nav.focus_target = Some(if method {
            FocusTarget::Method("traversal/1".into())
        } else {
            FocusTarget::Group(
                serde_json::to_string(&["class", "fixture", "Order application", "OrderService"])
                    .unwrap(),
            )
        });
        state.nav.selected = method.then(|| "traversal/1".into());
        let view = projection::layout(
            projection::project(&graph, &state.request().unwrap()),
            &runtime.dot,
        )?;
        state.auto_fit = false;
        state.nav.camera.zoom = if method { 1.0 } else { 0.78 };
        state.graph_interaction.center_after_layout = true;
        state.complete_graph_layout(view);
        state.runtime = Some(runtime);
        state.refresh_neighbors();
        state.status = "Centered traversal fixture · real Graphviz geometry · overloads and scrollable classes".into();
        Ok::<(), anyhow::Error>(())
    })();
    if let Err(error) = result {
        state.warning = format!("Cannot render traversal fixture: {error:#}");
    }
}

fn data() -> GraphData {
    let mut nodes = vec![node(0, "OrderController", "submit", "submit(Request)")];
    for (index, signature) in [
        "submit(Request)",
        "submit(Request, Options)",
        "validate(Order)",
        "find(Id)",
        "find(Id, Options)",
        "reserve(Order)",
        "release(Order)",
        "cancel(Id)",
        "confirm(Id)",
        "dispatch(Order)",
        "refund(Id)",
        "list(Customer)",
        "notify(Customer)",
        "audit(Order)",
        "summarize(Order)",
        "archive(Id)",
    ]
    .iter()
    .enumerate()
    {
        let label = signature.split('(').next().expect("fixture signature");
        nodes.push(node(index + 1, "OrderService", label, signature));
    }
    nodes.push(node(17, "OrderRepository", "save", "save(Order)"));
    nodes.push(node(18, "Metrics", "sample", "sample()"));
    let edges = [(0, 1), (0, 2), (1, 3), (1, 17), (2, 17), (3, 17)]
        .into_iter()
        .map(|(from, to)| Edge {
            from,
            to,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
            provenance: String::new(),
        })
        .collect();
    GraphData {
        nodes,
        edges,
        ..Default::default()
    }
}

fn node(index: usize, container: &str, label: &str, signature: &str) -> Node {
    Node {
        id: format!("traversal/{index}"),
        build: "fixture".into(),
        project: "Order application".into(),
        namespace: "example.orders".into(),
        container: container.into(),
        label: label.into(),
        signature: signature.into(),
        kind: "method".into(),
        language: "java".into(),
        file: format!(
            "fixture/{}/{container}.java",
            match container {
                "OrderController" => "api",
                "OrderService" => "services",
                _ => "utilities",
            }
        ),
        ..Default::default()
    }
}
