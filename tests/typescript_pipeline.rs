//! Compiler-backed acceptance. No dependency installation or project build is performed.
//! Set CODEGRAPH_TEST_TYPESCRIPT_ROOT to an installed TypeScript 6.0.x package.
use codegraph::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::{BuildTarget, target_from_file},
    fingerprint,
    model::GraphData,
    store::SqliteStore,
};
use std::{
    collections::HashSet,
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn fixture() -> tempfile::TempDir {
    let temporary = tempfile::Builder::new()
        .prefix("CodeGraph TypeScript acceptance ")
        .tempdir()
        .unwrap();
    let engine = std::env::var_os("CODEGRAPH_TEST_TYPESCRIPT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../PacificFateMapEditor/node_modules/typescript")
        })
        .canonicalize()
        .expect("Set CODEGRAPH_TEST_TYPESCRIPT_ROOT to installed TypeScript 6.0.x");
    fs::create_dir(temporary.path().join("node_modules")).unwrap();
    symlink(engine, temporary.path().join("node_modules/typescript")).unwrap();
    write(
        temporary.path(),
        "tsconfig.json",
        r#"{"compilerOptions":{"strict":true,"target":"ES2022","module":"NodeNext","moduleResolution":"NodeNext","skipLibCheck":true},"include":["src/**/*.ts"]}"#,
    );
    temporary
}

