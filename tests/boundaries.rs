use codecallnavigator::model::GraphData;

#[test]
fn unknown_boundary_flow_kind_is_rejected_and_legacy_targets_default_to_calls() {
    let value = serde_json::json!({"symbol":"source","certainty":"possible","provenance":"test"});
    let target: codecallnavigator::flow::CallTarget = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(target.kind, "call");
    assert_eq!(serde_json::to_value(target).unwrap(), value);
    let mut invalid = value;
    invalid["kind"] = "unknown".into();
    assert!(serde_json::from_value::<codecallnavigator::flow::CallTarget>(invalid).is_err());
}

#[test]
fn boundary_inventory_is_preserved_in_native_graphs() {
    let mut value = serde_json::to_value(GraphData::default()).unwrap();
    value["boundaries"] = serde_json::json!({"version":1,"routes":[],"uses":[]});
    let graph: GraphData = serde_json::from_value(value).unwrap();
    assert_eq!(
        serde_json::to_value(graph).unwrap()["boundaries"]["version"],
        1
    );
}

use codecallnavigator::{
    boundary::{self, BoundaryInventory, BoundaryUse, Configuration, HttpScope, RouteFact, Server},
    model::{Node, SourceSite},
};
fn graph() -> GraphData {
    GraphData {
        nodes: ["source", "static", "dynamic", "other"]
            .into_iter()
            .map(|id| Node {
                id: id.into(),
                build: "/repo/web/tsconfig.json".into(),
                project: "web".into(),
                file: "/repo/app.ts".into(),
                signature: format!("{id}()"),
                language: "typescript".into(),
                ..Default::default()
            })
            .collect(),
        boundaries: Some(BoundaryInventory {
            version: 1,
            routes: Vec::new(),
            uses: vec![BoundaryUse {
                symbol: "source".into(),
                build: "/repo/web/tsconfig.json".into(),
                application: "web".into(),
                kind: "http".into(),
                path: "/items/new".into(),
                method: Some("GET".into()),
                site: site(),
                rule: "test.fetch".into(),
                flow: false,
            }],
        }),
        ..Default::default()
    }
}
fn site() -> SourceSite {
    SourceSite {
        file: "/repo/app.ts".into(),
        line: 1,
        column: None,
        span: None,
    }
}
fn route(symbol: &str, path: &str) -> RouteFact {
    RouteFact {
        symbol: Some(symbol.into()),
        build: "/repo/web/tsconfig.json".into(),
        application: "web".into(),
        framework: "next".into(),
        kind: "http".into(),
        path: Some(path.into()),
        methods: vec!["GET".into()],
        site: site(),
        restrictions: vec![],
        order: Some(0),
    }
}
fn infer(routes: Vec<RouteFact>) -> GraphData {
    let mut graph = graph();
    graph.boundaries.as_mut().unwrap().routes = routes;
    boundary::resolve(&mut graph, &Configuration::default()).unwrap();
    graph
}
#[test]
fn static_route_wins_default_single_parameter_but_restrictions_block_uniqueness() {
    let plain = route("static", "/items/new");
    let dynamic = route("dynamic", "/items/{id}");
    let graph = infer(vec![plain.clone(), dynamic.clone()]);
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].to, 1);
    let mut constrained = plain.clone();
    constrained.symbol = Some("other".into());
    constrained.restrictions.push("params=enabled".into());
    assert!(infer(vec![plain, constrained, dynamic]).edges.is_empty());
}
#[test]
fn unsupported_route_shapes_and_unknown_order_stay_blockers() {
    for mut blocker in [
        route("other", "/items/{id:int}"),
        route("other", "/items/{*path}"),
        route("other", "/items/new"),
    ] {
        blocker.restrictions.push("unsupported".into());
        assert!(
            infer(vec![route("static", "/items/new"), blocker])
                .edges
                .is_empty()
        );
    }
    let mut blocker = route("other", "/items/{id}");
    blocker.order = None;
    assert!(
        infer(vec![route("static", "/items/new"), blocker])
            .edges
            .is_empty()
    );
}
#[test]
fn explicit_cross_language_scope_never_ranks_between_applications() {
    let mut graph = graph();
    let mut first = route("static", "/items/new");
    first.build = "/repo/api".into();
    first.framework = "spring".into();
    first.application = "api-a".into();
    let mut second = first.clone();
    second.symbol = Some("other".into());
    second.application = "api-b".into();
    graph.boundaries.as_mut().unwrap().routes = vec![first.clone(), second];
    let config = Configuration {
        version: 1,
        http: vec![HttpScope {
            client_application: None,
            client: "/repo/web/tsconfig.json".into(),
            servers: vec![Server::Target("/repo/api".into())],
        }],
    };
    assert!(boundary::resolve(&mut graph, &config).is_err());
    let mut explicit = config.clone();
    explicit.http[0].servers = vec![Server::Application {
        target: "/repo/api".into(),
        application: "api-a".into(),
    }];
    boundary::resolve(&mut graph, &explicit).unwrap();
    assert_eq!(graph.edges.len(), 1);
    graph.boundaries.as_mut().unwrap().routes = vec![first];
    boundary::resolve(&mut graph, &config).unwrap();
    assert_eq!(graph.edges.len(), 1);
}
#[test]
fn aspnet_case_variants_are_competitors_and_unmapped_servers_are_not_guessed() {
    let mut first = route("static", "/items/new");
    first.framework = "aspnet".into();
    first.build = "/repo/api.csproj".into();
    let mut second = first.clone();
    second.path = Some("/ITEMS/NEW".into());
    second.symbol = Some("other".into());
    assert!(infer(vec![first.clone()]).edges.is_empty());
    let mut graph = graph();
    graph.boundaries.as_mut().unwrap().routes = vec![first, second];
    let config = Configuration {
        version: 1,
        http: vec![HttpScope {
            client_application: None,
            client: "/repo/web/tsconfig.json".into(),
            servers: vec![Server::Target("/repo/api.csproj".into())],
        }],
    };
    boundary::resolve(&mut graph, &config).unwrap();
    assert!(graph.edges.is_empty());
}
#[test]
fn facts_roundtrip_with_optional_blockers_and_portable_paths_and_ids() {
    let mut graph = infer(vec![route("static", "/items/new")]);
    let mut blocker = route("other", "/elsewhere");
    blocker.symbol = None;
    graph.boundaries.as_mut().unwrap().routes.push(blocker);
    let roots = vec![("repo".into(), "/repo".into())];
    let artifact = codecallnavigator::artifact::canonical(&graph, &roots).unwrap();
    let local = artifact
        .hydrate(&[("repo".into(), "/relocated".into())])
        .unwrap();
    assert!(
        local.boundaries.as_ref().unwrap().uses[0]
            .build
            .starts_with("/relocated/")
    );
    assert_eq!(
        artifact.bytes().unwrap(),
        codecallnavigator::artifact::canonical(&local, &[("repo".into(), "/relocated".into())])
            .unwrap()
            .bytes()
            .unwrap()
    );
    let directory = tempfile::tempdir().unwrap();
    let store = codecallnavigator::store::SqliteStore::open(directory.path()).unwrap();
    store
        .publish(&graph, &std::sync::atomic::AtomicBool::new(false))
        .unwrap();
    let restored = store.load_active().unwrap().unwrap();
    assert_eq!(graph.boundaries, restored.boundaries);
    assert_eq!(
        artifact.bytes().unwrap(),
        codecallnavigator::artifact::canonical(&restored, &roots)
            .unwrap()
            .bytes()
            .unwrap()
    );
}
#[test]
fn configuration_normalizes_maven_selectors_and_tracks_absence_and_edits() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path();
    let targets = Vec::new();
    let (_, missing) = Configuration::load(project, &targets).unwrap();
    assert!(missing[0].missing);
    std::fs::create_dir(project.join(".codecallnavigator")).unwrap();
    std::fs::create_dir(project.join("api")).unwrap();
    std::fs::write(project.join("api/pom.xml"), "<project/>").unwrap();
    std::fs::write(project.join("tsconfig.json"), "{}").unwrap();
    std::fs::write(
        project.join(".codecallnavigator/boundaries.yml"),
        "version: 1\nhttp:\n  - client: tsconfig.json\n    servers: [api/pom.xml]\n",
    )
    .unwrap();
    assert!(codecallnavigator::fingerprint::is_stale(&missing).unwrap());
    assert!(Configuration::load(project, &targets).is_err());
    let targets = vec![
        codecallnavigator::discovery::target_from_file(&project.join("tsconfig.json")).unwrap(),
        codecallnavigator::discovery::target_from_file(&project.join("api/pom.xml")).unwrap(),
    ];
    let (config, observed) = Configuration::load(project, &targets).unwrap();
    assert_eq!(
        config.http[0].servers[0].target(),
        project
            .join("api")
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .as_ref()
    );
    std::fs::write(project.join(".codecallnavigator/boundaries.yml"), "version: 2\n").unwrap();
    assert!(codecallnavigator::fingerprint::is_stale(&observed).unwrap());
}
#[test]
fn recompilation_uses_facts_instead_of_previous_inferred_relationships() {
    let mut graph = infer(vec![route("static", "/items/new")]);
    assert_eq!(graph.edges.len(), 1);
    graph.boundaries.as_mut().unwrap().routes.clear();
    boundary::resolve(&mut graph, &Configuration::default()).unwrap();
    assert!(graph.edges.is_empty());
}
#[test]
fn legacy_flow_target_kind_stays_implicit_and_explicit_boundaries_survive() {
    let target: codecallnavigator::flow::CallTarget = serde_json::from_value(
        serde_json::json!({"symbol":"source","certainty":"direct","provenance":"compiler"}),
    )
    .unwrap();
    assert_eq!(target.kind, "call");
    assert!(serde_json::to_value(&target).unwrap().get("kind").is_none());
    let mut target = target;
    target.kind = "http".into();
    assert_eq!(serde_json::to_value(&target).unwrap()["kind"], "http");
}

