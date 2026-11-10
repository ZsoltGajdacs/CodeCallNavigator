//! Acceptance against the pinned Roslyn analyzer, not fabricated compiler metadata.
use codegraph::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::BuildTarget,
    model::Visibility,
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
    assert_no_boilerplate(&graph.nodes);
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
    assert_visibility(&graph.nodes);
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

fn assert_no_boilerplate(nodes: &[codegraph::model::Node]) {
    for node in nodes {
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
}

fn assert_visibility(nodes: &[codegraph::model::Node]) {
    for (suffix, expected) in [
        ("Worker::Private()", Visibility::Private),
        ("Worker::DefaultPrivate()", Visibility::Private),
        ("Worker::Package()", Visibility::Internal),
        ("Worker::Protected()", Visibility::Protected),
        ("Worker::ProtectedInternal()", Visibility::ProtectedInternal),
        ("Worker::PrivateProtected()", Visibility::PrivateProtected),
        ("Worker::Start()", Visibility::Public),
    ] {
        let node = nodes.iter().find(|node| node.id.ends_with(suffix)).unwrap();
        assert_eq!(node.visibility, Some(expected), "{suffix}");
        assert_eq!(
            node.container_visibility,
            Some(Visibility::Internal),
            "{suffix}"
        );
    }
    for (container, expected) in [
        ("Worker.PrivateNested", Visibility::Private),
        ("Worker.ProtectedNested", Visibility::Protected),
        ("PublicVisibilityOwner", Visibility::Public),
    ] {
        let node = nodes
            .iter()
            .find(|node| node.container == container && node.label == "VisibleMember")
            .unwrap();
        assert_eq!(node.visibility, Some(Visibility::Public), "{container}");
        assert_eq!(node.container_visibility, Some(expected), "{container}");
    }
    let local = nodes
        .iter()
        .find(|node| node.kind == "localfunction")
        .unwrap();
    assert_eq!(local.visibility, None);
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET SDK"]
fn architecture_facts_include_attributes_inheritance_and_type_only_files() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("Architecture.csproj");
    fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup></Project>").unwrap();
    fs::write(
        root.path().join("Store.cs"),
        r#"using System;
namespace Architecture;
[AttributeUsage(AttributeTargets.All)] public sealed class MarkerAttribute : Attribute {}
[Marker] public interface IStore { void Save(); }
public sealed class Store : IStore { [Marker] public void Save() {} }
"#,
    )
    .unwrap();
    fs::write(
        root.path().join("Payload.cs"),
        "namespace Architecture; public record Payload(int Value);",
    )
    .unwrap();
    let graph = analyze(
        vec![BuildTarget {
            path: project,
            language: "csharp".into(),
            kind: "project".into(),
            label: "architecture".into(),
        }],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    let architecture = graph.architecture.unwrap();
    assert_eq!(architecture.version, 1);
    let store = architecture
        .files
        .iter()
        .find(|file| file.file.ends_with("Store.cs"))
        .unwrap();
    for (values, expected) in [
        (&store.annotations, "MarkerAttribute"),
        (&store.method_annotations, "MarkerAttribute"),
        (&store.base_types, "IStore"),
        (&store.annotated_interfaces, "MarkerAttribute"),
    ] {
        assert!(
            values.iter().any(|value| value == expected),
            "Missing {expected} in {values:?}"
        );
    }
    let payload = architecture
        .files
        .iter()
        .find(|file| file.file.ends_with("Payload.cs"))
        .unwrap();
    assert!(payload.kinds.iter().any(|kind| kind == "record"));
    assert!(payload.content.contains("record Payload"));
    assert!(payload.namespaces.iter().any(|name| name == "Architecture"));
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET SDK"]
fn linked_source_retains_each_compiler_projects_facts_and_node_ownership() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("Shared.cs");
    fs::write(
        &source,
        r#"using System;
namespace Shared;
[AttributeUsage(AttributeTargets.All)] public class FirstAttribute : Attribute {}
[AttributeUsage(AttributeTargets.All)] public class SecondAttribute : Attribute {}
#if FIRST
[First]
#else
[Second]
#endif
public class Service { public void Run() {} }
"#,
    )
    .unwrap();
    for (project, constants) in [("A", "FIRST"), ("B", "SECOND")] {
        let folder = root.path().join(project);
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join(format!("{project}.csproj")), format!(r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net10.0</TargetFramework><DefineConstants>{constants}</DefineConstants></PropertyGroup><ItemGroup><Compile Include="../Shared.cs" Link="Shared.cs" /></ItemGroup></Project>"#)).unwrap();
    }
    let solution = root.path().join("Linked.slnx");
    fs::write(
        &solution,
        r#"<Solution><Project Path="A/A.csproj" /><Project Path="B/B.csproj" /></Solution>"#,
    )
    .unwrap();
    let graph = analyze(
        vec![BuildTarget {
            path: solution,
            language: "csharp".into(),
            kind: "solution".into(),
            label: "linked".into(),
        }],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    let facts = graph.architecture.unwrap();
    for (project, expected, unexpected) in [
        ("A", "FirstAttribute", "SecondAttribute"),
        ("B", "SecondAttribute", "FirstAttribute"),
    ] {
        let file = facts
            .files
            .iter()
            .find(|file| {
                file.file == source.canonicalize().unwrap().to_string_lossy()
                    && file.project == project
            })
            .unwrap();
        assert!(file.annotations.iter().any(|value| value == expected));
        assert!(!file.annotations.iter().any(|value| value == unexpected));
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.project == project && node.label == "Run")
        );
    }
}
