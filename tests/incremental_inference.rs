//! Real engine subprocesses compare cached inference with fresh compiler analysis.
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

struct Fixture {
    directory: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("CodeCallNavigator incremental ")
            .tempdir()
            .unwrap();
        let fixture = Self { directory };
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        let modules = std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES")
            .map(PathBuf::from)
            .unwrap_or_else(|| repository.join("../PacificFateMapEditor/node_modules"));
        symlink(
            modules.canonicalize().unwrap(),
            fixture.root().join("node_modules"),
        )
        .unwrap();
        fixture.write("tsconfig.json", r#"{"compilerOptions":{"target":"ES2023","module":"esnext","moduleResolution":"bundler","jsx":"react-jsx","strict":true,"skipLibCheck":true},"include":["src/**/*.ts","src/**/*.tsx"]}"#);
        fixture.write("src/client.ts", CLIENT);
        fixture.write("src/observers.ts", OBSERVERS);
        fixture.write(
            "src/app/api/route.ts",
            "export function GET(){return new Response('api')}",
        );
        fixture.write(
            "src/app/about/page.tsx",
            "export default function About(){return null}",
        );
        fixture.write(
            ".codecallnavigator/layers.yml",
            "# Existing project configuration must survive\n",
        );
        fixture
    }
    fn root(&self) -> &Path {
        self.directory.path()
    }
    fn write(&self, relative: &str, text: &str) {
        let path = self.root().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn graph(&self) -> Vec<u8> {
        fs::read(self.root().join(".codecallnavigator/graph.json")).unwrap()
    }
    fn cache(&self) -> PathBuf {
        self.root().join(".codecallnavigator/inference.sqlite3")
    }
    fn analyze(&self, disabled: bool) -> Value {
        let output = run(self.root(), disabled, false);
        assert_eq!(output["event"], "complete", "{output}");
        let fingerprints: Vec<codecallnavigator_core::model::FileFingerprint> =
            serde_json::from_value(output["fingerprints"].clone()).unwrap();
        assert!(
            !codecallnavigator_core::fingerprint::is_stale(&fingerprints).unwrap(),
            "Cache/publication must not invalidate the completed input snapshot"
        );
        output
    }
    fn compare_full(&self) -> Vec<u8> {
        self.analyze(false);
        let cached = self.graph();
        let full = self.analyze(true);
        assert_eq!(counter(&full, "inference.cache_disabled"), 1);
        assert_eq!(
            self.graph(),
            cached,
            "Cached and clean full analysis must have identical canonical bytes"
        );
        cached
    }
}

fn engine() -> PathBuf {
    std::env::var_os("CODECALLNAVIGATOR_TEST_ENGINE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/codecallnavigator-engine")
        })
}
fn run(root: &Path, disabled: bool, cancel: bool) -> Value {
    let mut command = Command::new(engine());
    command
        .arg("serve-once")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if disabled {
        command.env("CODECALLNAVIGATOR_DISABLE_INFERENCE_CACHE", "1");
    } else {
        command.env_remove("CODECALLNAVIGATOR_DISABLE_INFERENCE_CACHE");
    }
    let mut child = command
        .spawn()
        .expect("Build codecallnavigator-engine before running this ignored acceptance test");
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut errors = child.stderr.take().unwrap();
    let stderr = std::thread::spawn(move || {
        let mut text = String::new();
        errors.read_to_string(&mut text).unwrap();
        text
    });
    let request = json!({"command":"analyze","version":1,"project":root,"roots":[],"infer_linked_roots":false,
        "targets":[{"path":root.join("tsconfig.json"),"language":"typescript","kind":"typescript","label":"incremental acceptance"}]});
    writeln!(input, "{request}").unwrap();
    let terminal = read_events(&mut output, &mut input, cancel);
    let status = child.wait().unwrap();
    let errors = stderr.join().unwrap();
    assert_eq!(
        status.success(),
        terminal["event"] == "complete",
        "{errors}\n{terminal}"
    );
    terminal
}
fn read_events(output: &mut impl BufRead, input: &mut impl Write, mut cancel: bool) -> Value {
    loop {
        let mut line = String::new();
        assert!(
            output.read_line(&mut line).unwrap() > 0,
            "Engine exited before a terminal event"
        );
        let event: Value = serde_json::from_str(&line).unwrap();
        if cancel && event["event"] == "progress" {
            writeln!(input, "cancel").unwrap();
            input.flush().unwrap();
            cancel = false;
        }
        if matches!(event["event"].as_str(), Some("complete" | "failed")) {
            return event;
        }
    }
}

