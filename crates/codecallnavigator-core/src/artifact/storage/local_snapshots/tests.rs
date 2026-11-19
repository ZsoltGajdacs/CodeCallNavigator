//! Real cache files exercise coordination and immutable reader lifetimes.

use super::*;
use crate::{artifact, model::GraphData};
use serde_json::json;

fn bundle(root: &Path, sequence: u8) -> EncodedArtifact {
    let mut artifact =
        artifact::canonical(&GraphData::default(), &[("repo".into(), root.into())]).unwrap();
    artifact.graph["diagnostics"] = json!([format!("run {sequence}")]);
    artifact.encode().unwrap()
}

fn identity(commit_digit: char) -> SnapshotIdentity {
    SnapshotIdentity {
        commit: commit_digit.to_string().repeat(40),
        project: "service/".into(),
        analyzer: "1".into(),
        selection: "default".into(),
    }
}

fn retention(identity: SnapshotIdentity, encoded: &EncodedArtifact) -> Retention<'_> {
    Retention { identity, encoded }
}

fn records(store: &Store, repository: &Path, identity: &SnapshotIdentity) -> Vec<SnapshotRecord> {
    store
        .list(repository, &identity.commit, &identity.project)
        .unwrap()
}

#[test]
fn distinct_clones_share_objects_and_keep_separate_indexes() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("cache");
    let first_clone = root.path().join("primary/.git");
    let second_clone = root.path().join("mirror/.git");
    let store = Store::at(cache.clone());
    let identity = identity('a');
    let encoded = bundle(root.path(), 1);
    let cancel = AtomicBool::new(false);

    assert!(records(&store, &first_clone, &identity).is_empty());
    assert!(!cache.exists());

    let first = store
        .retain(&first_clone, retention(identity.clone(), &encoded), &cancel)
        .unwrap();
    assert!(records(&store, &second_clone, &identity).is_empty());
    let second = store
        .retain(
            &second_clone,
            retention(identity.clone(), &encoded),
            &cancel,
        )
        .unwrap();
    assert_eq!(first, second);
    assert_ne!(
        store.repository_path(&first_clone),
        store.repository_path(&second_clone)
    );
    assert_eq!(std::fs::read_dir(cache.join("objects")).unwrap().count(), 2);
    assert_eq!(
        records(&store, &first_clone, &identity),
        std::slice::from_ref(&first)
    );
    let opened = store.capture(&first_clone, &first).unwrap();
    assert_eq!(opened.manifest_bytes(), encoded.as_ref());
    assert_eq!(
        opened.complete().unwrap().graph["diagnostics"],
        json!(["run 1"])
    );
}

#[test]
fn oldest_snapshot_eviction_preserves_open_bundle_handles() {
    let root = tempfile::tempdir().unwrap();
    let first_clone = root.path().join("primary/.git");
    let second_clone = root.path().join("mirror/.git");
    let mut store = Store::at(root.path().join("cache"));
    let first_bundle = bundle(root.path(), 1);
    let second_bundle = bundle(root.path(), 2);
    let first_identity = identity('a');
    let second_identity = identity('b');
    store.budget = [
        (&first_identity, &first_bundle),
        (&second_identity, &second_bundle),
    ]
    .into_iter()
    .map(|(identity, bundle)| {
        let entry = Entry::new(identity.clone(), bundle).unwrap();
        let metadata = serde_json::to_vec(&entry).unwrap().len() as u64;
        metadata
            + bundle.manifest.structure.compressed_bytes
            + bundle.manifest.flows.compressed_bytes
    })
    .max()
    .unwrap();

    let cancel = AtomicBool::new(false);
    let first = store
        .retain(
            &first_clone,
            retention(first_identity.clone(), &first_bundle),
            &cancel,
        )
        .unwrap();
    let opened = store.capture(&first_clone, &first).unwrap();
    let second = store
        .retain(
            &second_clone,
            retention(second_identity.clone(), &second_bundle),
            &cancel,
        )
        .unwrap();

    assert!(records(&store, &first_clone, &first_identity).is_empty());
    assert!(store.capture(&first_clone, &first).is_err());
    assert_eq!(records(&store, &second_clone, &second_identity), [second]);
    let old_structure = store
        .root()
        .join(first_bundle.manifest.structure.path("structure"));
    assert!(!old_structure.exists());
    assert_eq!(
        opened.complete().unwrap().graph["diagnostics"],
        json!(["run 1"])
    );
}

#[test]
fn corrupt_object_is_detected_and_republication_repairs_only_that_object() {
    let root = tempfile::tempdir().unwrap();
    let repository = root.path().join(".git");
    let store = Store::at(root.path().join("cache"));
    let identity = identity('a');
    let encoded = bundle(root.path(), 1);
    let cancel = AtomicBool::new(false);
    let record = store
        .retain(&repository, retention(identity.clone(), &encoded), &cancel)
        .unwrap();
    let saved = store.capture(&repository, &record).unwrap();
    assert_eq!(saved.manifest_bytes(), encoded.as_ref());
    let flow = store.root().join(encoded.manifest.flows.path("flows"));
    let structure = store
        .root()
        .join(encoded.manifest.structure.path("structure"));
    let correct_structure = fs::read(&structure).unwrap();
    let mut wrong_bytes = encoded.flows.clone();
    wrong_bytes[0] ^= 1;
    fs::write(&flow, wrong_bytes).unwrap();
    assert!(
        store
            .capture(&repository, &record)
            .unwrap()
            .complete()
            .unwrap_err()
            .to_string()
            .contains("digest mismatch")
    );
    assert_eq!(
        store
            .retain(&repository, retention(identity, &encoded), &cancel)
            .unwrap(),
        record
    );
    let repaired = store.capture(&repository, &record).unwrap();
    assert_eq!(fs::read(structure).unwrap(), correct_structure);
    assert_eq!(
        repaired.complete().unwrap().graph["diagnostics"],
        json!(["run 1"])
    );
}

#[test]
fn cancelled_and_over_budget_writes_preserve_preexisting_snapshots() {
    let root = tempfile::tempdir().unwrap();
    let repository = root.path().join(".git");
    let mut store = Store::at(root.path().join("cache"));
    let first_bundle = bundle(root.path(), 1);
    let second_bundle = bundle(root.path(), 2);
    let first_identity = identity('a');
    let second_identity = identity('b');
    let mut cancel = AtomicBool::new(false);
    let record = store
        .retain(
            &repository,
            retention(first_identity.clone(), &first_bundle),
            &cancel,
        )
        .unwrap();

    *cancel.get_mut() = true;
    assert!(
        store
            .retain(
                &repository,
                retention(second_identity.clone(), &second_bundle),
                &cancel
            )
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
    *cancel.get_mut() = false;
    store.budget = 1;
    assert!(
        store
            .retain(
                &repository,
                retention(second_identity.clone(), &second_bundle),
                &cancel
            )
            .unwrap_err()
            .to_string()
            .contains("size limit")
    );
    assert!(records(&store, &repository, &second_identity).is_empty());
    assert_eq!(
        records(&store, &repository, &first_identity),
        std::slice::from_ref(&record)
    );
    assert_eq!(
        store
            .capture(&repository, &record)
            .unwrap()
            .complete()
            .unwrap()
            .graph["diagnostics"],
        json!(["run 1"])
    );
}
