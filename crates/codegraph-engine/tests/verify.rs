//! Consumers verify portable snapshots without compiler processes or filesystem writes.
use codegraph_core::{
    artifact::{self, provenance},
    fingerprint,
    model::GraphData,
};
use serde_json::Value;
use std::{fs, path::Path, process::Command};

fn publish(root: &Path) -> Vec<u8> {
    fs::write(root.join("input.txt"), "one").unwrap();
    let roots = [
        ("repo".into(), root.into()),
        ("shared".into(), root.join("shared")),
    ];
    fs::create_dir(root.join("shared")).unwrap();
    let inputs = fingerprint::capture_excluding(
        [root.join("input.txt"), root.join("shared/input.txt")],
        &[],
    )
    .unwrap();
    let mut artifact = artifact::canonical(&GraphData::default(), &roots).unwrap();
    artifact.graph["analysis"] =
        serde_json::to_value(provenance::build(&inputs, &[], "", &roots).unwrap()).unwrap();
    let bytes = artifact.bytes().unwrap();
    fs::create_dir(root.join(".codegraph")).unwrap();
    fs::write(root.join(artifact::GRAPH_PATH), &bytes).unwrap();
    bytes
}
fn verify(root: &Path, shared: bool) -> (i32, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_codegraph-engine"));
    command.args(["verify", "--project"]).arg(root);
    if shared {
        command.args(["--root", "shared=shared"]);
    }
    let output = command.output().unwrap();
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}
#[test]
fn cli_checks_inputs_preserves_bytes_and_reports_mixed_failures() {
    let root = tempfile::tempdir().unwrap();
    let bytes = publish(root.path());
    let directory_before: Vec<_> = fs::read_dir(root.path().join(".codegraph"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    let (code, report) = verify(root.path(), true);
    assert_eq!(code, 0);
    assert_eq!(report["input_status"], "match");
    assert_eq!(report["digest"], blake3::hash(&bytes).to_hex().to_string());
    assert_eq!(verify(root.path(), false).0, 1);
    let original_time = fs::metadata(root.path().join("input.txt"))
        .unwrap()
        .modified()
        .unwrap();
    fs::write(root.path().join("input.txt"), "two").unwrap();
    fs::File::options()
        .write(true)
        .open(root.path().join("input.txt"))
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(original_time))
        .unwrap();
    let (code, report) = verify(root.path(), false);
    assert_eq!(code, 1);
    assert_eq!(report["input_status"], "stale");
    let reasons = report["reasons"].as_array().unwrap();
    assert!(
        reasons
            .iter()
            .any(|reason| reason["code"] == "input_changed")
    );
    assert!(
        reasons
            .iter()
            .any(|reason| reason["code"] == "unavailable_root")
    );
    assert_eq!(
        fs::read(root.path().join(artifact::GRAPH_PATH)).unwrap(),
        bytes
    );
    let directory_after: Vec<_> = fs::read_dir(root.path().join(".codegraph"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(directory_before, directory_after);
}
#[test]
fn cli_handles_legacy_malformed_and_missing_artifacts() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path());
    let legacy = artifact::canonical(
        &GraphData::default(),
        &[("repo".into(), root.path().into())],
    )
    .unwrap();
    fs::write(
        root.path().join(artifact::GRAPH_PATH),
        serde_json::to_vec_pretty(&legacy).unwrap(),
    )
    .unwrap();
    assert_eq!(verify(root.path(), true).1["input_status"], "unverifiable");
    fs::write(root.path().join(artifact::GRAPH_PATH), "{}").unwrap();
    let (code, report) = verify(root.path(), true);
    assert_eq!(code, 2);
    assert_eq!(report["contract_valid"], false);
    fs::remove_file(root.path().join(artifact::GRAPH_PATH)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codegraph-engine"))
        .args(["verify", "--project"])
        .arg(root.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
