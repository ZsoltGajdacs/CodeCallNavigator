//! Actual Roslyn symbols distinguish constructed owners and array element assemblies.
use codecallnavigator_engine::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::target_from_file,
};
use std::{collections::BTreeSet, fs, path::Path, process::Command, sync::atomic::AtomicBool};

fn write(root: &Path, name: &str, content: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn fixture(root: &Path) {
    write(
        root,
        "Directory.Build.props",
        include_str!("../fixtures/mediatr-dispatch/Directory.Build.props"),
    );
    write(
        root,
        "NuGet.config",
        include_str!("../fixtures/mediatr-dispatch/NuGet.config"),
    );
    for project in ["A", "B"] {
        write(
            root,
            &format!("{project}/{project}.csproj"),
            "<Project Sdk=\"Microsoft.NET.Sdk\" />",
        );
        write(
            root,
            &format!("{project}/Item.cs"),
            "namespace Collision; public sealed class Item {}",
        );
    }
    write(
        root,
        "App/App.csproj",
        r#"<Project Sdk="Microsoft.NET.Sdk"><ItemGroup>
        <ProjectReference Include="../A/A.csproj" Aliases="First" />
        <ProjectReference Include="../B/B.csproj" Aliases="Second" />
        </ItemGroup></Project>"#,
    );
    write(root, "App/Dispatch.cs", SOURCE);
    let solution = include_str!("../fixtures/mediatr-dispatch/MediatRDispatch.sln")
        .replace("Contracts", "A")
        .replace("AppA", "B")
        .replace("AppB", "App");
    write(root, "Fixture.sln", &solution);
}

#[test]
#[ignore = "requires pinned CallAtlas, .NET11 and cached public MediatR12.4.1"]
fn closed_compiler_types_and_direct_await_have_distinct_evidence() {
    let root = tempfile::Builder::new()
        .prefix("CodeCallNavigator MediatR types ")
        .tempdir()
        .unwrap();
    fixture(root.path());
    let graph = analyze(
        vec![target_from_file(&root.path().join("Fixture.sln")).unwrap()],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    let inferred: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.provenance == "codecallnavigator.mediatr.v1")
        .collect();
    let callers: BTreeSet<_> = inferred
        .iter()
        .map(|edge| graph.nodes[edge.from as usize].label.as_str())
        .collect();
    assert_eq!(
        callers,
        BTreeSet::from([
            "SendNestedCorrect",
            "SendArrayCorrect",
            "WrongAwait",
            "DirectParenthesizedAwait"
        ])
    );
    for edge in inferred {
        let caller = &graph.nodes[edge.from as usize].label;
        for evidence in &edge.evidence {
            let expected = if caller == "DirectParenthesizedAwait" {
                "true"
            } else {
                "false"
            };
            assert_eq!(
                evidence
                    .qualifiers
                    .get("mediatr.direct_await")
                    .map(String::as_str),
                Some(expected)
            );
        }
    }
    let erased = graph
        .diagnostics
        .iter()
        .filter(|message| message.contains("concrete closed payload type is unknown"))
        .count();
    assert_eq!(
        erased, 3,
        "open owners, T[] and unsupported dynamic construction must defer"
    );
}

#[test]
#[ignore = "requires pinned CallAtlas, .NET11 and cached public MediatR12.4.1"]
fn unsupported_handler_reports_compiler_error_instead_of_internal_exception() {
    let root = tempfile::tempdir().unwrap();
    fixture(root.path());
    let source = format!("{SOURCE}\n{INVALID_HANDLER}");
    write(root.path(), "App/Dispatch.cs", &source);
    let solution = root.path().join("Fixture.sln");
    let restore = Command::new("dotnet")
        .arg("restore")
        .arg(&solution)
        .arg("--configfile")
        .arg(root.path().join("NuGet.config"))
        .output()
        .unwrap();
    assert!(
        restore.status.success(),
        "fixture restore failed ({}):\n{}\n{}",
        restore.status,
        String::from_utf8_lossy(&restore.stdout),
        String::from_utf8_lossy(&restore.stderr)
    );
    let runtime = AnalyzerRuntime::discover().unwrap();
    let analyzer = [
        "analyzers/callatlas/callatlas.dll",
        "vendor/callatlas/src/CallAtlas.Cli/bin/Release/net11.0/callatlas.dll",
    ]
    .map(|path| runtime.bundle_root.join(path))
    .into_iter()
    .find(|path| path.is_file())
    .unwrap();
    let output = root.path().join("compiler-result.json");
    let result = Command::new("dotnet")
        .arg(analyzer)
        .arg("map")
        .arg(solution)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(
        result.status.code(),
        Some(1),
        "degraded compilation must fail"
    );
    let map: serde_json::Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    assert!(
        map["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["id"] == "CS1966")
    );
    assert_eq!(map["frameworkFactsVersion"], "1");
}

const SOURCE: &str = r#"
extern alias First;
extern alias Second;
using MediatR;
public class Outer<T> { public sealed class Request : IRequest<int> {} }
public sealed class Request<T> : IRequest<int> {}
public sealed class EventBox<T> : INotification {}
public sealed class ObjectHandler : INotificationHandler<EventBox<object>> {
    public Task Handle(EventBox<object> notification, CancellationToken token) => Task.CompletedTask;
}
public sealed class IntHandler : IRequestHandler<Outer<int>.Request,int> {
    public Task<int> Handle(Outer<int>.Request request, CancellationToken token) => Task.FromResult(1);
}
public sealed class ArrayHandler : IRequestHandler<Request<First::Collision.Item[]>,int> {
    public Task<int> Handle(Request<First::Collision.Item[]> request, CancellationToken token) => Task.FromResult(2);
}
public static class Dispatch {
    public static Task<int> SendNestedCorrect(ISender sender) => sender.Send(new Outer<int>.Request());
    public static Task<int> SendNestedWrong(ISender sender) => sender.Send(new Outer<string>.Request());
    public static Task<int> SendArrayCorrect(ISender sender) => sender.Send(new Request<First::Collision.Item[]>());
    public static Task<int> SendArrayWrong(ISender sender) => sender.Send(new Request<Second::Collision.Item[]>());
    public static Task<int> OpenOwner<T>(ISender sender) => sender.Send(new Outer<T[]>.Request());
    public static Task<int> OpenArray<T>(ISender sender) => sender.Send(new Request<T[]>());
    public static Task PublishDynamic(IPublisher publisher) => publisher.Publish(new EventBox<dynamic>());
    public static async Task WrongAwait(ISender sender) { await Ignore(sender.Send(new Outer<int>.Request())); }
    public static async Task<int> DirectParenthesizedAwait(ISender sender) { return await (sender.Send(new Outer<int>.Request())); }
    private static Task Ignore(Task<int> task) => Task.CompletedTask;
}
"#;

const INVALID_HANDLER: &str = r#"
public sealed class DynamicHandler : INotificationHandler<EventBox<dynamic>> {
    public Task Handle(EventBox<dynamic> notification, CancellationToken token) => Task.CompletedTask;
}
"#;
