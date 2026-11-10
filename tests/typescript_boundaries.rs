//! Installed Next/TypeScript bindings and the shared graph/flow boundary overlay.
use codegraph::boundary::{self, Configuration};
use codegraph_engine::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::target_from_file,
};
use std::{fs, os::unix::fs::symlink, path::Path, sync::atomic::AtomicBool};

fn copy_sources(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_sources(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
fn flow_targets(value: &serde_json::Value) -> Vec<serde_json::Value> {
    let mut targets = Vec::new();
    if value.get("span").is_some()
        && let Some(values) = value.get("targets").and_then(|value| value.as_array())
    {
        targets.extend(
            values
                .iter()
                .filter(|value| value["provenance"] == "codegraph.boundary.v1")
                .cloned(),
        );
    }
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                targets.extend(flow_targets(value));
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                targets.extend(flow_targets(value));
            }
        }
        _ => {}
    }
    targets
}
#[test]
#[ignore = "requires installed Node24, TypeScript6, React19 and Next16 fixture dependencies"]
fn next_navigation_and_fetch_agree_with_flow_without_synchronous_call_edges() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = root.join("fixtures/typescript-corpus");
    let temp = tempfile::tempdir().unwrap();
    copy_sources(&fixture.join("src"), &temp.path().join("src"));
    fs::copy(
        fixture.join("tsconfig.json"),
        temp.path().join("tsconfig.json"),
    )
    .unwrap();
    let modules = std::env::var_os("CODEGRAPH_TS_NODE_MODULES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("../PacificFateMapEditor/node_modules"));
    symlink(
        modules.canonicalize().unwrap(),
        temp.path().join("node_modules"),
    )
    .unwrap();
    let target = target_from_file(&temp.path().join("tsconfig.json")).unwrap();
    let mut graph = analyze(
        vec![target],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    let original = graph.edges.clone();
    boundary::resolve(&mut graph, &Configuration::default()).unwrap();
    let added = graph
        .edges
        .iter()
        .filter(|edge| edge.provenance == "codegraph.boundary.v1")
        .collect::<Vec<_>>();
    assert_eq!(added.len(), 4);
    assert_eq!(
        added
            .iter()
            .filter(|edge| edge.kind == "navigation")
            .count(),
        3
    );
    let existing = graph
        .edges
        .iter()
        .map(|edge| serde_json::to_value(edge).unwrap())
        .collect::<Vec<_>>();
    assert!(
        original
            .iter()
            .all(|edge| existing.contains(&serde_json::to_value(edge).unwrap()))
    );
    let flow = flow_targets(&serde_json::to_value(&graph.flow_inventory).unwrap());
    assert_eq!(flow.len(), 3); // two router invocations and builtin fetch; JSX Link is graph-only.
    for target in flow {
        let edge = added
            .iter()
            .find(|edge| {
                graph.nodes[edge.to as usize].id == target["symbol"] && edge.kind == target["kind"]
            })
            .unwrap();
        assert_eq!(edge.certainty, "possible");
    }
    assert!(
        graph
            .diagnostics
            .iter()
            .any(|value| value.contains("http boundary unresolved"))
    );
}
