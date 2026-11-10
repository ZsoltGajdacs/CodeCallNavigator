//! Cache acceptance uses real SQLite and the same canonical graph contract as the CLI.
use codegraph::{
    artifact,
    boundary::{self, BoundaryInventory, BoundaryUse, Configuration, DecisionCache, RouteFact},
    model::{GraphData, Node, SourceSite},
};
fn site() -> SourceSite {
    SourceSite {
        file: "/repo/app.ts".into(),
        line: 1,
        column: Some(1),
        span: None,
    }
}
fn route(target: &str, path: &str) -> RouteFact {
    RouteFact {
        symbol: Some(target.into()),
        build: "/repo/tsconfig.json".into(),
        application: "next-app".into(),
        framework: "next".into(),
        kind: "http".into(),
        path: Some(path.into()),
        methods: vec!["GET".into()],
        site: site(),
        restrictions: vec![],
        order: Some(0),
    }
}
fn usage(path: &str) -> BoundaryUse {
    BoundaryUse {
        symbol: "source".into(),
        build: "/repo/tsconfig.json".into(),
        application: "next-app".into(),
        kind: "http".into(),
        path: path.into(),
        method: Some("GET".into()),
        site: site(),
        rule: "test.fetch".into(),
        flow: false,
    }
}
fn graph() -> GraphData {
    GraphData {
        nodes: ["source", "one", "two"]
            .into_iter()
            .map(|id| Node {
                id: id.into(),
                build: "/repo/tsconfig.json".into(),
                project: "app".into(),
                file: "/repo/app.ts".into(),
                signature: format!("{id}()"),
                language: "typescript".into(),
                ..Default::default()
            })
            .collect(),
        boundaries: Some(BoundaryInventory {
            version: 1,
            routes: vec![route("one", "/one"), route("two", "/two")],
            uses: vec![usage("/one"), usage("/two")],
        }),
        engine_versions: "toolchain-v1".into(),
        ..Default::default()
    }
}
fn bytes(graph: &GraphData) -> Vec<u8> {
    artifact::canonical(graph, &[("repo".into(), "/repo".into())])
        .unwrap()
        .bytes()
        .unwrap()
}
#[test]
fn cache_reads_real_sqlite_and_reuses_decisions_without_changing_canonical_graph() {
    let project = tempfile::tempdir().unwrap();
    let mut cached = graph();
    let config = Configuration::default();
    let mut first = DecisionCache::open(project.path());
    boundary::resolve_cached(&mut cached, &config, &mut first).unwrap();
    assert!(
        !project.path().join(".codegraph/inference.sqlite3").exists(),
        "No publication before explicit post-graph commit"
    );
    first.commit();
    let mut second = DecisionCache::open(project.path());
    let mut current = graph();
    boundary::resolve_cached(&mut current, &config, &mut second).unwrap();
    assert_eq!(second.stats().hits, 2);
    assert_eq!(second.stats().misses, 0);
    let mut full = graph();
    boundary::resolve(&mut full, &config).unwrap();
    assert_eq!(bytes(&current), bytes(&full));
    assert_eq!(bytes(&cached), bytes(&full));
}
#[test]
fn unrelated_sqlite_database_at_reserved_path_is_preserved() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join(".codegraph")).unwrap();
    let path = project.path().join(".codegraph/inference.sqlite3");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE boundary_decisions(key TEXT,value TEXT);INSERT INTO boundary_decisions VALUES ('user','important');").unwrap();
    drop(db);
    let before = std::fs::read(&path).unwrap();
    let mut cache = DecisionCache::open(project.path());
    let mut graph = graph();
    boundary::resolve_cached(&mut graph, &Configuration::default(), &mut cache).unwrap();
    cache.commit();
    assert_eq!(
        blake3::hash(&std::fs::read(&path).unwrap()),
        blake3::hash(&before)
    );
    assert!(!cache.warnings().is_empty());
}

