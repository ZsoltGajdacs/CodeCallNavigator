//! Filesystem and persistent CLI ownership require a real process boundary.
use super::*;
use codecallnavigator_core::artifact;
use std::io::{BufRead, BufReader};

struct Session {
    child: std::process::Child,
    input: Option<std::process::ChildStdin>,
    output: BufReader<std::process::ChildStdout>,
    root: std::path::PathBuf,
    generation: String,
}
impl Session {
    fn new(root: &Path, manifest: &[u8]) -> Self {
        Self::start(root, manifest, None)
    }

    fn start(root: &Path, manifest: &[u8], library: Option<&Path>) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"));
        if let Some(library) = library {
            command.env("CODECALLNAVIGATOR_GRAPH_LIBRARY", library);
        }
        let mut child = command
            .arg("query-session")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        Self {
            input: child.stdin.take(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
            root: root.into(),
            generation: blake3::hash(manifest).to_hex().to_string(),
        }
    }
    fn response(&mut self, operation: Value) -> Value {
        let input = self.input.as_mut().unwrap();
        writeln!(input, "{}", json!({"version":2,"project":self.root,"expected_generation":self.generation,"query":operation})).unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
    fn query(&mut self, operation: Value) -> Value {
        let response = self.response(operation);
        assert!(response["error"].is_null(), "{response}");
        response["result"].clone()
    }
    fn resources(&mut self, action: &str) -> Value {
        self.query(json!({"kind":"flow_resources","action":action}))
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.input.take();
        let _ = self.child.wait();
    }
}

#[test]
fn session_residency_is_explicit_and_structure_stays_lazy() {
    let root = tempfile::tempdir().unwrap();
    let bytes = saved_graph(root.path());
    let mut session = Session::new(root.path(), &bytes);
    let symbol =
        session.query(json!({"kind":"search","text":"Start"}))["hits"][0]["target"]["symbol"]
            .clone();
    assert_eq!(session.resources("status")["loaded"], false);
    assert!(session.query(json!({"kind":"snapshot"}))["flow_inventory"].is_null());
    assert_eq!(session.resources("status")["loaded"], false);
    assert_eq!(
        session.query(json!({"kind":"flow","symbol":symbol}))["chart"]["symbol"],
        symbol
    );
    let loaded = session.resources("status");
    assert_eq!(loaded["loaded"], true);
    assert_eq!(loaded["functions"], 1);
    assert!(loaded["retained_bytes"].as_u64().unwrap() > 0);
    let unloaded = session.resources("unload");
    assert_eq!(unloaded["loaded"], false);
    assert_eq!(unloaded["retained_bytes"], 0);
    assert_eq!(session.resources("unload")["loaded"], false);
    assert_eq!(session.resources("load")["loaded"], true);
    assert_eq!(
        session.query(json!({"kind":"flow","symbol":symbol}))["chart"]["symbol"],
        symbol
    );
}

#[test]
fn corrupt_flows_preserve_structure_and_recover_in_the_same_session() {
    let root = tempfile::tempdir().unwrap();
    let library = root.path().join("isolated-library");
    let bundle = graph_fixture(root.path()).encode().unwrap();
    artifact::AnalysisLock::acquire_in(
        root.path(),
        artifact::local_snapshots::Store::at(library.clone()),
    )
    .unwrap()
    .publish_encoded(&bundle, &std::sync::atomic::AtomicBool::new(false))
    .unwrap();
    let manifest = artifact::storage::Manifest::from_bytes(bundle.as_ref()).unwrap();
    let path = library.join(manifest.flows.path("flows"));
    let original = fs::read(&path).unwrap();
    fs::write(&path, b"corrupt flow object").unwrap();
    let mut session = Session::start(root.path(), bundle.as_ref(), Some(&library));
    assert_eq!(session.query(json!({"kind":"status"}))["nodes"], 2);
    assert_eq!(
        session.response(json!({"kind":"flow_resources","action":"load"}))["error"]["code"],
        "flows_unavailable"
    );
    assert_eq!(session.resources("status")["loaded"], false);
    fs::write(&path, original).unwrap();
    assert_eq!(session.resources("load")["loaded"], true);
}

#[test]
fn unload_releases_owned_generation_after_a_new_publication() {
    let root = tempfile::tempdir().unwrap();
    let bytes = saved_graph(root.path());
    let mut session = Session::new(root.path(), &bytes);
    assert_eq!(session.resources("load")["loaded"], true);
    let mut next = artifact::load(root.path()).unwrap();
    next.graph["diagnostics"] = json!(["next generation"]);
    artifact::publish(
        root.path(),
        &next,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(session.resources("unload")["retained_bytes"], 0);
    let outdated = session.response(json!({"kind":"status"}));
    assert_eq!(outdated["error"]["code"], "generation_mismatch");
    session.generation =
        blake3::hash(&fs::read(artifact::working_manifest_path(root.path()).unwrap()).unwrap())
            .to_hex()
            .to_string();
    assert_eq!(session.resources("status")["loaded"], false);
}
