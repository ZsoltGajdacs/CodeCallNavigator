//! Explicit branching fixtures rendered with the production Graphviz layout.
use crate::runtime::Runtime as AnalyzerRuntime;
use crate::{
    app::{Navigation, State},
    projection::{self, FlowDirection, Grouping},
};
use codecallnavigator::{
    graph::GraphIndex,
    model::{Edge, EntryPoints, GraphData, Node},
};
use std::sync::Arc;

pub fn callees(state: &mut State) {
    install(state, FlowDirection::Callees);
}

pub fn callers(state: &mut State) {
    install(state, FlowDirection::Callers);
}

fn install(state: &mut State, direction: FlowDirection) {
    state.deterministic = true;
    state.jobs.boot.cancel();
    state.root = rune::TextEdit::new("Branching method fixture · no project scanned");
    state.nav = Navigation::default();
    state.nav.set_grouping(Grouping::Method);
    state.nav.set_direction(direction);
    state.nav.depth = 4;
    if direction == FlowDirection::Callers {
        state.nav.focus_method("fixture/6".into());
    }
    let result = (|| {
        let runtime = Arc::new(AnalyzerRuntime::discover()?);
        let graph = Arc::new(GraphIndex::new(Arc::new(data()))?);
        state.graph = Some(graph.clone());
        state.replace_graph_view(projection::layout(
            projection::project(&graph, &state.request().unwrap()),
            &runtime.dot,
        )?);
        state.runtime = Some(runtime);
        state.nav.camera.fit(state.graph_view.size, state.viewport);
        state.visible_revision = Some((state.epoch, state.revision));
        state.auto_fit = false;
        state.status =
            "Branching fixture · real Graphviz geometry · shared methods and recursion".into();
        Ok::<(), anyhow::Error>(())
    })();
    if let Err(error) = result {
        state.warning = format!("Cannot render method fixture: {error:#}");
    }
}

fn data() -> GraphData {
    let names = [
        "Main",
        "Load",
        "Validate",
        "ReadFile",
        "ReadCache",
        "CheckRules",
        "Store",
        "Disconnected",
    ];
    GraphData {
        nodes: names
            .iter()
            .enumerate()
            .map(|(index, name)| Node {
                id: format!("fixture/{index}"),
                label: (*name).into(),
                project: "Example application".into(),
                container: "Application".into(),
                signature: format!("Application::{name}()"),
                kind: "method".into(),
                ..Default::default()
            })
            .collect(),
        edges: [
            (0, 1),
            (0, 2),
            (1, 3),
            (1, 4),
            (2, 5),
            (3, 6),
            (4, 6),
            (5, 6),
            (6, 6),
        ]
        .into_iter()
        .map(|(from, to)| Edge {
            evidence: Vec::new(),
            provenance: String::new(),
            from,
            to,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
        })
        .collect(),
        entry_points: Some(EntryPoints {
            framework_entries: Vec::new(),
            candidates: vec!["fixture/0".into(), "fixture/7".into()],
            application_starts: vec!["fixture/0".into()],
        }),
        engine_versions: "Explicit method fixture · no analyzer executed".into(),
        ..Default::default()
    }
}
