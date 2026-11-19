//! Filesystem tests cover immutable handles, retention and independently failed parts.
//! In-memory round trips cannot prove these publication/read boundaries.
use super::*;
use crate::artifact::{self, Artifact};
use local_snapshots::{Store, WorkingPublication};
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

struct Fixture {
    _directory: tempfile::TempDir,
    project: std::path::PathBuf,
    store: Store,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let project = directory.path().join("checkout");
        fs::create_dir(&project).unwrap();
        let project = project.canonicalize().unwrap();
        let store = Store::at(directory.path().join("library"));
        Self {
            _directory: directory,
            project,
            store,
        }
    }

    fn path(&self) -> &Path {
        &self.project
    }

    fn publish(&self, artifact: &Artifact) {
        self.try_publish(artifact, false).unwrap();
    }

    fn try_publish(&self, artifact: &Artifact, cancelled: bool) -> anyhow::Result<()> {
        self.store.publish_working(
            self.path(),
            WorkingPublication {
                encoded: &artifact.encode()?,
                metadata: &serde_json::Value::Null,
                verified: None,
            },
            &AtomicBool::new(cancelled),
        )?;
        Ok(())
    }

    fn capture(&self) -> Captured {
        self.store.inspect_working(self.path()).unwrap().2.unwrap()
    }

    fn manifest(&self) -> Vec<u8> {
        fs::read(self.store.working_manifest_path(self.path()).unwrap()).unwrap()
    }
}

#[test]
fn compressed_publication_is_deterministic_and_roundtrips_complete_facts() {
    let root = Fixture::new();
    let artifact = artifact(root.path(), 0);
    let first = EncodedParts::encode(&artifact).unwrap();
    let repeated = EncodedParts::encode(&artifact).unwrap();
    assert_eq!(first.as_ref(), repeated.as_ref());
    assert_eq!(first.structure, repeated.structure);
    assert_eq!(first.flows, repeated.flows);
    root.publish(&artifact);
    let captured = root.capture();
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
    let root = Fixture::new();
    let first = artifact(root.path(), 0);
    root.publish(&first);
    let captured = root.capture();
    let manifest = Manifest::from_bytes(captured.manifest_bytes()).unwrap();
    let first_path = root.store.root().join(manifest.structure.path("structure"));
    root.publish(&artifact(root.path(), 1));
    root.publish(&artifact(root.path(), 1));
    assert!(
        first_path.exists(),
        "Previous generation must survive one publication"
    );
    root.publish(&artifact(root.path(), 2));
    assert!(!first_path.exists(), "Unreferenced object was not pruned");
    assert_eq!(
        captured.complete().unwrap().bytes().unwrap(),
        first.bytes().unwrap()
    );
    let directory = root.store.root().join("objects");
    assert_eq!(fs::read_dir(directory).unwrap().count(), 3); // two structures, shared flow
}

#[cfg(unix)]
#[test]
fn linked_flow_objects_fail_without_reading_outside_the_artifact_directory() {
    let root = Fixture::new();
    root.publish(&artifact(root.path(), 0));
    let bytes = root.manifest();
    let manifest = Manifest::from_bytes(&bytes).unwrap();
    let path = root.store.root().join(manifest.flows.path("flows"));
    let outside = root.path().join("outside");
    fs::rename(&path, &outside).unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    let captured = root.capture();
    assert!(captured.structure().is_ok());
    assert!(captured.load_flows().is_err());
}

#[test]
fn failed_flow_object_does_not_disable_structural_navigation() {
    let root = Fixture::new();
    root.publish(&artifact(root.path(), 0));
    let manifest = Manifest::from_bytes(&root.manifest()).unwrap();
    let path = root.store.root().join(manifest.flows.path("flows"));
    let saved = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    let captured = root.capture();
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
    root.publish(&without_flows);
    without_flows.graph["diagnostics"] = json!(["next structural generation"]);
    root.publish(&without_flows);
    assert!(!path.exists(), "Old flow object path should be pruned");
    assert!(
        captured.load_flows().unwrap().is_some(),
        "A successfully recovered handle must stay pinned after pruning"
    );
    root.publish(&artifact(root.path(), 0));
    let captured = root.capture();
    fs::write(&path, "corrupt").unwrap();
    assert!(captured.structure().is_ok());
    assert!(captured.load_flows().is_err());
    root.publish(&artifact(root.path(), 0));
    assert!(
        captured.load_flows().unwrap().is_some(),
        "Reanalysis must repair a corrupt pinned flow object"
    );
}

#[test]
fn cancelled_publication_preserves_exact_manifest_and_existing_objects() {
    let root = Fixture::new();
    root.publish(&artifact(root.path(), 0));
    let before = root.manifest();
    let result = root.try_publish(&artifact(root.path(), 1), true);
    assert!(result.is_err());
    assert_eq!(root.manifest(), before);
    assert!(root.capture().complete().is_ok());
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
    let root = Fixture::new();
    let artifact = artifact(root.path(), 0);
    root.publish(&artifact);
    let directory = root.store.root().to_path_buf();
    let manifest = root.manifest();
    for (name, expected) in companions::FILES {
        let path = directory.join(name);
        assert_eq!(fs::read(&path).unwrap(), expected);
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        root.publish(&artifact);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), modified);
        fs::write(&path, b"obsolete companion").unwrap();
        root.publish(&artifact);
        assert_eq!(fs::read(&path).unwrap(), expected);
        fs::remove_file(&path).unwrap();
        root.publish(&artifact);
        assert_eq!(fs::read(&path).unwrap(), expected);
    }
    assert_eq!(root.manifest(), manifest);
    let readme = fs::read_to_string(directory.join("README.md")).unwrap();
    assert!(readme.split_whitespace().count() < 200);
}

#[test]
fn graph_reads_do_not_repair_or_require_companions() {
    let root = Fixture::new();
    root.publish(&artifact(root.path(), 0));
    let directory = root.store.root().to_path_buf();
    fs::remove_file(directory.join("graph.schema.json")).unwrap();
    fs::write(directory.join("README.md"), b"custom text").unwrap();
    let modified = fs::metadata(directory.join("README.md"))
        .unwrap()
        .modified()
        .unwrap();
    assert!(root.capture().complete().is_ok());
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
    let root = Fixture::new();
    root.publish(&artifact(root.path(), 0));
    let directory = root.store.root().to_path_buf();
    let manifest = root.manifest();
    let path = directory.join("graph.schema.json");
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    let result = root.try_publish(&artifact(root.path(), 1), false);
    assert!(format!("{:#}", result.unwrap_err()).contains("Cannot publish graph companion"));
    assert_eq!(root.manifest(), manifest);
    assert!(root.capture().complete().is_ok());
    fs::remove_dir(&path).unwrap();
    root.publish(&artifact(root.path(), 1));
    assert_ne!(root.manifest(), manifest);
    assert_eq!(fs::read(path).unwrap(), companions::FILES[1].1);
}

#[cfg(unix)]
#[test]
fn linked_companion_is_rejected_without_touching_its_target() {
    let root = Fixture::new();
    root.publish(&artifact(root.path(), 0));
    let directory = root.store.root().to_path_buf();
    let manifest = root.manifest();
    let path = directory.join("README.md");
    fs::remove_file(&path).unwrap();
    let outside = root.path().join("outside");
    fs::write(&outside, b"outside bytes").unwrap();
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    assert!(root.try_publish(&artifact(root.path(), 1), false).is_err());
    assert_eq!(root.manifest(), manifest);
    assert_eq!(fs::read(outside).unwrap(), b"outside bytes");
}