fn targets(bytes: &[u8], caller: &str, provenance: &str) -> BTreeSet<String> {
    let graph: Value = serde_json::from_slice(bytes).unwrap();
    let nodes = graph["graph"]["nodes"].as_array().unwrap();
    let caller_ids = nodes
        .iter()
        .filter(|node| node["label"] == caller)
        .map(|node| node["id"].clone())
        .collect::<Vec<_>>();
    graph["graph"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| edge["provenance"] == provenance && caller_ids.contains(&edge["from"]))
        .map(|edge| {
            let target = nodes.iter().find(|node| node["id"] == edge["to"]).unwrap();
            format!(
                "{}:{}",
                target["file"].as_str().unwrap(),
                target["label"].as_str().unwrap()
            )
        })
        .collect()
}

#[test]
#[ignore = "requires prebuilt standalone engine and installed Node24/TypeScript6/Next16"]
fn cached_inference_matches_full_analysis_after_compiler_facts_change() {
    let fixture = Fixture::new();
    let cold = fixture.analyze(false);
    assert!(
        fixture.cache().is_file(),
        "The engine must create the derived inference cache"
    );
    let initial = fixture.graph();
    assert!(targets(&initial, "missing", "codecallnavigator.boundary.v1").is_empty());
    assert_eq!(
        targets(&initial, "notify", "codecallnavigator.observer.v1").len(),
        2
    );
    assert_cache_warmth(&fixture, &cold, &initial);
    assert_configuration_invalidation(&fixture, &initial);
    fixture.write(
        "src/app/missing/route.ts",
        "export function GET(){return new Response('new target')}",
    );
    let added = fixture.compare_full();
    assert_eq!(
        targets(&added, "missing", "codecallnavigator.boundary.v1").len(),
        1,
        "A cached negative match must be invalidated when a route is added"
    );
    fixture.write(
        "src/app/api/route.ts",
        "export function POST(){return new Response('changed verb')}",
    );
    let changed = fixture.compare_full();
    assert!(
        targets(&changed, "request", "codecallnavigator.boundary.v1").is_empty(),
        "Old inferred GET target cannot survive a handler verb change"
    );
    fixture.write(
        "src/client.ts",
        &CLIENT.replace("fetch('/api')", "fetch('/missing')"),
    );
    let rewired = fixture.compare_full();
    assert_eq!(
        targets(&rewired, "request", "codecallnavigator.boundary.v1").len(),
        1
    );
    fs::remove_file(fixture.root().join("src/app/missing/route.ts")).unwrap();
    let removed = fixture.compare_full();
    assert!(targets(&removed, "missing", "codecallnavigator.boundary.v1").is_empty());
    fixture.write(
        "src/observers.ts",
        &OBSERVERS.replace("subscribe(logUpdate);", ""),
    );
    let unregistered = fixture.compare_full();
    assert_eq!(
        targets(&unregistered, "notify", "codecallnavigator.observer.v1").len(),
        1,
        "Fresh compiler facts must remove vanished observer wiring"
    );
    fs::write(fixture.cache(), b"corrupt derived cache").unwrap();
    let recovered = fixture.analyze(false);
    assert!(counter(&recovered, "inference.cache_rejected") > 0);
    assert_eq!(
        fixture.graph(),
        unregistered,
        "Corrupt derived storage must fall back to correct full inference"
    );
    fixture.analyze(true);
    assert_eq!(fixture.graph(), unregistered);
    assert_only_graph_json(fixture.root());
}

fn assert_cache_warmth(fixture: &Fixture, cold: &Value, initial: &[u8]) {
    let warm = fixture.analyze(false);
    assert_eq!(fixture.graph(), initial);
    assert_eq!(cold["digest"], warm["digest"]);
    assert!(counter(cold, "inference.cache_misses") >= 3);
    assert!(
        counter(&warm, "inference.cache_hits") >= 3,
        "Positive and negative decisions should be reused"
    );
    assert_eq!(counter(&warm, "inference.cache_misses"), 0);
    let cleared = Command::new(engine())
        .args(["clear-cache", "--project"])
        .arg(fixture.root())
        .output()
        .unwrap();
    assert!(
        cleared.status.success(),
        "{}",
        String::from_utf8_lossy(&cleared.stderr)
    );
    assert!(!fixture.cache().exists());
    assert_eq!(
        fixture.graph(),
        initial,
        "Clearing preserves the published graph"
    );
    let rebuilt = fixture.analyze(false);
    assert_eq!(counter(&rebuilt, "inference.cache_hits"), 0);
    assert!(counter(&rebuilt, "inference.cache_misses") >= 3);
    assert_eq!(fixture.graph(), initial);
    assert_eq!(rebuilt["digest"], cold["digest"]);
    assert_only_graph_json(fixture.root());
}

