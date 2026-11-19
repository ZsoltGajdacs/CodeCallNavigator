//! Engine-owned local snapshot preparation and discovery.
use super::{analysis, cancellation::CANCELLED, snapshot_worktree::ResolvedRevision};
use anyhow::{Context, Result, ensure};
use codecallnavigator_core::{
    artifact::{
        local_snapshots::{SnapshotRecord, Store},
        revision::Repository,
    },
    protocol::Event,
};
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::Write,
    path::{Component, Path, PathBuf},
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
    mut targets: Vec<PathBuf>,
    progress: &dyn Fn(String),
) -> Result<SnapshotRecord> {
    targets.sort();
    targets.dedup();
    for target in &targets {
        ensure!(
            target
                .components()
                .all(|part| matches!(part, Component::Normal(_) | Component::CurDir)),
            "Snapshot targets must be project-relative paths without parent traversal"
        );
    }
    let resolved = ResolvedRevision::resolve(project, revision, &CANCELLED)?;
    let repository = Repository::open(project)?;
    let store = Store::system()?;
    let key = serde_json::to_vec(&(
        &resolved.commit,
        &repository.project,
        &targets,
        env!("CARGO_PKG_VERSION"),
    ))?;
    let job = Preparation::open(&store, &repository, blake3::hash(&key).to_hex().as_str())?;
    let (_lease, waited) = job.acquire(&resolved.commit, progress)?;
    if waited && let Some(snapshot) = job.reuse(&store, &repository, &resolved.commit)? {
        return Ok(snapshot);
    }
    job.clear_receipt()?;

    progress(format!("Preparing local snapshot at {}", resolved.commit));
    let digest = resolved.with_checkout(&CANCELLED, |checkout| {
        analyze_revision(project, checkout, targets, progress)
    })?;
    let snapshot = records(&store, &repository, &resolved.commit)?
        .into_iter()
        .find(|record| record.digest == digest)
        .context("Analysis completed but the revision snapshot could not be retained")?;
    job.record(&snapshot)?;
    Ok(snapshot)
}

fn analyze_revision(
    project: &Path,
    checkout: &Path,
    targets: Vec<PathBuf>,
    progress: &dyn Fn(String),
) -> Result<String> {
    for target in &targets {
        let path = checkout.join(target).canonicalize()?;
        ensure!(
            path.starts_with(checkout),
            "Snapshot target resolves outside its project"
        );
    }
    let selected = analysis::select_targets(checkout, targets)?;
    ensure!(!selected.is_empty(), "No analysis targets in this revision");
    for target in &selected {
        if target.language == "typescript" {
            let directory = target
                .path
                .parent()
                .context("TypeScript target has no directory")?;
            borrow_node_modules(project, checkout, directory)?;
        }
    }
    let Event::Complete { digest, .. } =
        analysis::analyze(checkout, Vec::new(), selected, progress)?
    else {
        anyhow::bail!("Unexpected snapshot analysis result");
    };
    Ok(digest)
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

    fn acquire(&self, commit: &str, progress: &dyn Fn(String)) -> Result<(File, bool)> {
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
        let waited = wait_for_lease(&lock, commit, progress)?;
        Ok((lock, waited))
    }

    fn reuse(
        &self,
        store: &Store,
        repository: &Repository,
        commit: &str,
    ) -> Result<Option<SnapshotRecord>> {
        let path = self.directory.join(&self.key);
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            return Ok(None);
        };
        if !metadata.is_file() || metadata.len() != 64 {
            return Ok(None);
        }
        let id = fs::read_to_string(path)?;
        let Some(snapshot) = records(store, repository, commit)?
            .into_iter()
            .find(|record| record.id == id)
        else {
            return Ok(None);
        };
        let available = store
            .capture(&repository.common_dir, &snapshot)
            .and_then(|snapshot| snapshot.complete())
            .is_ok();
        Ok(available.then_some(snapshot))
    }

    fn record(&self, snapshot: &SnapshotRecord) -> Result<()> {
        let mut written = tempfile::NamedTempFile::new_in(&self.directory)?;
        written.write_all(snapshot.id.as_bytes())?;
        written.persist(self.directory.join(&self.key))?;
        Ok(())
    }

    fn clear_receipt(&self) -> Result<()> {
        match fs::remove_file(self.directory.join(&self.key)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

fn wait_for_lease(lock: &File, commit: &str, progress: &dyn Fn(String)) -> Result<bool> {
    let mut announced = false;
    loop {
        ensure!(
            !CANCELLED.load(Ordering::Acquire),
            "Snapshot preparation cancelled"
        );
        match lock.try_lock() {
            Ok(()) => return Ok(announced),
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
