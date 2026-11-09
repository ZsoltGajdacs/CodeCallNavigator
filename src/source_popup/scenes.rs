//! Explicit tooling scenes use an actual checked-in Java source, never normal startup.
use crate::{
    app::State,
    projection::{self, Grouping},
};
use codegraph::graph::GraphIndex;
use std::sync::Arc;

pub fn source(state: &mut State) {
    crate::scenes::install(state);
    let mut data = (*state.graph.as_ref().unwrap().data).clone();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tools/java-bytecode/src/main/java/codegraph/bytecode/SourceInventory.java"
    );
    for node in &mut data.nodes {
        node.file = path.into();
        node.language = "Java".into();
        node.build = concat!(env!("CARGO_MANIFEST_DIR"), "/tools/java-bytecode").into();
        node.line = 60;
    }
    state.graph = Some(Arc::new(GraphIndex::new(Arc::new(data)).unwrap()));
}

pub fn files(state: &mut State) {
    source(state);
    let graph = state.graph.as_ref().unwrap();
    let mut data = (*graph.data).clone();
    let folder = std::path::Path::new(&data.nodes[0].file).parent().unwrap();
    let mut paths: Vec<_> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "java"))
        .collect();
    paths.sort();
    let template = data.nodes[0].clone();
    data.nodes = paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| {
            let mut node = template.clone();
            node.id = format!("source-file-fixture-{index}");
            node.file = path.to_string_lossy().into_owned();
            node
        })
        .collect();
    data.edges.clear();
    data.entry_points = None;
    state.graph = Some(Arc::new(GraphIndex::new(Arc::new(data)).unwrap()));
    state.nav.focus = None;
    state.nav.grouping = Grouping::Namespace;
    state.graph_view =
        projection::project(state.graph.as_ref().unwrap(), &state.request().unwrap());
    crate::scenes::position(&mut state.graph_view);
    state.nav.camera.fit(state.graph_view.size, state.viewport);
}
