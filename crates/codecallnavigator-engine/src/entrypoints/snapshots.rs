//! Engine-owned local snapshot preparation and discovery.
mod selection;

use super::{analysis, cancellation::CANCELLED, snapshot_worktree::ResolvedRevision};
use anyhow::{Context, Result, ensure};
use codecallnavigator_core::{
    artifact::{
        local_snapshots::{SnapshotRecord, Store, VerifiedPublication},
        revision::Repository,
    },
    discovery::BuildTarget,
};
pub(super) use selection::Selection;
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    thread,
    time::Duration,
};

#[derive(Serialize)]
pub(super) struct Status {
    pub cache_path: PathBuf,
    pub commit: String,
    pub snapshots: Vec<SnapshotRecord>,
}

pub(super) fn status(project: &Path, revision: &str) -> Result<Status> {
    let repository = Repository::open(project)?;
    let commit = repository.resolve(revision)?;
    let store = Store::system()?;
    Ok(Status {
        cache_path: store.repository_path(&repository.common_dir),
        snapshots: records(&store, &repository, &commit)?,
        commit,
    })
}

pub(super) fn prepare(
    project: &Path,
    revision: &str,
    selection: Selection,
    progress: &dyn Fn(String),
) -> Result<SnapshotRecord> {
    let selection = selection.normalize()?;
    let project = project
        .canonicalize()
        .context("Cannot open snapshot project")?;
    let resolved = ResolvedRevision::resolve(&project, revision, &CANCELLED)?;
    let repository = Repository::open(&project)?;
    let store = Store::system()?;
    let key = serde_json::to_vec(&(
        &resolved.commit,
        project.as_os_str().as_encoded_bytes(),
        &repository.project,
        &selection,
        env!("CARGO_PKG_VERSION"),
    ))?;
    let job = Preparation::open(&store, &repository, blake3::hash(&key).to_hex().as_str())?;
    // Serialize equivalent requests from this checkout. Even after waiting for
    // one, observe the caller's dependencies and toolchain before registering.
    let _lease = job.acquire(&resolved.commit, progress)?;

    progress(format!("Preparing local snapshot at {}", resolved.commit));
    let staged = resolved.with_checkout(&CANCELLED, |checkout| {
        let selected = selection.resolve(checkout, revision)?;
        analyze_revision(&project, checkout, selected, progress)
    })?;
    let identity = staged.verified.identity(&staged.metadata)?;
    ensure!(
        staged.verified.common_dir() == repository.common_dir
            && identity.project == repository.project
            && identity.commit == resolved.commit,
        "Verified snapshot does not belong to the requested clone, project, and commit"
    );
    store.retain_verified(
        &repository.common_dir,
        VerifiedPublication {
            identity,
            encoded: &staged.encoded,
            metadata: &staged.metadata,
        },
        &CANCELLED,
    )
}

fn analyze_revision(
    project: &Path,
    checkout: &Path,
    selected: Vec<BuildTarget>,
    progress: &dyn Fn(String),
) -> Result<analysis::PreparedRevision> {
    for target in &selected {
        if target.language == "typescript" {
            let directory = target
                .path
                .parent()
                .context("TypeScript target has no directory")?;
            borrow_node_modules(project, checkout, directory)?;
        }
    }
    analysis::prepare_revision(checkout, selected, progress)
}

fn borrow_node_modules(project: &Path, checkout: &Path, target: &Path) -> Result<()> {
    for directory in target
        .ancestors()
        .take_while(|path| path.starts_with(checkout))
    {
        let relative = directory.strip_prefix(checkout)?;
        let original = project.join(relative).join("node_modules");
        let temporary = directory.join("node_modules");
        if !original.is_dir() {
            continue;
        }
        match fs::symlink_metadata(&temporary) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::os::unix::fs::symlink(original.canonicalize()?, temporary)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
fn records(store: &Store, repository: &Repository, commit: &str) -> Result<Vec<SnapshotRecord>> {
    store.list(&repository.common_dir, commit, &repository.project)
}

struct Preparation {
    directory: PathBuf,
    key: String,
}

impl Preparation {
    fn open(store: &Store, repository: &Repository, key: &str) -> Result<Self> {
        let directory = store
            .repository_path(&repository.common_dir)
            .join("preparation");
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory)?;
        ensure!(
            fs::symlink_metadata(&directory)?.file_type().is_dir(),
            "Snapshot preparation location must be a real directory"
        );
        Ok(Self {
            directory,
            key: key.to_owned(),
        })
    }

    fn acquire(&self, commit: &str, progress: &dyn Fn(String)) -> Result<File> {
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let lock = options.open(self.directory.join(format!("{}.lock", self.key)))?;
        ensure!(
            lock.metadata()?.is_file(),
            "Snapshot preparation lock must be a file"
        );
        wait_for_lease(&lock, commit, progress)?;
        Ok(lock)
    }
}

fn wait_for_lease(lock: &File, commit: &str, progress: &dyn Fn(String)) -> Result<()> {
    let mut announced = false;
    loop {
        ensure!(
            !CANCELLED.load(Ordering::Acquire),
            "Snapshot preparation cancelled"
        );
        match lock.try_lock() {
            Ok(()) => return Ok(()),
            Err(TryLockError::WouldBlock) => {
                if !announced {
                    progress(format!("Waiting for snapshot preparation at {}", commit));
                    announced = true;
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(TryLockError::Error(error)) => {
                return Err(error).context("Cannot lock snapshot preparation");
            }
        }
    }
}
