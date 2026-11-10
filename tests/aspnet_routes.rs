//! Real Microsoft.AspNetCore.App controller declarations through the pinned Roslyn adapter.
use codegraph_core::boundary::RouteFact;
use codegraph_engine::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::target_from_file,
    model::GraphData,
};
use std::fs;
use std::{path::Path, sync::atomic::AtomicBool};

fn route<'a>(graph: &'a GraphData, application: &str, path: &str, action: &str) -> &'a RouteFact {
    graph
        .boundaries
        .as_ref()
        .unwrap()
        .routes
        .iter()
        .find(|route| {
            route.application == application
                && route.path.as_deref() == Some(path)
                && route.symbol.as_ref().is_some_and(|symbol| {
                    graph
                        .nodes
                        .iter()
                        .any(|node| node.id == *symbol && node.label == action)
                })
        })
        .unwrap_or_else(|| panic!("Missing {application} {path} {action}"))
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET10 ASP.NET reference pack"]
fn real_attribute_routes_keep_scopes_and_unsupported_competitors() {
    let target =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/mediatr-dispatch/MediatRDispatch.sln");
    let graph = analyze(
        vec![target_from_file(&target).unwrap()],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    let routes: Vec<_> = graph
        .boundaries
        .as_ref()
        .map(|facts| {
            facts
                .routes
                .iter()
                .filter(|route| route.framework == "aspnet")
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(routes.len(), 18);
    let entries = &graph.entry_points.as_ref().unwrap().framework_entries;
    assert!(
        routes
            .iter()
            .filter_map(|route| route.symbol.as_ref())
            .all(|id| entries.contains(id))
    );
    assert_supported_routes(&graph);
    assert_blockers(&graph);
    assert_exclusions(&routes);
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(|edge| edge.provenance == "codegraph.mediatr.v1")
            .count(),
        9
    );
    graph.validate().unwrap();
}

fn assert_supported_routes(graph: &GraphData) {
    assert_eq!(route(graph, "AppA", "/api/items", "List").methods, ["GET"]);
    assert_eq!(route(graph, "AppB", "/api/items", "List").methods, ["GET"]);
    assert_eq!(
        route(graph, "AppA", "/api/items", "Create").methods,
        ["POST"]
    );
    assert!(
        route(graph, "AppA", "/api/items/{id}", "Get")
            .restrictions
            .is_empty()
    );
    for (path, action) in [
        ("/health", "Health"),
        ("/ready", "Ready"),
        ("/api/items/many-one", "MultipleTemplates"),
        ("/api/items/many-two", "MultipleTemplates"),
    ] {
        assert!(route(graph, "AppA", path, action).restrictions.is_empty());
    }
}

fn assert_blockers(graph: &GraphData) {
    assert!(
        route(
            graph,
            "AppA",
            "/api/items/constraint/{id:int}",
            "Constrained"
        )
        .restrictions
        .iter()
        .any(|value| value == "aspnet.unsupported-template")
    );
    assert!(
        route(
            graph,
            "AppA",
            "/api/items/constraint/42",
            "ConstraintLiteral"
        )
        .restrictions
        .is_empty()
    );
    assert_eq!(
        route(graph, "AppA", "/api/items/ordered", "Ordered").order,
        Some(1)
    );
    assert_eq!(
        route(graph, "AppA", "/api/items/ordered", "DefaultOrder").order,
        Some(0)
    );
    assert_eq!(
        route(graph, "AppA", "/api/prioritized/value", "Value").order,
        Some(2)
    );
    assert!(
        !route(graph, "AppA", "/api/items/unsupported/{*rest}", "CatchAll")
            .restrictions
            .is_empty()
    );
}

fn assert_exclusions(routes: &[&RouteFact]) {
    assert!(
        routes
            .iter()
            .all(|route| route.kind == "http" && route.site.line > 0)
    );
    assert!(
        routes
            .iter()
            .all(|route| !route
                .path
                .as_deref()
                .is_some_and(|path| path.contains("pretend")
                    || path.contains("excluded")
                    || path.ends_with("hidden")
                    || path.ends_with("generic")
                    || path.ends_with("private")
                    || path.ends_with("static")))
    );
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET10 ASP.NET reference pack"]
fn silent_http_provider_does_not_invent_an_extra_class_prefix_route() {
    let (_root, graph) = controller_probe(
        r#"using Microsoft.AspNetCore.Mvc;
        [Route("api")] public class TestController:ControllerBase {
            [HttpGet][Route("only")] public string Run()=>"ok";
        }"#,
    );
    let routes = &graph.boundaries.as_ref().unwrap().routes;
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path.as_deref(), Some("/api/only"));
    assert_eq!(routes[0].methods, ["GET"]);
}

fn controller_probe(source: &str) -> (tempfile::TempDir, GraphData) {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Probe.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup><ItemGroup><FrameworkReference Include="Microsoft.AspNetCore.App" /></ItemGroup></Project>"#).unwrap();
    fs::write(
        root.path().join("NuGet.config"),
        include_str!("../fixtures/mediatr-dispatch/NuGet.config"),
    )
    .unwrap();
    fs::write(root.path().join("Controller.cs"), source).unwrap();
    let graph = analyze(
        vec![target_from_file(&root.path().join("Probe.csproj")).unwrap()],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    (root, graph)
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET10 ASP.NET reference pack"]
fn custom_matching_suppression_is_retained_as_an_unsupported_competitor() {
    let (_root, graph) = controller_probe(
        r#"using Microsoft.AspNetCore.Mvc;
        [Route("api")] public class TestController:ControllerBase {
            [HttpGet("disabled")][DisabledRoute] public string Run()=>"ok";
        }
        public sealed class DisabledRouteAttribute:System.Attribute, Microsoft.AspNetCore.Routing.ISuppressMatchingMetadata {
            public bool SuppressMatching=>true;
        }"#,
    );
    let route = &graph.boundaries.as_ref().unwrap().routes[0];
    assert_eq!(route.path.as_deref(), Some("/api/disabled"));
    assert!(
        route
            .restrictions
            .iter()
            .any(|value| value == "aspnet.request-constraints")
    );
}
