//! Real public MediatR contracts and Roslyn compilations, with isolated applications.
use codecallnavigator_engine::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::target_from_file,
    model::{Edge, GraphData},
};
use std::{collections::BTreeSet, path::Path, sync::atomic::AtomicBool};

fn inferred_targets(graph: &GraphData) -> BTreeSet<(String, String)> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.provenance == "codecallnavigator.mediatr.v1")
        .map(|edge| {
            let from = &graph.nodes[edge.from as usize];
            let to = &graph.nodes[edge.to as usize];
            (
                format!("{}.{}", from.container, from.label),
                format!("{}.{}", to.container, to.label),
            )
        })
        .collect()
}

fn assert_evidence(graph: &GraphData, edge: &Edge) {
    assert_eq!(edge.kind, "call");
    assert_eq!(edge.certainty, "possible");
    assert_eq!(edge.count, 1);
    assert!(!edge.evidence.is_empty());
    for evidence in &edge.evidence {
        assert!(matches!(
            evidence.rule.as_str(),
            "codecallnavigator.mediatr:send" | "codecallnavigator.mediatr:publish"
        ));
        assert_eq!(evidence.version, 1);
        assert_eq!(evidence.semantics, "possible-invocation");
        assert!(
            evidence.registration.is_none(),
            "DI registration is not proven"
        );
        for qualifier in evidence.qualifiers.values() {
            assert!(!qualifier.contains("Version="));
            assert!(!qualifier.contains("Culture="));
            assert!(!qualifier.contains("PublicKeyToken="));
        }
        assert_eq!(
            evidence.qualifiers.get("mediatr.completion"),
            Some(&"task".to_string())
        );
        assert!(
            evidence
                .qualifiers
                .get("mediatr.operation")
                .is_some_and(|value| matches!(value.as_str(), "Send" | "Publish"))
        );
        assert!(evidence.invocation.file.ends_with("Dispatch.cs"));
        assert!(evidence.invocation.line > 0);
        assert!(
            evidence
                .supporting_symbols
                .contains(&graph.nodes[edge.to as usize].id)
        );
        if graph.nodes[edge.from as usize].label == "Conditional" {
            assert_eq!(
                evidence.qualifiers.get("mediatr.conditional_context"),
                Some(&"true".to_string())
            );
            assert_eq!(
                evidence.qualifiers.get("mediatr.direct_await"),
                Some(&"true".to_string())
            );
        }
    }
}

#[test]
#[ignore = "requires pinned CallAtlas, .NET11 and restored public MediatR fixture"]
fn real_framework_dispatch_uses_request_symbols_and_application_scope() {
    let project =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/mediatr-dispatch/MediatRDispatch.sln");
    let graph = analyze(
        vec![target_from_file(&project).unwrap()],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert_eq!(inferred_targets(&graph), expected_targets());
    let entries = &graph.entry_points.as_ref().unwrap().framework_entries;
    let handlers = graph
        .nodes
        .iter()
        .filter(|node| node.label == "Handle" || node.label.ends_with(".Handle"))
        .filter(|node| entries.contains(&node.id))
        .count();
    assert_eq!(handlers, 7);
    assert!(
        graph
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("anonymous callable has no native identity"))
    );
    assert_eq!(
        graph
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.starts_with("MediatR dispatch deferred"))
            .count(),
        6
    );
    for edge in &graph.edges {
        if edge.provenance == "codecallnavigator.mediatr.v1" {
            assert_evidence(&graph, edge);
        }
    }
    graph.validate().unwrap();
}

fn expected_targets() -> BTreeSet<(String, String)> {
    [
        (
            "DispatchAppA.Dispatch.SendPing",
            "DispatchAppA.PingHandler.Handle",
        ),
        (
            "DispatchAppA.Dispatch.NamedArguments",
            "DispatchAppA.PingHandler.Handle",
        ),
        (
            "DispatchAppA.Dispatch.Conditional",
            "DispatchAppA.PingHandler.Handle",
        ),
        (
            "DispatchAppA.Dispatch.Invoke",
            "DispatchAppA.PingHandler.Handle",
        ),
        (
            "DispatchAppA.Dispatch.SendReset",
            "DispatchAppA.ResetHandler.MediatR.IRequestHandler<DispatchContracts.Reset>.Handle",
        ),
        (
            "DispatchAppA.Dispatch.PublishChanged",
            "DispatchAppA.FirstChangedHandler.Handle",
        ),
        (
            "DispatchAppA.Dispatch.PublishChanged",
            "DispatchAppA.ChangedHandlerBase.Handle",
        ),
        (
            "DispatchAppB.Dispatch.SendPing",
            "DispatchAppB.PingHandler.Handle",
        ),
        (
            "DispatchAppB.Dispatch.PublishChanged",
            "DispatchAppB.ChangedHandler.Handle",
        ),
    ]
    .into_iter()
    .map(|(from, to)| (from.to_owned(), to.to_owned()))
    .collect()
}
