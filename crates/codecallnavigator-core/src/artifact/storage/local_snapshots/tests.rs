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
    assert_ne!(first.id, second.id);
    assert_eq!(first.digest, second.digest);
    assert!(store.capture(&second_clone, &first).is_err());
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
    store.budget = [&first_bundle, &second_bundle]
        .into_iter()
        .map(|bundle| {
            bundle.manifest.structure.compressed_bytes + bundle.manifest.flows.compressed_bytes
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

#[test]
fn working_and_verified_references_share_a_manifest_and_survive_history_budget() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().canonicalize().unwrap();
    let repository = project.join(".git");
    let mut store = Store::at(project.join("library"));
    store.budget = 0;
    let encoded = bundle(&project, 1);
    let identity = identity('a');
    let metadata = json!({"targets":[{"kind":"maven","analyzer":"java"}]});
    let cancel = AtomicBool::new(false);

    let record = store
        .publish_working(
            &project,
            WorkingPublication {
                encoded: &encoded,
                metadata: &metadata,
                verified: Some((&repository, identity.clone())),
            },
            &cancel,
        )
        .unwrap()
        .unwrap();
    let working_path = store.working_manifest_path(&project).unwrap();
    let snapshot_path = store.snapshot_manifest_path(&repository, &record).unwrap();
    assert_eq!(working_path, snapshot_path);
    assert!(
        snapshot_path
            .components()
            .any(|part| part.as_os_str() == identity.commit.as_str())
    );
    assert_eq!(records(&store, &repository, &identity), [record]);

    let next = bundle(&project, 2);
    store
        .publish_working(
            &project,
            WorkingPublication {
                encoded: &next,
                metadata: &json!({"mode":"working"}),
                verified: None,
            },
            &cancel,
        )
        .unwrap();
    assert!(snapshot_path.exists());
    assert_eq!(records(&store, &repository, &identity).len(), 1);
    let newest = bundle(&project, 3);
    store
        .publish_working(
            &project,
            WorkingPublication {
                encoded: &newest,
                metadata: &Value::Null,
                verified: None,
            },
            &cancel,
        )
        .unwrap();
    assert!(!snapshot_path.exists());
    assert!(records(&store, &repository, &identity).is_empty());
}

#[test]
fn copied_manifest_and_altered_catalog_cannot_relabel_a_verified_commit() {
    let directory = tempfile::tempdir().unwrap();
    let repository = directory.path().join(".git");
    let store = Store::at(directory.path().join("library"));
    let encoded = bundle(directory.path(), 1);
    let original = identity('a');
    let other = identity('b');
    let record = store
        .retain(
            &repository,
            retention(original, &encoded),
            &AtomicBool::new(false),
        )
        .unwrap();
    let project = directory.path().canonicalize().unwrap();
    store
        .publish_working(
            &project,
            WorkingPublication {
                encoded: &encoded,
                metadata: &Value::Null,
                verified: None,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    let db = catalog::open(store.root(), true).unwrap();
    db.execute("UPDATE working SET current=?1", [&record.id])
        .unwrap();
    assert!(
        store
            .working_manifest_path(&project)
            .unwrap_err()
            .to_string()
            .contains("reference mismatch")
    );
    let original_path = store.snapshot_manifest_path(&repository, &record).unwrap();
    let mut relabeled = record.clone();
    relabeled.identity = other.clone();
    let copied_path = store
        .snapshot_manifest_path(&repository, &relabeled)
        .unwrap();
    fs::create_dir_all(copied_path.parent().unwrap()).unwrap();
    fs::copy(original_path, copied_path).unwrap();
    assert!(records(&store, &repository, &other).is_empty());
    assert!(store.capture(&repository, &relabeled).is_err());

    db.execute(
        "UPDATE bundles SET sha=?1 WHERE id=?2",
        [&other.commit, &record.id],
    )
    .unwrap();
    let error = store
        .list(&repository, &other.commit, &other.project)
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("another clone, project, or commit")
    );
}

fn save_working(store: &Store, project: &Path, encoded: &EncodedArtifact) {
    store
        .publish_working(
            project,
            WorkingPublication {
                encoded,
                metadata: &Value::Null,
                verified: None,
            },
            &AtomicBool::new(false),
        )
        .unwrap();
}

#[test]
fn concurrent_writers_preserve_each_worktree_and_open_reader() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    let store = Store::at(base.join("library"));
    let projects = [base.join("checkout-a"), base.join("checkout-b")];
    for project in &projects {
        fs::create_dir(project).unwrap();
        save_working(&store, project, &bundle(project, 0));
    }
    let pinned = store.inspect_working(&projects[0]).unwrap().2.unwrap();
    let barrier = std::sync::Barrier::new(projects.len());
    std::thread::scope(|scope| {
        for (index, project) in projects.iter().enumerate() {
            let store = &store;
            let barrier = &barrier;
            scope.spawn(move || {
                let candidate = bundle(project, (index + 1) as u8);
                barrier.wait();
                save_working(store, project, &candidate);
            });
        }
    });

    for (index, project) in projects.iter().enumerate() {
        let (_, bytes, graph) = store.inspect_working(project).unwrap();
        assert_eq!(bytes, bundle(project, (index + 1) as u8).as_ref());
        graph.unwrap().complete().unwrap();
    }
    assert_eq!(
        pinned.complete().unwrap().graph["diagnostics"],
        json!(["run 0"])
    );
    assert_eq!(
        fs::read_dir(store.root().join("objects")).unwrap().count(),
        4
    );
}

#[test]
fn abrupt_writer_exit_preserves_committed_references_and_reclaims_staged_objects() {
    const PROJECT_ENV: &str = "CCNAVIGATOR_TEST_INTERRUPTED_LIBRARY_PROJECT";
    if let Some(project) = std::env::var_os(PROJECT_ENV).map(PathBuf::from) {
        exit_before_reference_commit(&project);
    }
    let root = tempfile::tempdir().unwrap();
    let project = root.path().canonicalize().unwrap();
    let store = Store::at(project.join("library"));
    let previous = bundle(&project, 0);
    save_working(&store, &project, &previous);
    let staged = bundle(&project, 1);
    let orphan = store
        .root()
        .join(staged.manifest.structure.path("structure"));

    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "artifact::storage::local_snapshots::tests::abrupt_writer_exit_preserves_committed_references_and_reclaims_staged_objects"])
        .env(PROJECT_ENV, &project)
        .output()
        .unwrap();
    assert_eq!(
        child.status.code(),
        Some(86),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(orphan.is_file());
    let (_, bytes, graph) = store.inspect_working(&project).unwrap();
    assert_eq!(bytes, previous.as_ref());
    graph.unwrap().complete().unwrap();

    save_working(&store, &project, &bundle(&project, 2));
    assert!(
        !orphan.exists(),
        "A later publication must reclaim interrupted files"
    );
}

fn exit_before_reference_commit(project: &Path) -> ! {
    let store = Store::at(project.join("library"));
    let candidate = bundle(project, 1);
    let key = path_key(project);
    let entry = Entry::working(key.clone(), key.clone(), &Value::Null, candidate.as_ref()).unwrap();
    let id = entry.id().unwrap();
    let path = entry.path(&id).unwrap();
    let cancel = AtomicBool::new(false);
    let _guard = store.write_guard(&cancel).unwrap();
    let mut db = catalog::open(store.root(), true).unwrap();
    store.check_directories(&path, true).unwrap();
    let tx = db.transaction().unwrap();
    catalog::insert(&tx, &id, &entry, false).unwrap();
    catalog::set_current(&tx, &key, &id).unwrap();
    store.write_bundle(&path, &candidate, &cancel).unwrap();
    // Skip all destructors at precisely the boundary between writing the
    // immutable files and committing their reference transaction.
    std::process::exit(86);
}
