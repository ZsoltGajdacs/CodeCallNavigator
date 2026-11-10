//! Explicit deterministic tooling scene; normal startup always uses the user's workspace.
use crate::{
    app::State,
    projection::{self, GraphView},
};
use codegraph::{
    graph::GraphIndex,
    model::{Edge, EntryPoints, GraphData, Node},
};
use std::sync::Arc;

pub fn install(state: &mut State) {
    state.scene = true;
    state.jobs.boot.cancel();
    state.root = rune::TextEdit::new("Test fixture · no project scanned");
    state.graph = Some(Arc::new(
        GraphIndex::new(Arc::new(fixture_data())).expect("valid fixture"),
    ));
    state.nav.focus = Some("fixture/project/Start".into());
    state.nav.depth = 4;
    state.status = "Deterministic test fixture · 6 symbols · 6 relationships".into();
    state.replace_graph_view(projection::project(
        state.graph.as_ref().unwrap(),
        &state.request().unwrap(),
    ));
    position(&mut state.graph_view);
    state.nav.camera.fit(state.graph_view.size, state.viewport);
    state.auto_fit = false;
    state.visible_revision = Some((state.epoch, state.revision));
}

fn fixture_data() -> GraphData {
    GraphData {
        entry_points: Some(EntryPoints {
            framework_entries: Vec::new(),
            candidates: vec![
                "fixture/project/Start".into(),
                "fixture/project/OrderTest".into(),
            ],
            application_starts: vec!["fixture/project/Start".into()],
        }),
        nodes: fixture_nodes(),
        edges: fixture_edges(),
        engine_versions: "Deterministic fixture · no analyzer executed".into(),
        diagnostics: vec![
            "This is the explicit headless test fixture. No real project has been analyzed.".into(),
        ],
        ..Default::default()
    }
}

fn fixture_nodes() -> Vec<Node> {
    let names = [
        "Start",
        "LoadOrder",
        "ValidateOrder",
        "SaveOrder",
        "GeneratedMapper",
        "OrderTest",
    ];
    names
        .iter()
        .enumerate()
        .map(|(index, name)| Node {
            id: format!("fixture/project/{name}"),
            project: "Fixture application".into(),
            build: "fixture".into(),
            language: "C#".into(),
            kind: "method".into(),
            label: name.to_string(),
            signature: format!("Orders::{name}()"),
            namespace: "Fixture".into(),
            container: "Orders".into(),
            file: format!("fixture/{name}.cs"),
            line: 1,
            is_test: index == 5,
            is_generated: index == 4,
            body_unavailable: false,
            ..Default::default()
        })
        .collect()
}

fn fixture_edges() -> Vec<Edge> {
    [(0, 1), (1, 2), (2, 3), (3, 2), (1, 4), (5, 1)]
        .into_iter()
        .map(|(from, to)| Edge {
            provenance: String::new(),
            from,
            to,
            kind: if (from, to) == (2, 3) {
                "methodgroup"
            } else {
                "call"
            }
            .into(),
            certainty: if (from, to) == (1, 2) {
                "dispatch"
            } else {
                "direct"
            }
            .into(),
            count: 1,
        })
        .collect()
}

pub fn position(view: &mut GraphView) {
    use std::collections::BTreeMap;
    let mut rows: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for (index, order) in view.ordering.iter().enumerate() {
        rows.entry(order.rank).or_default().push(index);
    }
    let mut y = 30.0;
    let mut width: f32 = 400.0;
    for indices in rows.values() {
        let mut x = 30.0;
        let mut height: f32 = 61.2;
        for &index in indices {
            let node = &mut view.nodes[index];
            if node.size[0] == 0.0 {
                node.size = [237.6, 61.2];
            }
            node.position = [x, y];
            x += node.size[0] + 50.0;
            height = height.max(node.size[1]);
        }
        width = width.max(x);
        y += height + 80.0;
    }
    for edge in &mut view.edges {
        let from = &view.nodes[edge.from];
        let to = &view.nodes[edge.to];
        edge.points = vec![
            [
                from.position[0] + from.size[0] / 2.0,
                from.position[1] + from.size[1],
            ],
            [to.position[0] + to.size[0] / 2.0, to.position[1]],
        ];
    }
    view.size = [width, y.max(300.0)];
}

/// Explicit tooling scene: open the actual published SQLite generation synchronously
/// so virtual-clock screenshots cannot race the normal background loader.
pub fn cached(state: &mut State) {
    cached_scene(state, false);
}

pub fn cached_focus(state: &mut State) {
    cached_scene(state, true);
}

fn cached_scene(state: &mut State, focus: bool) {
    state.deterministic = true;
    state.jobs.boot.cancel();
    let result = (|| {
        let boot = crate::jobs::load_boot(crate::jobs::data_directory())?;
        let loaded = boot
            .graph
            .ok_or_else(|| anyhow::anyhow!("No published graph in CODEGRAPH_DATA_DIR"))?;
        state.graph = Some(loaded.graph);
        state.generation = Some(loaded.generation);
        state.runtime = Some(boot.runtime);
        state.store = Some(boot.store);
        state.warning = boot.warnings.join("\n");
        state.nav = crate::app::Navigation::default();
        if focus {
            select_cached_method(state);
        }
        let request = state.request().unwrap();
        let graph = state.graph.as_ref().unwrap().clone();
        state.replace_graph_view(projection::layout(
            projection::project(&graph, &request),
            &state.runtime.as_ref().unwrap().dot,
        )?);
        state.nav.camera.fit(state.graph_view.size, state.viewport);
        state.visible_revision = Some((state.epoch, state.revision));
        state.next_stale_check = f64::MAX;
        state.auto_fit = false;
        state.status = format!(
            "Last successful analysis · {} symbols · {} relationships",
            graph.data.nodes.len(),
            graph.data.edges.len()
        );
        Ok::<(), anyhow::Error>(())
    })();
    if let Err(error) = result {
        state.warning = format!("Cannot open cached tooling scene: {error:#}");
    }
}

fn select_cached_method(state: &mut State) {
    let Some(graph) = &state.graph else {
        return;
    };
    let target = graph.data.nodes.iter().find(|node| {
        node.file.ends_with("AdvancedExtractionJobFlow.java")
            && node.signature.contains(
                ":execute(eu.zsgw.storyengine.extraction.flow.AdvancedExtractionJobFlowContext)",
            )
    });
    if let Some(target) = target {
        state.nav.selected = Some(target.id.clone());
        state.nav.focus = Some(target.id.clone());
        state.nav.direction = crate::projection::FlowDirection::Callees;
        state.nav.depth = 2;
        state.refresh_neighbors();
    }
}