#[test]
fn unknown_request_method_is_unresolved_and_flow_requires_an_actual_matching_call() {
    let mut graph = graph();
    graph.boundaries.as_mut().unwrap().routes = vec![route("static", "/items/new")];
    graph.boundaries.as_mut().unwrap().uses[0].method = None;
    boundary::resolve(&mut graph, &Configuration::default()).unwrap();
    assert!(graph.edges.is_empty());
    let usage = &mut graph.boundaries.as_mut().unwrap().uses[0];
    usage.method = Some("GET".into());
    usage.flow = true;
    usage.site.span = Some(codecallnavigator::flow::SourceSpan { start: 1, end: 5 });
    boundary::resolve(&mut graph, &Configuration::default()).unwrap();
    assert!(graph.edges.is_empty());
    use codecallnavigator::flow::{
        CallSite, CallTarget, FlowInventory, FunctionFlowIR, SourceSpan, Statement, StatementKind,
    };
    graph.flow_inventory = Some(FlowInventory {
        version: 1,
        functions: vec![FunctionFlowIR {
            symbol: "source".into(),
            file: "/repo/app.ts".into(),
            body: vec![Statement {
                span: SourceSpan { start: 1, end: 5 },
                kind: StatementKind::Action {
                    text: "fetch".into(),
                    calls: vec![CallSite {
                        span: SourceSpan { start: 1, end: 5 },
                        label: "fetch".into(),
                        targets: vec![CallTarget {
                            symbol: "dynamic".into(),
                            certainty: "direct".into(),
                            provenance: "compiler".into(),
                            ..Default::default()
                        }],
                        diagnostic: Some("native diagnostic".into()),
                    }],
                },
            }],
            ..Default::default()
        }],
    });
    boundary::resolve(&mut graph, &Configuration::default()).unwrap();
    assert_eq!(graph.edges.len(), 1);
    let function = &mut graph.flow_inventory.as_mut().unwrap().functions[0];
    codecallnavigator::flow::visit_calls_mut(function, &mut |call| {
        assert_eq!(call.targets.len(), 2);
        assert_eq!(call.targets[1].kind, "http");
        assert!(
            call.diagnostic
                .as_ref()
                .unwrap()
                .starts_with("native diagnostic")
        );
    });
    graph.boundaries.as_mut().unwrap().routes.clear();
    boundary::resolve(&mut graph, &Configuration::default()).unwrap();
    codecallnavigator::flow::visit_calls_mut(
        &mut graph.flow_inventory.as_mut().unwrap().functions[0],
        &mut |call| {
            assert_eq!(call.targets.len(), 1);
            assert_eq!(call.targets[0].provenance, "compiler");
            assert_eq!(call.diagnostic.as_deref(), Some("native diagnostic"));
        },
    );
}
#[test]
fn invalid_versions_and_overlapping_client_scopes_are_rejected_independent_of_facts() {
    let mut graph = GraphData::default();
    assert!(
        boundary::resolve(
            &mut graph,
            &Configuration {
                version: 2,
                http: vec![]
            }
        )
        .is_err()
    );
    let generic = HttpScope {
        client: "/repo/web/tsconfig.json".into(),
        client_application: None,
        servers: vec![Server::Target("/repo/api".into())],
    };
    let mut specific = generic.clone();
    specific.client_application = Some("next-app".into());
    let config = Configuration {
        version: 1,
        http: vec![generic, specific],
    };
    assert!(boundary::resolve(&mut graph, &config).is_err());
}
