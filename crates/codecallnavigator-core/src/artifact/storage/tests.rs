//! Filesystem tests cover immutable handles, retention and independently failed parts.
//! In-memory round trips cannot prove these publication/read boundaries.
use super::*;
use crate::artifact::{self, Artifact};
use serde_json::json;
use std::{fs, path::Path, sync::atomic::AtomicBool};

fn artifact(root: &Path, sequence: usize) -> Artifact {
    let mut artifact = artifact::canonical(
        &crate::model::GraphData::default(),
        &[("repo".into(), root.into())],
    )
    .unwrap();
    artifact.graph["diagnostics"] = json!([format!("generation {sequence}")]);
    artifact.graph["flow_inventory"] = json!({"version":1,"functions":[]});
    artifact
}

fn publish(root: &Path, artifact: &Artifact) {
    artifact::publish(root, artifact, &AtomicBool::new(false)).unwrap();
}

#[test]
fn compressed_publication_is_deterministic_and_roundtrips_complete_facts() {
    let root = tempfile::tempdir().unwrap();
    let artifact = artifact(root.path(), 0);
    let first = EncodedParts::encode(&artifact).unwrap();
    let repeated = EncodedParts::encode(&artifact).unwrap();
    assert_eq!(first.as_ref(), repeated.as_ref());
    assert_eq!(first.structure, repeated.structure);
    assert_eq!(first.flows, repeated.flows);
    publish(root.path(), &artifact);
    let captured = Captured::open(root.path()).unwrap();
    assert_eq!(captured.manifest_bytes(), first.as_ref());
    let structure = captured.structure().unwrap();
    assert!(structure.graph.get("flow_inventory").is_none());
    assert_eq!(structure.graph["flow_catalogue"]["inventory_version"], 1);
    assert_eq!(
        captured
            .load_flows_for(&structure)
            .unwrap()
            .unwrap()
            .functions
            .len(),
        0
    );
    assert_eq!(
        captured.complete().unwrap().bytes().unwrap(),
        artifact.bytes().unwrap()
    );
}

#[test]
fn captured_handles_survive_pruning_and_only_current_and_previous_parts_remain() {
    let root = tempfile::tempdir().unwrap();
    let first = artifact(root.path(), 0);
    publish(root.path(), &first);
    let captured = Captured::open(root.path()).unwrap();
    let manifest = Manifest::from_bytes(captured.manifest_bytes()).unwrap();
    let first_path = root
        .path()
        .join(".codecallnavigator")
        .join(manifest.structure.path("structure"));
    publish(root.path(), &artifact(root.path(), 1));
    publish(root.path(), &artifact(root.path(), 1));
    assert!(
        first_path.exists(),
        "Previous generation must survive one publication"
    );
    publish(root.path(), &artifact(root.path(), 2));
    assert!(!first_path.exists(), "Unreferenced object was not pruned");
    assert_eq!(
        captured.complete().unwrap().bytes().unwrap(),
        first.bytes().unwrap()
    );
    let directory = root.path().join(".codecallnavigator/objects");
    assert_eq!(fs::read_dir(directory).unwrap().count(), 3); // two structures, shared flow
}

#[cfg(unix)]
#[test]
fn linked_flow_objects_fail_without_reading_outside_the_artifact_directory() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path(), &artifact(root.path(), 0));
    let bytes = fs::read(root.path().join(artifact::GRAPH_PATH)).unwrap();
    let manifest = Manifest::from_bytes(&bytes).unwrap();
    let path = root
        .path()
        .join(".codecallnavigator")
        .join(manifest.flows.path("flows"));
    let outside = root.path().join("outside");
    fs::rename(&path, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    let captured = Captured::open(root.path()).unwrap();
    assert!(captured.structure().is_ok());
    assert!(captured.load_flows().is_err());
}

#[test]
fn failed_flow_object_does_not_disable_structural_navigation() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path(), &artifact(root.path(), 0));
    let manifest =
        Manifest::from_bytes(&fs::read(root.path().join(artifact::GRAPH_PATH)).unwrap()).unwrap();
    let path = root
        .path()
        .join(".codecallnavigator")
        .join(manifest.flows.path("flows"));
    let saved = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    let captured = Captured::open(root.path()).unwrap();
    assert!(captured.structure().is_ok());
    assert!(captured.load_flows().is_err());
    assert!(captured.complete().is_err());
    fs::write(&path, saved).unwrap();
    assert!(
        captured.load_flows().unwrap().is_some(),
        "Restoring an object must recover the same captured generation"
    );
    let mut without_flows = artifact(root.path(), 1);
    without_flows.graph["flow_inventory"] = serde_json::Value::Null;
    publish(root.path(), &without_flows);
    without_flows.graph["diagnostics"] = json!(["next structural generation"]);
    publish(root.path(), &without_flows);
    assert!(!path.exists(), "Old flow object path should be pruned");
    assert!(
        captured.load_flows().unwrap().is_some(),
        "A successfully recovered handle must stay pinned after pruning"
    );
    publish(root.path(), &artifact(root.path(), 0));
    let captured = Captured::open(root.path()).unwrap();
    fs::write(&path, "corrupt").unwrap();
    assert!(captured.structure().is_ok());
    assert!(captured.load_flows().is_err());
    publish(root.path(), &artifact(root.path(), 0));
    assert!(
        captured.load_flows().unwrap().is_some(),
        "Reanalysis must repair a corrupt pinned flow object"
    );
}