fn warm(project: &std::path::Path, native: &GraphData) {
    let mut graph = native.clone();
    let mut cache = DecisionCache::open(project);
    boundary::resolve_cached(&mut graph, &Configuration::default(), &mut cache).unwrap();
    cache.commit();
    assert!(cache.warnings().is_empty(), "{:?}", cache.warnings());
}
fn checked(native: &GraphData, cache: &mut DecisionCache) -> GraphData {
    let mut cached = native.clone();
    let mut full = native.clone();
    boundary::resolve_cached(&mut cached, &Configuration::default(), cache).unwrap();
    boundary::resolve(&mut full, &Configuration::default()).unwrap();
    assert_eq!(blake3::hash(&bytes(&cached)), blake3::hash(&bytes(&full)));
    cached
}
fn db(project: &std::path::Path) -> rusqlite::Connection {
    rusqlite::Connection::open(project.join(".codegraph/inference.sqlite3")).unwrap()
}
#[test]
fn unrelated_route_changes_reuse_unchanged_requests_and_blockers_invalidate_matching_ones() {
    let project = tempfile::tempdir().unwrap();
    let native = graph();
    warm(project.path(), &native);
    let mut changed = native.clone();
    changed.boundaries.as_mut().unwrap().routes[1].site.line = 2;
    let mut cache = DecisionCache::open(project.path());
    checked(&changed, &mut cache);
    assert_eq!((cache.stats().hits, cache.stats().misses), (1, 1));
    let mut changed = native;
    changed.boundaries.as_mut().unwrap().routes[0]
        .restrictions
        .push("unknown-condition".into());
    let mut cache = DecisionCache::open(project.path());
    let result = checked(&changed, &mut cache);
    assert_eq!(result.edges.len(), 1);
    assert_eq!((cache.stats().hits, cache.stats().misses), (1, 1));
}
#[test]
fn negative_membership_addition_removal_and_missing_support_recompute() {
    let project = tempfile::tempdir().unwrap();
    let mut native = graph();
    native.boundaries.as_mut().unwrap().uses = vec![usage("/missing")];
    warm(project.path(), &native);
    native
        .boundaries
        .as_mut()
        .unwrap()
        .routes
        .push(route("one", "/missing"));
    let mut cache = DecisionCache::open(project.path());
    let result = checked(&native, &mut cache);
    assert_eq!(result.edges.len(), 1);
    assert_eq!(cache.stats().misses, 1);
    cache.commit();
    native
        .boundaries
        .as_mut()
        .unwrap()
        .routes
        .retain(|route| route.path.as_deref() != Some("/missing"));
    let mut cache = DecisionCache::open(project.path());
    let result = checked(&native, &mut cache);
    assert!(result.edges.is_empty());
    assert_eq!(cache.stats().misses, 1);
    let mut native = graph();
    warm(project.path(), &native);
    native.nodes.retain(|node| node.id != "one");
    let missing = &mut native.boundaries.as_mut().unwrap().routes[0];
    missing.symbol = None;
    missing.restrictions.push("missing-source-handler".into());
    let mut cache = DecisionCache::open(project.path());
    let result = checked(&native, &mut cache);
    assert_eq!(result.edges.len(), 1);
    assert_eq!((cache.stats().hits, cache.stats().misses), (1, 1));
}
#[test]
fn cache_hits_bind_current_indices_and_never_preserve_previous_derivations() {
    let project = tempfile::tempdir().unwrap();
    let native = graph();
    warm(project.path(), &native);
    let mut reordered = native;
    reordered.nodes.reverse();
    let mut cache = DecisionCache::open(project.path());
    let mut result = checked(&reordered, &mut cache);
    assert_eq!(cache.stats().hits, 2);
    for edge in &result.edges {
        assert_eq!(result.nodes[edge.from as usize].id, "source");
    }
    result.boundaries.as_mut().unwrap().routes.clear();
    let mut cache = DecisionCache::open(project.path());
    let mut result = checked(&result, &mut cache);
    assert!(result.edges.is_empty());
    assert_eq!(result.diagnostics.len(), 2);
    boundary::resolve(&mut result, &Configuration::default()).unwrap();
    assert_eq!(
        result.diagnostics.len(),
        2,
        "Owned unresolved diagnostics are replaced"
    );
}
#[test]
fn unknown_format_and_implementation_are_misses_and_owned_cache_can_refresh() {
    for field in ["format", "implementation"] {
        let project = tempfile::tempdir().unwrap();
        let native = graph();
        warm(project.path(), &native);
        db(project.path())
            .execute(
                "UPDATE cache_meta SET value='incompatible' WHERE name=?1",
                [field],
            )
            .unwrap();
        let mut cache = DecisionCache::open(project.path());
        checked(&native, &mut cache);
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 2);
        assert!(!cache.warnings().is_empty());
        cache.commit();
        let mut refreshed = DecisionCache::open(project.path());
        checked(&native, &mut refreshed);
        assert_eq!(refreshed.stats().hits, 2);
    }
}
#[test]
fn malformed_winner_and_unresolved_records_fall_back_and_checksums_bind_the_key() {
    for value in [
        "null",
        "{}",
        r#"{"decision":{"kind":"unresolved"},"checksum":"bad"}"#,
    ] {
        let project = tempfile::tempdir().unwrap();
        let native = graph();
        warm(project.path(), &native);
        db(project.path())
            .execute("UPDATE boundary_decisions SET value=?1", [value])
            .unwrap();
        let mut cache = DecisionCache::open(project.path());
        checked(&native, &mut cache);
        assert_eq!((cache.stats().rejected, cache.stats().misses), (2, 2));
    }
    let project = tempfile::tempdir().unwrap();
    let mut native = graph();
    native
        .boundaries
        .as_mut()
        .unwrap()
        .uses
        .push(usage("/missing"));
    warm(project.path(), &native);
    let database = db(project.path());
    let negative: String = database
        .query_row(
            "SELECT value FROM boundary_decisions WHERE value LIKE '%unresolved%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    database
        .execute(
            "UPDATE boundary_decisions SET value=?1 WHERE value NOT LIKE '%unresolved%'",
            [negative],
        )
        .unwrap();
    drop(database);
    let mut cache = DecisionCache::open(project.path());
    checked(&native, &mut cache);
    assert_eq!(
        (
            cache.stats().hits,
            cache.stats().rejected,
            cache.stats().misses
        ),
        (1, 2, 2)
    );
}
#[test]
fn corrupt_whole_file_and_failed_writes_are_nonfatal_and_preserved() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join(".codegraph")).unwrap();
    let path = project.path().join(".codegraph/inference.sqlite3");
    std::fs::write(&path, b"unverifiable cache bytes").unwrap();
    let mut cache = DecisionCache::open(project.path());
    checked(&graph(), &mut cache);
    cache.commit();
    assert!(!cache.warnings().is_empty());
    assert_eq!(std::fs::read(&path).unwrap(), b"unverifiable cache bytes");
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let mut cache = DecisionCache::open(project.path());
    checked(&graph(), &mut cache);
    cache.commit();
    assert!(path.is_dir());
    assert!(!cache.warnings().is_empty());
}
#[test]
fn committing_retains_only_current_decisions_and_skipping_commit_keeps_previous_cache() {
    let project = tempfile::tempdir().unwrap();
    let native = graph();
    warm(project.path(), &native);
    let path = project.path().join(".codegraph/inference.sqlite3");
    let before = blake3::hash(&std::fs::read(&path).unwrap());
    let mut shortened = native;
    shortened.boundaries.as_mut().unwrap().uses.truncate(1);
    let mut cache = DecisionCache::open(project.path());
    checked(&shortened, &mut cache);
    assert_eq!(
        blake3::hash(&std::fs::read(&path).unwrap()),
        before,
        "Preparing does not publish cache"
    );
    cache.commit();
    let count: i64 = db(project.path())
        .query_row("SELECT count(*) FROM boundary_decisions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
}
#[cfg(unix)]
#[test]
fn symbolic_and_hard_links_for_database_and_sidecars_are_preserved() {
    use std::os::unix::fs::symlink;
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir(project.path().join(".codegraph")).unwrap();
        let external = project.path().join("user-data");
        std::fs::write(&external, b"important").unwrap();
        let link = project
            .path()
            .join(format!(".codegraph/inference.sqlite3{suffix}"));
        symlink(&external, &link).unwrap();
        let mut cache = DecisionCache::open(project.path());
        checked(&graph(), &mut cache);
        cache.commit();
        assert_eq!(std::fs::read(&external).unwrap(), b"important");
        assert!(link.is_symlink());
        assert!(!cache.warnings().is_empty());
    }
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join(".codegraph")).unwrap();
    let external = project.path().join("user-data");
    std::fs::write(&external, b"important").unwrap();
    std::fs::hard_link(
        &external,
        project.path().join(".codegraph/inference.sqlite3"),
    )
    .unwrap();
    let mut cache = DecisionCache::open(project.path());
    checked(&graph(), &mut cache);
    cache.commit();
    assert_eq!(std::fs::read(external).unwrap(), b"important");
    assert!(!cache.warnings().is_empty());
}
#[test]
fn immutable_readonly_cache_handles_project_paths_with_uri_characters() {
    let project = tempfile::Builder::new()
        .prefix("cache #? ")
        .tempdir()
        .unwrap();
    let native = graph();
    warm(project.path(), &native);
    let mut cache = DecisionCache::open(project.path());
    checked(&native, &mut cache);
    assert_eq!(cache.stats().hits, 2);
    assert!(cache.warnings().is_empty());
}

