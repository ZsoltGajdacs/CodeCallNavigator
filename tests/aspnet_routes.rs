//! Real Microsoft.AspNetCore.App controller declarations through the pinned Roslyn adapter.
use codecallnavigator_core::boundary::RouteFact;
use codecallnavigator_engine::{
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
#[ignore = "requires pinned CallAtlas and installed .NET11 ASP.NET reference pack"]
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
            .filter(|edge| edge.provenance == "codecallnavigator.mediatr.v1")
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
#[ignore = "requires pinned CallAtlas and installed .NET11 ASP.NET reference pack"]
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
    fs::write(root.path().join("Probe.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net11.0</TargetFramework></PropertyGroup><ItemGroup><FrameworkReference Include="Microsoft.AspNetCore.App" /></ItemGroup></Project>"#).unwrap();
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
#[ignore = "requires pinned CallAtlas and installed .NET11 ASP.NET reference pack"]
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

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET11 ASP.NET reference pack"]
fn endpoint_metadata_providers_block_return_and_parameter_routes() {
    let (_root, graph) = controller_probe(RETURN_METADATA_SOURCE);
    let routes = &graph.boundaries.as_ref().unwrap().routes;
    for action in [
        "Direct",
        "TaskResult",
        "ValueTaskResult",
        "DerivedTaskResult",
        "CustomAwaitableResult",
    ] {
        let route = action_route(&graph, action);
        assert!(
            route
                .restrictions
                .iter()
                .any(|reason| reason == "aspnet.return-endpoint-metadata"),
            "Unrestricted metadata provider: {action}"
        );
        assert!(route.path.is_none() && route.methods.is_empty() && route.order.is_none());
    }
    for action in ["Parameter", "ParameterSpecific"] {
        let route = action_route(&graph, action);
        assert!(
            route
                .restrictions
                .iter()
                .any(|reason| reason == "aspnet.parameter-endpoint-metadata"),
            "Unrestricted parameter provider: {action}"
        );
        assert!(route.path.is_none() && route.methods.is_empty() && route.order.is_none());
    }
    for path in ["/api/fake", "/api/fake-task", "/api/fake-parameter"] {
        let route = routes
            .iter()
            .find(|route| route.path.as_deref() == Some(path))
            .unwrap();
        assert!(
            route.restrictions.is_empty(),
            "Unrelated interface/wrapper must not be treated as framework metadata: {path}"
        );
    }
}

fn action_route<'a>(graph: &'a GraphData, action: &str) -> &'a RouteFact {
    let symbol = &graph
        .nodes
        .iter()
        .find(|node| node.label == action)
        .unwrap()
        .id;
    graph
        .boundaries
        .as_ref()
        .unwrap()
        .routes
        .iter()
        .find(|route| route.symbol.as_ref() == Some(symbol))
        .unwrap()
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET11 ASP.NET reference pack"]
fn custom_endpoint_http_method_metadata_blocks_unproved_request_methods() {
    let (_root, graph) = controller_probe(METHOD_METADATA_SOURCE);
    let routes = &graph.boundaries.as_ref().unwrap().routes;
    let constrained = routes
        .iter()
        .find(|route| route.path.as_deref() == Some("/api/limited"))
        .unwrap();
    assert!(
        constrained
            .restrictions
            .iter()
            .any(|reason| reason == "aspnet.request-constraints")
    );
    let unrelated = routes
        .iter()
        .find(|route| route.path.as_deref() == Some("/api/fake"))
        .unwrap();
    assert!(unrelated.restrictions.is_empty());
    assert_eq!(unrelated.methods, ["GET"]);
}

const RETURN_METADATA_SOURCE: &str = r#"
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Http.Metadata;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Routing;
using System.Reflection;
using System.Threading.Tasks;
[Route("api")] public class TestController:ControllerBase {
    [HttpGet("direct")] public MetadataPayload Direct()=>new();
    [HttpGet("task")] public Task<MetadataPayload> TaskResult()=>Task.FromResult(new MetadataPayload());
    [HttpGet("value-task")] public ValueTask<MetadataPayload> ValueTaskResult()=>new(new MetadataPayload());
    [HttpGet("derived-task")] public DerivedTask DerivedTaskResult()=>new();
    [HttpGet("custom-awaitable")] public CustomAwaitable CustomAwaitableResult()=>new();
    [HttpGet("parameter")] public string Parameter([FromQuery]MetadataPayload value)=>"ok";
    [HttpGet("parameter-specific")] public string ParameterSpecific([FromQuery]ParameterPayload value)=>"ok";
    [HttpGet("fake")] public FakePayload Fake()=>new();
    [HttpGet("fake-task")] public Pretend.Task<MetadataPayload> FakeTask()=>new();
    [HttpGet("fake-parameter")] public string FakeParameter([FromQuery]FakeParameterPayload value)=>"ok";
}
public sealed class MetadataPayload:IEndpointMetadataProvider {
    public static void PopulateMetadata(MethodInfo method,EndpointBuilder builder)=>builder.Metadata.Add(new NeverMatch());
}
public sealed class NeverMatch:ISuppressMatchingMetadata {public bool SuppressMatching=>true;}
public sealed class DerivedTask:Task<MetadataPayload> {public DerivedTask():base(()=>new MetadataPayload()){} }
public sealed class CustomAwaitable {
    public System.Runtime.CompilerServices.TaskAwaiter<MetadataPayload> GetAwaiter()=>Task.FromResult(new MetadataPayload()).GetAwaiter();
}
public sealed class ParameterPayload:IEndpointParameterMetadataProvider {
    public static void PopulateMetadata(ParameterInfo parameter,EndpointBuilder builder)=>builder.Metadata.Add(new NeverMatch());
}
public sealed class FakePayload:Pretend.IEndpointMetadataProvider {}
public sealed class FakeParameterPayload:Pretend.IEndpointParameterMetadataProvider {}
namespace Pretend {public interface IEndpointMetadataProvider {} public interface IEndpointParameterMetadataProvider {} public sealed class Task<T> {}}
"#;

const METHOD_METADATA_SOURCE: &str = r#"
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Routing;
[Route("api")] public class TestController:ControllerBase {
    [Route("limited")][PostOnly] public string Limited()=>"ok";
    [HttpGet("fake")][FakeMethods] public string Fake()=>"ok";
}
public sealed class PostOnlyAttribute:System.Attribute,IHttpMethodMetadata {
    public bool AcceptCorsPreflight=>false;
    public System.Collections.Generic.IReadOnlyList<string> HttpMethods=>new[]{"POST"};
}
public sealed class FakeMethodsAttribute:System.Attribute,Pretend.IHttpMethodMetadata {}
namespace Pretend {public interface IHttpMethodMetadata {}}
"#;

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET11 ASP.NET reference pack"]
fn model_conventions_block_routes_only_in_scopes_where_mvc_applies_them() {
    let (_root, graph) = controller_probe(CONVENTION_SOURCE);
    for action in [
        "ControllerRun",
        "ActionRun",
        "ParameterRun",
        "ParameterBaseRun",
        "PropertyRun",
    ] {
        let route = action_route(&graph, action);
        assert!(route.path.is_none() && route.methods.is_empty() && route.order.is_none());
        assert!(
            route
                .restrictions
                .iter()
                .any(|reason| reason == "aspnet.model-convention")
        );
    }
    for action in ["ApplicationRun", "FakeRun", "FakeParameterRun"] {
        let route = action_route(&graph, action);
        assert!(route.path.is_some() && route.restrictions.is_empty());
    }
}

const CONVENTION_SOURCE: &str = r#"
using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.Mvc.ApplicationModels;
[Route("api/controller")][RewriteController] public class ControllerOnlyController:ControllerBase {
    [HttpGet("original")] public string ControllerRun()=>"ok";
}
[Route("api/action")] public class ActionOnlyController:ControllerBase {
    [HttpGet("original")][RewriteAction] public string ActionRun()=>"ok";
}
[Route("api/application")][RewriteApplication] public class AppOnlyController:ControllerBase {
    [HttpGet("original")] public string ApplicationRun()=>"ok";
}
[Route("api/fake")][FakeConvention] public class FakeController:ControllerBase {
    [HttpGet("original")] public string FakeRun()=>"ok";
}
[Route("api/parameter")] public class ParameterController:ControllerBase {
    [HttpGet("original")] public string ParameterRun([RewriteParameter]string value)=>"ok";
    [HttpGet("base")] public string ParameterBaseRun([RewriteParameterBase]string value)=>"ok";
    [HttpGet("fake")] public string FakeParameterRun([FakeParameterConvention]string value)=>"ok";
}
[Route("api/property")] public class PropertyController:ControllerBase {
    [RewriteProperty]public string Input{get;set;}="";
    [HttpGet("original")]public string PropertyRun()=>"ok";
}
public sealed class RewriteParameterAttribute:System.Attribute,IParameterModelConvention {
    public void Apply(ParameterModel parameter){foreach(var selector in parameter.Action.Selectors)selector.AttributeRouteModel=new(new RouteAttribute("/changed-parameter"));}
}
public sealed class RewriteParameterBaseAttribute:System.Attribute,IParameterModelBaseConvention {
    public void Apply(ParameterModelBase parameter){if(parameter is ParameterModel value)foreach(var selector in value.Action.Selectors)selector.AttributeRouteModel=new(new RouteAttribute("/changed-parameter-base"));}
}
public sealed class RewritePropertyAttribute:System.Attribute,IParameterModelBaseConvention {
    public void Apply(ParameterModelBase parameter){if(parameter is PropertyModel value)foreach(var selector in value.Controller.Selectors)selector.AttributeRouteModel=new(new RouteAttribute("changed-property"));}
}
public sealed class FakeParameterConventionAttribute:System.Attribute,Pretend.IParameterModelConvention,Pretend.IParameterModelBaseConvention {}
public sealed class RewriteControllerAttribute:System.Attribute,IControllerModelConvention {
    public void Apply(ControllerModel controller){foreach(var selector in controller.Selectors)selector.AttributeRouteModel=new(new RouteAttribute("changed-controller"));}
}
public sealed class RewriteActionAttribute:System.Attribute,IActionModelConvention {
    public void Apply(ActionModel action){foreach(var selector in action.Selectors)selector.AttributeRouteModel=new(new RouteAttribute("/changed-action"));}
}
public sealed class RewriteApplicationAttribute:System.Attribute,IApplicationModelConvention {
    public void Apply(ApplicationModel model){foreach(var controller in model.Controllers)foreach(var selector in controller.Selectors)selector.AttributeRouteModel=new(new RouteAttribute("changed-application"));}
}
public sealed class FakeConventionAttribute:System.Attribute,Pretend.IControllerModelConvention {}
namespace Pretend {public interface IControllerModelConvention {} public interface IParameterModelConvention {} public interface IParameterModelBaseConvention {}}
"#;