fn assert_configuration_invalidation(fixture: &Fixture, initial: &[u8]) {
    fixture.write("next.config.js", "module.exports={basePath:'/deployment'};");
    let blocked = fixture.compare_full();
    assert!(
        targets(&blocked, "go", "codecallnavigator.boundary.v1").is_empty(),
        "New routing configuration must invalidate a previously cached navigation match"
    );
    fs::remove_file(fixture.root().join("next.config.js")).unwrap();
    assert_eq!(
        fixture.compare_full(),
        initial,
        "Removing the routing blocker must restore exactly the initial artifact"
    );
}

#[test]
#[ignore = "requires prebuilt standalone engine and installed Node24/TypeScript6/Next16"]
fn failed_and_cancelled_incremental_runs_preserve_the_published_graph() {
    let fixture = Fixture::new();
    fixture.analyze(false);
    let successful = fixture.graph();
    let cache = fs::read(fixture.cache()).unwrap();
    fixture.write(
        "src/broken.ts",
        "export const invalid: number = 'not a number';",
    );
    assert_eq!(run(fixture.root(), false, false)["event"], "failed");
    assert_eq!(fixture.graph(), successful);
    assert_eq!(fs::read(fixture.cache()).unwrap(), cache);
    fs::remove_file(fixture.root().join("src/broken.ts")).unwrap();
    fixture.write("src/client.ts", &CLIENT.replace("'/api'", "'/missing'"));
    assert_eq!(run(fixture.root(), false, true)["event"], "failed");
    assert_eq!(fixture.graph(), successful);
    assert_eq!(fs::read(fixture.cache()).unwrap(), cache);
    fixture.compare_full();
    assert_only_graph_json(fixture.root());
}
fn counter(complete: &Value, name: &str) -> u64 {
    complete["metrics"]["counters"][name].as_u64().unwrap_or(0)
}

#[test]
#[ignore = "requires prebuilt standalone engine and installed Node24/TypeScript6/Next16"]
fn publication_failure_does_not_commit_new_inference_decisions() {
    let fixture = Fixture::new();
    fixture.analyze(false);
    let previous_graph = fixture.graph();
    let previous_cache = fs::read(fixture.cache()).unwrap();
    fixture.write("src/client.ts", &CLIENT.replace("'/api'", "'/missing'"));
    let directory = fixture.root().join(".codecallnavigator");
    let permissions = fs::metadata(&directory).unwrap().permissions();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o555)).unwrap();
    let failed = run(fixture.root(), false, false);
    fs::set_permissions(&directory, permissions).unwrap();
    assert_eq!(
        failed["event"], "failed",
        "Publication must actually fail before this test can verify recovery"
    );
    assert!(
        failed["message"]
            .as_str()
            .unwrap()
            .contains(".graph-publish-"),
        "The failure must occur during graph publication: {failed}"
    );
    assert_eq!(fixture.graph(), previous_graph);
    assert_eq!(
        fs::read(fixture.cache()).unwrap(),
        previous_cache,
        "New decisions cannot commit before graph publication succeeds"
    );
    fixture.compare_full();
}

fn assert_only_graph_json(root: &Path) {
    let names = fs::read_dir(root.join(".codecallnavigator"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".json"))
        .collect::<Vec<_>>();
    assert_eq!(names, ["graph.json"]);
    assert_eq!(
        fs::read_to_string(root.join(".codecallnavigator/layers.yml")).unwrap(),
        "# Existing project configuration must survive\n"
    );
}
const CLIENT: &str = "import {redirect} from 'next/navigation'; export function request(){return fetch('/api')} export function missing(){return fetch('/missing')} export function go(){redirect('/about')}";
const OBSERVERS: &str = "type Listener=()=>number; const callbacks=new Set<Listener>(); export function subscribe(callback:Listener){callbacks.add(callback)} export function notify(){for(const callback of callbacks)callback()} export function redraw(){return 1} export function logUpdate(){return 2} subscribe(redraw); subscribe(logUpdate);";
