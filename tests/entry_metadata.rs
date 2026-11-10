use codegraph::{
    model::{EntryPoints, GraphData, Node},
    store::SqliteStore,
};
use std::sync::atomic::AtomicBool;

fn graph() -> GraphData {
    GraphData {
        nodes: vec![Node {
            id: "main".into(),
            ..Default::default()
        }],
        entry_points: Some(EntryPoints {
            framework_entries: Vec::new(),
            candidates: vec!["main".into()],
            application_starts: vec!["main".into()],
        }),
        ..Default::default()
    }
}

#[test]
fn entry_metadata_survives_sqlite_and_old_graphs_remain_readable() {
    let root = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(root.path()).unwrap();
    let data = graph();
    store.publish(&data, &AtomicBool::new(false)).unwrap();
    let loaded = store.load_active().unwrap().unwrap();
    assert_eq!(loaded.entry_points, data.entry_points);
    let mut legacy = serde_json::to_value(&data).unwrap();
    legacy.as_object_mut().unwrap().remove("entry_points");
    let legacy: GraphData = serde_json::from_value(legacy).unwrap();
    assert!(legacy.entry_points.is_none());
    let generation = store.publish(&legacy, &AtomicBool::new(false)).unwrap();
    // A pre-feature SQLite generation has no entry_points metadata key at all.
    let connection = rusqlite::Connection::open(root.path().join(generation)).unwrap();
    connection
        .execute(
            "UPDATE metadata SET value=json_remove(value, '$.entry_points') WHERE key='graph'",
            [],
        )
        .unwrap();
    assert!(store.load_active().unwrap().unwrap().entry_points.is_none());
}

#[test]
fn metadata_rejects_unknown_or_duplicate_entry_identities() {
    let mut data = graph();
    data.entry_points
        .as_mut()
        .unwrap()
        .candidates
        .push("missing".into());
    assert!(data.validate().is_err());
    let mut data = graph();
    data.entry_points
        .as_mut()
        .unwrap()
        .application_starts
        .push("main".into());
    assert!(data.validate().is_err());
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET SDK"]
fn real_csharp_entries_keep_uncalled_methods_and_verify_executable_starts() {
    use codegraph_engine::{
        analyzers::{AnalyzerRuntime, analyze},
        discovery::BuildTarget,
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let build = |path: &str| BuildTarget {
        path: root.join(path),
        language: "csharp".into(),
        kind: "solution".into(),
        label: path.into(),
    };
    let graph = analyze(
        vec![
            build("fixtures/dotnet/Fixture.sln"),
            build("fixtures/dotnet-collisions/Collisions.sln"),
        ],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    let entries = graph.entry_points.as_ref().unwrap();
    assert_eq!(entries.application_starts.len(), 5);
    assert_eq!(
        entries
            .application_starts
            .iter()
            .filter(|id| id.ends_with("Program::<top-level>"))
            .count(),
        2
    );
    let disconnected = graph
        .nodes
        .iter()
        .find(|node| node.id.ends_with("Fixture.Worker::Disconnected()"))
        .unwrap();
    assert!(entries.candidates.contains(&disconnected.id));
    assert!(!entries.application_starts.contains(&disconnected.id));
    assert!(
        entries
            .application_starts
            .iter()
            .all(|id| graph.nodes.iter().any(|node| &node.id == id))
    );
}

#[test]
fn framework_entries_and_edge_provenance_survive_cache_without_becoming_starts() {
    use codegraph::model::Edge;
    let root = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(root.path()).unwrap();
    let mut data = graph();
    let entries = data.entry_points.as_mut().unwrap();
    entries.application_starts.clear();
    entries.framework_entries.push("main".into());
    data.edges.push(Edge {
        from: 0,
        to: 0,
        kind: "react-render".into(),
        certainty: "framework-possible".into(),
        count: 1,
        provenance: "react-rules.v1:jsx".into(),
    });
    let generation = store.publish(&data, &AtomicBool::new(false)).unwrap();
    let loaded = store.load_active().unwrap().unwrap();
    assert_eq!(loaded.entry_points, data.entry_points);
    assert!(loaded.entry_points.unwrap().application_starts.is_empty());
    assert_eq!(loaded.edges[0].provenance, "react-rules.v1:jsx");
    let connection = rusqlite::Connection::open(root.path().join(generation)).unwrap();
    connection.execute("UPDATE metadata SET value=json_remove(value, '$.edge_provenance', '$.entry_points.framework_entries') WHERE key='graph'", []).unwrap();
    let loaded = store.load_active().unwrap().unwrap();
    assert!(loaded.edges[0].provenance.is_empty());
    assert!(loaded.entry_points.unwrap().framework_entries.is_empty());
}