fn flow_graph() -> GraphData {
    use codegraph::flow::{
        CallSite, FlowInventory, FunctionFlowIR, SourceSpan, Statement, StatementKind,
    };
    let mut graph = graph();
    let usage = &mut graph.boundaries.as_mut().unwrap().uses[0];
    usage.flow = true;
    usage.site.span = Some(SourceSpan { start: 1, end: 5 });
    graph.boundaries.as_mut().unwrap().uses.truncate(1);
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
                        targets: vec![],
                        diagnostic: Some("fresh native diagnostic".into()),
                    }],
                },
            }],
            ..Default::default()
        }],
    });
    graph
}
#[test]
fn reused_routing_decision_still_checks_fresh_flow_and_reconstructs_native_diagnostics() {
    let project = tempfile::tempdir().unwrap();
    let native = flow_graph();
    warm(project.path(), &native);
    let mut absent = native.clone();
    absent.flow_inventory = None;
    let mut cache = DecisionCache::open(project.path());
    let absent = checked(&absent, &mut cache);
    assert!(absent.edges.is_empty());
    assert_eq!(cache.stats().hits, 1);
    let mut cache = DecisionCache::open(project.path());
    let mut result = checked(&native, &mut cache);
    assert_eq!(result.edges.len(), 1);
    assert_eq!(cache.stats().hits, 1);
    codegraph::flow::visit_calls_mut(
        &mut result.flow_inventory.as_mut().unwrap().functions[0],
        &mut |call| {
            assert_eq!(call.targets[0].kind, "http");
            assert!(
                call.diagnostic
                    .as_deref()
                    .unwrap()
                    .starts_with("fresh native diagnostic")
            );
        },
    );
}
#[test]
fn unrelated_wal_database_is_not_opened_for_mutation_before_ownership_is_known() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir(project.path().join(".codegraph")).unwrap();
    let path = project.path().join(".codegraph/inference.sqlite3");
    let database = rusqlite::Connection::open(&path).unwrap();
    database.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE user_content(value TEXT); INSERT INTO user_content VALUES ('important');").unwrap();
    let paths = [
        path.clone(),
        path.with_file_name("inference.sqlite3-wal"),
        path.with_file_name("inference.sqlite3-shm"),
    ];
    let before: Vec<_> = paths
        .iter()
        .map(|path| blake3::hash(&std::fs::read(path).unwrap()))
        .collect();
    let mut cache = DecisionCache::open(project.path());
    checked(&graph(), &mut cache);
    cache.commit();
    let after: Vec<_> = paths
        .iter()
        .map(|path| blake3::hash(&std::fs::read(path).unwrap()))
        .collect();
    assert_eq!(before, after);
    assert!(!cache.warnings().is_empty());
    drop(database);
}
#[test]
fn locked_cache_commit_is_bounded_and_does_not_fail_graph_results() {
    let project = tempfile::tempdir().unwrap();
    let native = graph();
    warm(project.path(), &native);
    let database = db(project.path());
    database.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let mut cache = DecisionCache::open(project.path());
    checked(&native, &mut cache);
    let started = std::time::Instant::now();
    cache.commit();
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    assert!(!cache.warnings().is_empty());
    database.execute_batch("ROLLBACK").unwrap();
}
