//! Acceptance against the pinned Roslyn analyzer, not fabricated compiler metadata.
use codegraph::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::BuildTarget,
};
use std::{fs, sync::atomic::AtomicBool};

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET SDK"]
fn source_callables_exclude_object_boilerplate_without_losing_overloads() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("Policy.csproj");
    fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net10.0</TargetFramework><OutputType>Exe</OutputType></PropertyGroup></Project>").unwrap();
    fs::write(
        root.path().join("Program.cs"),
        include_str!("../fixtures/dotnet-policy/Program.cs"),
    )
    .unwrap();
    let graph = analyze(
        vec![BuildTarget {
            path: project,
            language: "csharp".into(),
            kind: "project".into(),
            label: "policy".into(),
        }],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    graph.validate().unwrap();
    for node in &graph.nodes {
        assert!(
            !matches!(
                node.kind.as_str(),
                "constructor" | "cctor" | "fieldinit" | "field" | "delegate"
            ),
            "Unexpected node {}",
            node.id
        );
        assert!(
            !["::ToString()", "::GetHashCode()", "::Equals(System.Object)"]
                .iter()
                .any(|suffix| node.id.ends_with(suffix)),
            "Object member {}",
            node.id
        );
    }
    for suffix in [
        "Worker::Private()",
        "Worker::Package()",
        "Worker::Protected()",
        "Worker::ToString(System.Int32)",
        "Worker::GetHashCode(System.Int32)",
        "Worker::Equals(System.Int32)",
        "Worker::get_Value()",
        "Worker::get_Automatic()",
        "Worker::set_Automatic(System.Int32)",
    ] {
        assert!(
            graph.nodes.iter().any(|node| node.id.ends_with(suffix)),
            "Missing {suffix}"
        );
    }
    let only = graph
        .nodes
        .iter()
        .find(|node| node.id.ends_with("Worker::ConstructorOnly()"))
        .unwrap();
    assert!(
        graph
            .entry_points
            .as_ref()
            .unwrap()
            .candidates
            .contains(&only.id)
    );
}
