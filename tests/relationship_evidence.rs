use codegraph::{
    artifact::{canonical, compare},
    flow::SourceSpan,
    method_policy,
    model::{Edge, GraphData, Node, RelationshipEvidence, SourceSite},
    store::SqliteStore,
};
use std::{collections::HashSet, path::PathBuf, sync::atomic::AtomicBool};

fn graph(root: &str) -> GraphData {
    let nodes: Vec<_> = ["Caller", "Target", "Support"]
        .into_iter()
        .map(|name| Node {
            id: format!("{root}/App.csproj|{name}()"),
            project: "App".into(),
            build: format!("{root}/App.csproj"),
            language: "csharp".into(),
            kind: "method".into(),
            label: name.into(),
            signature: format!("{name}()"),
            file: format!("{root}/Main.cs"),
            ..Default::default()
        })
        .collect();
    let supporting_symbols = nodes.iter().map(|node| node.id.clone()).collect();
    GraphData {
        nodes,
        edges: vec![Edge {
            from: 0,
            to: 1,
            kind: "framework".into(),
            certainty: "possible".into(),
            count: 2,
            provenance: "framework.rule".into(),
            evidence: vec![RelationshipEvidence {
                rule: "framework.rule".into(),
                version: 1,
                semantics: "possible-invocation".into(),
                invocation: SourceSite {
                    file: format!("{root}/Main.cs"),
                    line: 12,
                    column: Some(5),
                    span: Some(SourceSpan { start: 10, end: 20 }),
                },
                registration: Some(SourceSite {
                    file: format!("{root}/Main.cs"),
                    line: 5,
                    column: None,
                    span: None,
                }),
                supporting_symbols,
                channel: Some("update".into()),
            }],
        }],
        ..Default::default()
    }
}

fn roots(root: &str) -> Vec<(String, PathBuf)> {
    vec![("repo".into(), root.into())]
}

#[test]
fn evidence_paths_and_supporting_identities_are_portable_and_comparable() {
    let before = canonical(&graph("/tmp/first"), &roots("/tmp/first")).unwrap();
    let mut reordered = graph("/tmp/second");
    reordered.edges[0].evidence[0].supporting_symbols.reverse();
    let after = canonical(&reordered, &roots("/tmp/second")).unwrap();
    assert_eq!(before.bytes().unwrap(), after.bytes().unwrap());
    let hydrated = after.hydrate(&roots("/tmp/local")).unwrap();
    let evidence = &hydrated.edges[0].evidence[0];
    assert_eq!(evidence.invocation.file, "/tmp/local/Main.cs");
    assert_eq!(
        evidence.registration.as_ref().unwrap().file,
        "/tmp/local/Main.cs"
    );
    assert!(
        evidence
            .supporting_symbols
            .iter()
            .all(|id| hydrated.nodes.iter().any(|node| &node.id == id))
    );
    let mut changed = graph("/tmp/first");
    changed.edges[0].evidence[0].invocation.line = 13;
    let result = compare(&before, &canonical(&changed, &roots("/tmp/first")).unwrap()).unwrap();
    assert!(!result.equal);
    assert_eq!(result.edges.changed.len(), 1);
    assert!(result.nodes.changed.is_empty());
    assert_eq!(changed.edges[0].count, 2);
}

#[test]
fn invalid_evidence_is_rejected_without_hiding_graph_relationships() {
    let valid = graph("/tmp/project");
    valid.validate().unwrap();
    let invalid = [
        ("rule", serde_json::json!(" ")),
        ("version", serde_json::json!(2)),
        ("semantics", serde_json::json!("runtime-observed")),
        ("supporting_symbols", serde_json::json!(["missing"])),
        ("invocation", serde_json::json!({"file":"main.cs","line":0})),
        (
            "invocation",
            serde_json::json!({"file":"main.cs","line":1,"column":0}),
        ),
        (
            "invocation",
            serde_json::json!({"file":"main.cs","line":1,"span":{"start":3,"end":3}}),
        ),
        ("registration", serde_json::json!({"file":"","line":1})),
    ];
    for (field, value) in invalid {
        let mut graph = valid.clone();
        let mut evidence = serde_json::to_value(&graph.edges[0].evidence[0]).unwrap();
        evidence[field] = value;
        graph.edges[0].evidence[0] = serde_json::from_value(evidence).unwrap();
        assert!(graph.validate().is_err(), "Accepted malformed {field}");
    }
}

#[test]
fn pruning_supporting_declarations_retains_surviving_edge_and_source_evidence() {
    let mut graph = graph("/tmp/project");
    let removed = graph.nodes[2].id.clone();
    method_policy::prune(&mut graph, &HashSet::from([removed.clone()])).unwrap();
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].count, 2);
    let evidence = &graph.edges[0].evidence[0];
    assert!(!evidence.supporting_symbols.contains(&removed));
    assert_eq!(evidence.supporting_symbols.len(), 2);
    assert_eq!(evidence.invocation.line, 12);
    graph.validate().unwrap();
}

#[test]
fn evidence_roundtrips_in_sqlite_and_legacy_edges_default_empty() {
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(directory.path()).unwrap();
    let graph = graph("/tmp/project");
    let generation = store.publish(&graph, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        store.load_active().unwrap().unwrap().edges[0].evidence,
        graph.edges[0].evidence
    );
    let connection = rusqlite::Connection::open(directory.path().join(generation)).unwrap();
    let raw: String = connection
        .query_row("SELECT value FROM metadata WHERE key='graph'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut metadata: serde_json::Value = serde_json::from_str(&raw).unwrap();
    metadata.as_object_mut().unwrap().remove("edge_evidence");
    connection
        .execute(
            "UPDATE metadata SET value=? WHERE key='graph'",
            [metadata.to_string()],
        )
        .unwrap();
    assert!(
        store.load_active().unwrap().unwrap().edges[0]
            .evidence
            .is_empty()
    );
    metadata["edge_evidence"] = serde_json::json!([]);
    connection
        .execute(
            "UPDATE metadata SET value=? WHERE key='graph'",
            [metadata.to_string()],
        )
        .unwrap();
    assert!(store.load_active().is_err());
    let mut edge = serde_json::to_value(&graph.edges[0]).unwrap();
    edge.as_object_mut().unwrap().remove("evidence");
    let legacy: Edge = serde_json::from_value(edge).unwrap();
    assert!(legacy.evidence.is_empty());
    assert!(
        serde_json::to_value(legacy)
            .unwrap()
            .get("evidence")
            .is_none()
    );
}