fn write(root: &Path, name: &str, contents: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn target(root: &Path, name: &str) -> BuildTarget {
    target_from_file(&root.join(name)).unwrap()
}

fn run(targets: Vec<BuildTarget>) -> anyhow::Result<GraphData> {
    analyze(
        targets,
        &AnalyzerRuntime::discover()?,
        &AtomicBool::new(false),
        &|_| {},
    )
}

fn has_edge(graph: &GraphData, from: &str, to: &str) -> bool {
    graph.edges.iter().any(|edge| {
        graph.nodes[edge.from as usize].label == from && graph.nodes[edge.to as usize].label == to
    })
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn real_compiler_keeps_callback_calls_on_the_callback_and_roundtrips_provenance() {
    let fixture = fixture();
    write(
        fixture.path(),
        "src/main.ts",
        "export function leaf() { return 1; }\nexport function start() { const work = () => leaf(); return work(); }\nexport function disconnected() { return 2; }\n",
    );
    let graph = run(vec![target(fixture.path(), "tsconfig.json")]).unwrap();
    assert!(has_edge(&graph, "start", "work"));
    assert!(has_edge(&graph, "work", "leaf"));
    assert!(!has_edge(&graph, "start", "leaf"));
    assert!(graph.nodes.iter().any(|node| node.label == "disconnected"));
    assert!(graph.edges.iter().all(|edge| !edge.provenance.is_empty()));
    assert!(graph.engine_versions.contains("6.0."));
    let facts = graph.architecture.as_ref().unwrap();
    let source = facts
        .files
        .iter()
        .find(|file| file.file.ends_with("/src/main.ts"))
        .unwrap();
    assert_eq!(source.build, graph.nodes[0].build);
    assert!(source.content.contains("export function leaf()"));

    let cache = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(cache.path()).unwrap();
    store.publish(&graph, &AtomicBool::new(false)).unwrap();
    let loaded = store.load_active().unwrap().unwrap();
    assert_eq!(loaded.edges.len(), graph.edges.len());
    assert_eq!(loaded.edges[0].provenance, graph.edges[0].provenance);
    assert_eq!(
        (loaded.entry_points, loaded.architecture),
        (graph.entry_points, graph.architecture)
    );
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn source_project_references_need_no_outputs_and_duplicate_selections_share_symbols() {
    let fixture = fixture();
    write(
        fixture.path(),
        "tsconfig.json",
        r#"{"files":[],"references":[{"path":"lib"},{"path":"app"}]}"#,
    );
    write(
        fixture.path(),
        "lib/tsconfig.json",
        r#"{"compilerOptions":{"composite":true,"strict":true,"target":"ES2022","module":"NodeNext","outDir":"../output/lib"},"include":["*.ts"]}"#,
    );
    write(
        fixture.path(),
        "lib/service.ts",
        "export function service() { return 42; }\n",
    );
    write(
        fixture.path(),
        "app/tsconfig.json",
        r#"{"compilerOptions":{"composite":true,"strict":true,"target":"ES2022","module":"NodeNext","outDir":"../output/app"},"references":[{"path":"../lib"}],"include":["*.ts"]}"#,
    );
    write(
        fixture.path(),
        "app/main.ts",
        "import { service } from '../lib/service'; export function main() { return service(); }\n",
    );
    let root = target(fixture.path(), "tsconfig.json");
    let graph = run(vec![root.clone()]).unwrap();
    assert!(has_edge(&graph, "main", "service"));
    assert!(
        !fixture.path().join("output").exists(),
        "analysis emitted files"
    );
    let combined = run(vec![root, target(fixture.path(), "lib/tsconfig.json")]).unwrap();
    assert_eq!(combined.nodes.len(), graph.nodes.len());
    assert_eq!(combined.edges.len(), graph.edges.len());
    assert_eq!(
        combined
            .nodes
            .iter()
            .map(|node| &node.id)
            .collect::<HashSet<_>>()
            .len(),
        combined.nodes.len()
    );
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn overlapping_configs_are_rejected_before_graph_publication() {
    let fixture = fixture();
    write(
        fixture.path(),
        "src/main.ts",
        "export function main() { return 1; }\n",
    );
    fs::copy(
        fixture.path().join("tsconfig.json"),
        fixture.path().join("tsconfig.other.json"),
    )
    .unwrap();
    let error = run(vec![
        target(fixture.path(), "tsconfig.json"),
        target(fixture.path(), "tsconfig.other.json"),
    ])
    .unwrap_err();
    assert!(
        format!("{error:#}").to_lowercase().contains("overlap"),
        "{error:#}"
    );
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn type_errors_and_cancellation_preserve_the_previous_generation() {
    let fixture = fixture();
    write(
        fixture.path(),
        "src/main.ts",
        "export function main(): number { return 1; }\n",
    );
    let build = target(fixture.path(), "tsconfig.json");
    let cache = tempfile::tempdir().unwrap();
    let cli = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_codegraph"))
            .arg("analyze")
            .arg(&build.path)
            .env("CODEGRAPH_DATA_DIR", cache.path())
            .output()
            .unwrap()
    };
    let success = cli();
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    let store = SqliteStore::open(cache.path()).unwrap();
    let active = store.active_generation().unwrap().unwrap();
    let graph = store.load_active().unwrap().unwrap();
    write(
        fixture.path(),
        "src/main.ts",
        "export function main(): number { return 'wrong'; }\n",
    );
    let failure = cli();
    assert!(!failure.status.success());
    assert!(String::from_utf8_lossy(&failure.stderr).contains("2322"));
    let result = analyze(
        vec![build],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(true),
        &|_| {},
    );
    assert!(result.is_err());
    assert_eq!(
        store.active_generation().unwrap().as_deref(),
        Some(active.as_str())
    );
    assert_eq!(
        store.load_active().unwrap().unwrap().nodes.len(),
        graph.nodes.len()
    );
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn consumed_dependency_types_and_new_source_files_mark_graphs_stale() {
    let fixture = fixture();
    write(
        fixture.path(),
        "node_modules/local-service/package.json",
        r#"{"name":"local-service","version":"1.0.0","types":"index.d.ts"}"#,
    );
    write(
        fixture.path(),
        "node_modules/local-service/index.d.ts",
        "export declare function external(): number;\n",
    );
    write(
        fixture.path(),
        "src/main.ts",
        "import { external } from 'local-service'; export function main() { return external(); }\n",
    );
    fs::create_dir_all(fixture.path().join("src/spare/nested")).unwrap();
    let build = target(fixture.path(), "tsconfig.json");
    let graph = run(vec![build.clone()]).unwrap();
    assert!(
        !graph
            .nodes
            .iter()
            .any(|node| node.file.contains("node_modules"))
    );
    assert!(!fingerprint::is_stale(&graph.fingerprints).unwrap());
    write(
        fixture.path(),
        "node_modules/local-service/index.d.ts",
        "export declare function external(): number;\nexport declare function added(): void;\n",
    );
    assert!(fingerprint::is_stale(&graph.fingerprints).unwrap());
    let refreshed = run(vec![build]).unwrap();
    assert!(!fingerprint::is_stale(&refreshed.fingerprints).unwrap());
    write(
        fixture.path(),
        "src/spare/nested/new.ts",
        "export function newFunction() {}\n",
    );
    assert!(fingerprint::is_stale(&refreshed.fingerprints).unwrap());
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn framework_entries_and_render_edges_survive_the_production_pipeline() {
    let fixture = fixture();
    write(
        fixture.path(),
        "node_modules/react/package.json",
        r#"{"name":"react","version":"19.0.0","types":"index.d.ts"}"#,
    );
    write(
        fixture.path(),
        "node_modules/react/index.d.ts",
        "export declare function createElement(component: unknown, props: unknown): unknown;\nexport declare function useEffect(effect: () => void | (() => void), dependencies: unknown[]): void;\n",
    );
    write(
        fixture.path(),
        "node_modules/next/package.json",
        r#"{"name":"next","version":"16.4.2"}"#,
    );
    write(
        fixture.path(),
        "src/app/page.ts",
        "import {createElement, useEffect} from 'react';\nexport function effectLeaf() {}\nexport function Child() { return 1; }\nexport default function Page() { useEffect(() => {effectLeaf();}, []); return createElement(Child, null); }\n",
    );
    let graph = run(vec![target(fixture.path(), "tsconfig.json")]).unwrap();
    let page = graph
        .nodes
        .iter()
        .find(|node| node.label == "Page")
        .unwrap();
    let entries = graph.entry_points.as_ref().unwrap();
    assert!(entries.framework_entries.contains(&page.id));
    assert!(!entries.application_starts.contains(&page.id));
    let render = graph
        .edges
        .iter()
        .find(|edge| edge.kind == "react-render")
        .unwrap();
    assert_eq!(graph.nodes[render.from as usize].label, "Page");
    assert_eq!(graph.nodes[render.to as usize].label, "Child");
    assert!(render.is_uncertain());
    assert!(render.provenance.contains("react"));
    assert!(!has_edge(&graph, "Page", "effectLeaf"));
    let cache = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(cache.path()).unwrap();
    store.publish(&graph, &AtomicBool::new(false)).unwrap();
    let loaded = store.load_active().unwrap().unwrap();
    assert_eq!(loaded.entry_points, graph.entry_points);
    assert_eq!(
        loaded
            .edges
            .iter()
            .find(|edge| edge.kind == "react-render")
            .unwrap()
            .provenance,
        render.provenance
    );
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn packaged_resource_layout_runs_without_development_adapter_fallback() {
    let fixture = fixture();
    write(
        fixture.path(),
        "src/main.ts",
        "export function main() { return 1; }\n",
    );
    let bundle = tempfile::tempdir().unwrap();
    let resources = bundle.path().join("CodeGraph.app/Contents/Resources");
    let destination = resources.join("analyzers/typescript");
    fs::create_dir_all(&destination).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/typescript");
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
        }
    }
    let mut runtime = AnalyzerRuntime::discover().unwrap();
    runtime.bundle_root = resources;
    let graph = analyze(
        vec![target(fixture.path(), "tsconfig.json")],
        &runtime,
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert!(graph.nodes.iter().any(|node| node.label == "main"));
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn source_method_policy_applies_before_storage_search_and_traversal() {
    let fixture = fixture();
    write(
        fixture.path(),
        "src/main.ts",
        r#"
function leaf() {}
export function toString() { leaf(); }
export class Base {
  constructor() { leaf(); }
  toString() { return ''; }
  valueOf() { return this; }
  hasOwnProperty(key: PropertyKey) { return false; }
}
export class Other {
  toString(value: string) { return value; }
  static toString() { return 'static'; }
  private hidden() { leaf(); }
  protected guarded() { leaf(); }
  shown() { leaf(); }
  get value() { leaf(); return 1; }
}
export class Implicit {
  instance = leaf();
  static field = leaf();
  static { leaf(); }
  callback = () => leaf();
}
export function start() { toString(); new Base(); new Implicit(); new Other().toString('x'); }
"#,
    );
    let graph = run(vec![target(fixture.path(), "tsconfig.json")]).unwrap();
    graph.validate().unwrap();
    assert!(graph.nodes.iter().all(|node| !matches!(
        node.kind.as_str(),
        "constructor" | "implicit-constructor" | "initializer"
    )));
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.label == "toString")
            .count(),
        3
    );
    assert!(
        !graph
            .nodes
            .iter()
            .any(|node| matches!(node.label.as_str(), "valueOf" | "hasOwnProperty"))
    );
    for label in ["hidden", "guarded", "shown", "value", "callback"] {
        assert!(has_edge(&graph, label, "leaf"), "lost authored {label}");
    }
    assert!(!has_edge(&graph, "start", "leaf"));
    assert!(
        !graph
            .edges
            .iter()
            .any(|edge| graph.nodes[edge.from as usize].kind == "module"
                && graph.nodes[edge.to as usize].label == "leaf")
    );
    let cache = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(cache.path()).unwrap();
    store.publish(&graph, &AtomicBool::new(false)).unwrap();
    let loaded = store.load_active().unwrap().unwrap();
    assert_eq!(loaded.nodes.len(), graph.nodes.len());
    assert_eq!(loaded.entry_points, graph.entry_points);
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn multiple_selected_configs_bound_live_compiler_workers_and_preserve_graphs() {
    use std::{sync::atomic::Ordering, thread, time::Duration};
    let fixture = fixture();
    write(
        fixture.path(),
        "src/main.ts",
        "export function leafOne() {} export function startOne() { leafOne(); }",
    );
    write(
        fixture.path(),
        "other/tsconfig.json",
        r#"{"compilerOptions":{"strict":true,"target":"ES2022","module":"NodeNext","moduleResolution":"NodeNext","skipLibCheck":true},"include":["*.ts"]}"#,
    );
    write(
        fixture.path(),
        "other/main.ts",
        "export function leafTwo() {} export function startTwo() { leafTwo(); }",
    );
    let scope = fixture
        .path()
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let done = std::sync::Arc::new(AtomicBool::new(false));
    let finished = done.clone();
    let sampler_scope = scope.clone();
    let sampler = thread::spawn(move || {
        let mut maximum = 0;
        while !finished.load(Ordering::Acquire) {
            maximum = maximum.max(live_typescript_workers(&sampler_scope));
            thread::sleep(Duration::from_millis(10));
        }
        maximum
    });
    let result = run(vec![
        target(fixture.path(), "tsconfig.json"),
        target(fixture.path(), "other/tsconfig.json"),
    ]);
    done.store(true, Ordering::Release);
    let maximum = sampler.join().unwrap();
    let graph = result.unwrap();
    assert!(has_edge(&graph, "startOne", "leafOne"));
    assert!(has_edge(&graph, "startTwo", "leafTwo"));
    assert!(maximum > 0, "Sampler never observed a real compiler worker");
    assert!(
        maximum <= 1,
        "Observed {maximum} live TypeScript compiler workers"
    );
    assert_eq!(
        live_typescript_workers(&scope),
        0,
        "Compiler worker leaked after completion"
    );
}

fn live_typescript_workers(scope: &str) -> usize {
    let processes = std::process::Command::new("/bin/ps")
        .args(["-axo", "command="])
        .output()
        .unwrap();
    assert!(
        processes.status.success(),
        "Cannot inspect worker lifecycle"
    );
    String::from_utf8(processes.stdout)
        .unwrap()
        .lines()
        .filter(|line| line.contains("analyze.js worker ") && line.contains(scope))
        .count()
}
