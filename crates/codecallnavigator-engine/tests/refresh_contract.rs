//! Real publication locks and negotiated one-shot failures; no analyzer doubles.
use codecallnavigator_core::{
    artifact::{self, AnalysisLock},
    model::GraphData,
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::atomic::AtomicBool,
};

fn exchange(request: Value) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("serve-once")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    writeln!(input, "{request}").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect()
}

fn request(root: &Path) -> Value {
    json!({"command":"analyze","version":1,"project":root,"roots":[],"targets":[]})
}

fn publish(root: &Path, note: &str) -> String {
    let graph = GraphData {
        diagnostics: vec![note.into()],
        ..Default::default()
    };
    let graph = artifact::canonical(&graph, &[("repo".into(), root.into())]).unwrap();
    artifact::publish(root, &graph, &AtomicBool::new(false)).unwrap();
    blake3::hash(&fs::read(root.join(artifact::GRAPH_PATH)).unwrap())
        .to_hex()
        .to_string()
}

#[test]
fn busy_failure_is_structured_only_when_requested_and_old_envelope_is_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let _owner = AnalysisLock::acquire(root.path()).unwrap();
    let legacy = exchange(request(root.path()));
    assert_eq!(legacy.last().unwrap()["event"], "failed");
    assert!(legacy.last().unwrap().get("code").is_none());
    let mut typed = request(root.path());
    typed["failure_version"] = json!(1);
    let typed = exchange(typed);
    assert_eq!(typed.last().unwrap()["code"], "busy");
    assert!(
        typed.last().unwrap()["message"]
            .as_str()
            .unwrap()
            .contains("Another analysis")
    );
}

#[test]
fn intervening_manual_publication_rejects_planned_generation_under_child_lock() {
    let root = tempfile::tempdir().unwrap();
    let planned = publish(root.path(), "planned selection");
    let current = publish(root.path(), "intervening manual selection");
    let mut selected = request(root.path());
    selected["failure_version"] = json!(1);
    selected["expected_generation"] = json!(planned);
    let frames = exchange(selected);
    assert_eq!(frames.last().unwrap()["code"], "generation_changed");
    assert_eq!(
        blake3::hash(&fs::read(root.path().join(artifact::GRAPH_PATH)).unwrap())
            .to_hex()
            .to_string(),
        current
    );
}
