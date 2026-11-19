//! Seed canonical graph fixtures through the real library writer.
use codecallnavigator_core::artifact::{
    Artifact,
    local_snapshots::{Retention, SnapshotIdentity, Store},
    revision::Repository,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

pub fn retain(root: &Path, graph: &Artifact) -> PathBuf {
    let repository = Repository::open(root).unwrap();
    let store = Store::system().unwrap();
    let record = store
        .retain(
            &repository.common_dir,
            Retention {
                identity: SnapshotIdentity {
                    commit: repository.resolve("HEAD").unwrap(),
                    project: repository.project,
                    analyzer: "canonical graph fixture".into(),
                    selection: "{}".into(),
                },
                encoded: &graph.encode().unwrap(),
            },
            &AtomicBool::new(false),
        )
        .unwrap();
    store
        .snapshot_manifest_path(&repository.common_dir, &record)
        .unwrap()
}
