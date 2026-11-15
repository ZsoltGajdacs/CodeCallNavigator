use std::sync::atomic::AtomicBool;

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET SDK"]
fn real_csharp_entries_keep_uncalled_methods_and_verify_executable_starts() {
    use codecallnavigator_engine::{
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
