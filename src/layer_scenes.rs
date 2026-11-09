//! Explicit architecture fixture using the production Graphviz band layout.
use crate::{
    app::{Navigation, State},
    projection::{self, Grouping},
};
use codegraph::{
    analyzers::AnalyzerRuntime,
    architecture::{Assignment, LayerAssignments, LayerDefinition, Selection, file_key},
};
use std::sync::Arc;

pub fn install(state: &mut State) {
    crate::scenes::install(state);
    state.scene = false;
    state.deterministic = true;
    state.nav = Navigation::default();
    state.nav.set_grouping(Grouping::Layer);
    state.nav.filter.include_generated = true;
    state.nav.filter.include_tests = true;
    state.architecture_selection = Some(Selection::Repository);
    let graph = state.graph.as_ref().unwrap().clone();
    let layer = |name: &str| match name {
        "Start" => "Controller",
        "SaveOrder" => "Repository",
        "GeneratedMapper" => "Utilities",
        "OrderTest" => "Tests",
        _ => "Service",
    };
    state.architecture = Some(Arc::new(LayerAssignments {
        files: graph
            .data
            .nodes
            .iter()
            .map(|node| {
                (
                    file_key(&node.build, &node.project, &node.file),
                    Assignment {
                        layer: layer(&node.label).into(),
                        rule: "Synthetic scene".into(),
                    },
                )
            })
            .collect(),
        layers: ["Controller", "Service", "Repository", "Utilities"]
            .into_iter()
            .map(|name| LayerDefinition {
                name: name.into(),
                support: name == "Utilities",
                ..Default::default()
            })
            .collect(),
        origin: "Synthetic architectural fixture · no project scanned".into(),
    }));
    for _ in 0..3 {
        let view = projection::project(&graph, &state.request().unwrap());
        state.nav.expanded.extend(
            view.nodes
                .into_iter()
                .filter(|node| node.member.is_none())
                .map(|node| node.key),
        );
    }
    let result = (|| {
        let runtime = Arc::new(AnalyzerRuntime::discover()?);
        state.graph_view = projection::layout(
            projection::project(&graph, &state.request().unwrap()),
            &runtime.dot,
        )?;
        state.runtime = Some(runtime);
        state.nav.camera.fit(state.graph_view.size, state.viewport);
        state.visible_revision = Some((state.epoch, state.revision));
        state.auto_fit = false;
        state.status =
            "Architectural fixture · production Graphviz bands · source graph unchanged".into();
        Ok::<(), anyhow::Error>(())
    })();
    if let Err(error) = result {
        state.warning = format!("Cannot render architecture fixture: {error:#}");
    }
}