#[test]
fn cancelled_publication_preserves_exact_manifest_and_existing_objects() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path(), &artifact(root.path(), 0));
    let before = fs::read(root.path().join(artifact::GRAPH_PATH)).unwrap();
    let result = artifact::publish(
        root.path(),
        &artifact(root.path(), 1),
        &AtomicBool::new(true),
    );
    assert!(result.is_err());
    assert_eq!(
        fs::read(root.path().join(artifact::GRAPH_PATH)).unwrap(),
        before
    );
    assert!(Captured::open(root.path()).unwrap().complete().is_ok());
}

#[test]
fn corruption_lengths_and_legacy_graphs_are_rejected() {
    let artifact = artifact(Path::new("/fixture"), 0);
    let encoded = EncodedParts::encode(&artifact).unwrap();
    let mut part = encoded.manifest.flows.clone();
    part.uncompressed_bytes += 1;
    assert!(part.decode(encoded.flows.as_slice()).is_err());
    part = encoded.manifest.flows.clone();
    part.digest = "a".repeat(64);
    assert!(part.decode(encoded.flows.as_slice()).is_err());
    let mut manifest = serde_json::to_value(&encoded.manifest).unwrap();
    manifest["flows"]["digest"] = json!("../../outside");
    assert!(Manifest::from_bytes(&serde_json::to_vec(&manifest).unwrap()).is_err());
    assert!(Manifest::from_bytes(&artifact.bytes().unwrap()).is_err());
    let mut embedded = artifact;
    embedded.graph["flow_catalogue"] = json!({"inventory_version":1,"symbols":[]});
    assert!(super::codec::decode_structure(&embedded.bytes().unwrap()).is_err());
}

// Companion persistence and repair require real filesystem entries; pure encoding
// tests cannot prove preservation, replacement, or read-only loading behavior.
#[test]
fn publication_distributes_and_repairs_contracts_without_changing_generation() {
    let root = tempfile::tempdir().unwrap();
    let artifact = artifact(root.path(), 0);
    publish(root.path(), &artifact);
    let directory = root.path().join(".codecallnavigator");
    let manifest = fs::read(directory.join("graph.json")).unwrap();
    for (name, expected) in companions::FILES {
        let path = directory.join(name);
        assert_eq!(fs::read(&path).unwrap(), expected);
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        publish(root.path(), &artifact);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
        fs::write(&path, b"obsolete companion").unwrap();
        publish(root.path(), &artifact);
        assert_eq!(fs::read(&path).unwrap(), expected);
        fs::remove_file(&path).unwrap();
        publish(root.path(), &artifact);
        assert_eq!(fs::read(&path).unwrap(), expected);
    }
    assert_eq!(fs::read(directory.join("graph.json")).unwrap(), manifest);
    let readme = fs::read_to_string(directory.join("README.md")).unwrap();
    assert!(readme.split_whitespace().count() < 200);
}

#[test]
fn graph_reads_do_not_repair_or_require_companions() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path(), &artifact(root.path(), 0));
    let directory = root.path().join(".codecallnavigator");
    fs::remove_file(directory.join("graph.schema.json")).unwrap();
    fs::write(directory.join("README.md"), b"custom text").unwrap();
    let modified = fs::metadata(directory.join("README.md"))
        .unwrap()
        .modified()
        .unwrap();
    assert!(Captured::open(root.path()).unwrap().complete().is_ok());
    assert!(!directory.join("graph.schema.json").exists());
    assert_eq!(
        fs::read(directory.join("README.md")).unwrap(),
        b"custom text"
    );
    assert_eq!(
        fs::metadata(directory.join("README.md"))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
}

#[test]
fn companion_write_failure_preserves_active_manifest_and_can_recover() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path(), &artifact(root.path(), 0));
    let directory = root.path().join(".codecallnavigator");
    let manifest = fs::read(directory.join("graph.json")).unwrap();
    let path = directory.join("graph.schema.json");
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    let result = artifact::publish(
        root.path(),
        &artifact(root.path(), 1),
        &AtomicBool::new(false),
    );
    assert!(format!("{:#}", result.unwrap_err()).contains("Cannot publish graph companion"));
    assert_eq!(fs::read(directory.join("graph.json")).unwrap(), manifest);
    assert!(Captured::open(root.path()).unwrap().complete().is_ok());
    fs::remove_dir(&path).unwrap();
    publish(root.path(), &artifact(root.path(), 1));
    assert_ne!(fs::read(directory.join("graph.json")).unwrap(), manifest);
    assert_eq!(fs::read(path).unwrap(), companions::FILES[1].1);
}

#[cfg(unix)]
#[test]
fn linked_companion_is_rejected_without_touching_its_target() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path(), &artifact(root.path(), 0));
    let directory = root.path().join(".codecallnavigator");
    let manifest = fs::read(directory.join("graph.json")).unwrap();
    let path = directory.join("README.md");
    fs::remove_file(&path).unwrap();
    let outside = root.path().join("outside");
    fs::write(&outside, b"outside bytes").unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    assert!(
        artifact::publish(
            root.path(),
            &artifact(root.path(), 1),
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert_eq!(fs::read(directory.join("graph.json")).unwrap(), manifest);
    assert_eq!(fs::read(outside).unwrap(), b"outside bytes");
}
