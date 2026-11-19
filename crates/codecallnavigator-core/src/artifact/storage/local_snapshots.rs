//! Local revision bundles shared by linked worktrees of one Git clone.
mod catalog;
#[cfg(test)]
mod tests;

use super::{
    Manifest,
    captured::open_object,
    publication::{atomic_write, write_object},
};
use crate::artifact::{Artifact, EncodedArtifact};
use anyhow::{Context, Result, ensure};
use catalog::{Catalog, Entry, InventoryItem, read_directory};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::{self, File, OpenOptions, TryLockError},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

const SNAPSHOT_BUDGET: u64 = 2 * 1024 * 1024 * 1024;
const CACHE_OVERRIDE: &str = "CODECALLNAVIGATOR_SNAPSHOT_CACHE";

/// The engine supplies revision and analysis information after validating the inputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotIdentity {
    pub commit: String,
    pub project: String,
    pub analyzer: String,
    pub selection: String,
}

/// Identical identity and bundle bytes always produce the same stable variant ID.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id: String,
    pub identity: SnapshotIdentity,
    pub digest: String,
}

/// Identity proven by the engine paired with the exact successfully published bytes.
pub struct Retention<'a> {
    pub identity: SnapshotIdentity,
    pub encoded: &'a EncodedArtifact,
}

pub struct Store {
    root: PathBuf,
    budget: u64,
}

impl Store {
    pub fn system() -> Result<Self> {
        if let Some(value) = env::var_os(CACHE_OVERRIDE) {
            let path = PathBuf::from(value);
            ensure!(
                path.is_absolute(),
                "{CACHE_OVERRIDE} must be an absolute path"
            );
            return Ok(Self::at(path));
        }
        Ok(Self::at(
            system_cache()?.join("CodeCallNavigator/snapshots/v1"),
        ))
    }

    pub fn at(root: PathBuf) -> Self {
        Self {
            root,
            budget: SNAPSHOT_BUDGET,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The caller supplies Git's canonical common directory, not a worktree path.
    pub fn repository_path(&self, common_dir: &Path) -> PathBuf {
        self.root.join("repos").join(
            blake3::hash(common_dir.as_os_str().as_encoded_bytes())
                .to_hex()
                .as_str(),
        )
    }

    pub fn list(
        &self,
        common_dir: &Path,
        commit: &str,
        project: &str,
    ) -> Result<Vec<SnapshotRecord>> {
        let Some(_guard) = read_guard(&self.root)? else {
            return Ok(Vec::new());
        };
        let mut records = catalog::entries(&self.repository_path(common_dir))?
            .into_iter()
            .map(|(_, entry)| entry.record)
            .filter(|record| record.identity.commit == commit && record.identity.project == project)
            .collect::<Vec<_>>();
        records.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(records)
    }

    pub fn capture(&self, common_dir: &Path, record: &SnapshotRecord) -> Result<CapturedSnapshot> {
        ensure!(catalog::digest(&record.id), "Invalid graph snapshot ID");
        let _guard = read_guard(&self.root)?.context("Local graph snapshot cache is missing")?;
        let repository = self.repository_path(common_dir);
        real_directory(&repository)?;
        real_directory(&self.root.join("objects"))?;
        let entry = Entry::read(&catalog::entry_path(&repository, &record.id))
            .context("Cannot open local graph snapshot")?;
        ensure!(
            entry.record == *record,
            "Local graph snapshot identity changed"
        );
        let manifest = entry.manifest()?;
        let structure = open_object(&self.root.join(manifest.structure.path("structure")))?;
        let flows = open_object(&self.root.join(manifest.flows.path("flows")))?;
        Ok(CapturedSnapshot {
            bytes: entry.manifest.into_bytes(),
            manifest,
            structure,
            flows,
        })
    }

    pub fn retain(
        &self,
        common_dir: &Path,
        retention: Retention<'_>,
        cancel: &AtomicBool,
    ) -> Result<SnapshotRecord> {
        let Retention { identity, encoded } = retention;
        check_cancel(cancel)?;
        let mut entry = Entry::new(identity, encoded)?;
        let repository = self.repository_path(common_dir);
        let path = catalog::entry_path(&repository, &entry.record.id);
        private_directory(&self.root)?;
        let _guard = write_guard(&self.root, cancel)?;
        for directory in [
            self.root.join("objects"),
            self.root.join("repos"),
            repository.clone(),
        ] {
            private_directory(&directory)?;
        }

        let mut catalog = Catalog::load(&self.root)?;
        entry.inserted = catalog.insertion(&path)?;
        let bytes = serde_json::to_vec(&entry)?;
        let record = entry.record.clone();
        let item = InventoryItem {
            entry,
            bytes: bytes.len() as u64,
        };
        let evictions = catalog.evict_to_fit(&path, &item, self.budget)?;
        // Reclaim files left by a previously interrupted write, even if this
        // attempt is later cancelled. This uses only the current committed index.
        catalog.prune(&self.root)?;
        check_cancel(cancel)?;
        self.write_objects(encoded)?;
        check_cancel(cancel)?;
        // Index publication makes the complete bundle visible. Finish eviction
        // while still holding the exclusive lock, including after cancellation.
        atomic_write(&path, &bytes)?;
        File::open(repository)?.sync_all()?;
        catalog.apply(path, item, evictions)?;
        catalog.prune(&self.root)?;
        Ok(record)
    }

    fn write_objects(&self, encoded: &EncodedArtifact) -> Result<()> {
        let manifest = &encoded.manifest;
        for (part, name, bytes) in [
            (
                &manifest.structure,
                "structure",
                encoded.structure.as_slice(),
            ),
            (&manifest.flows, "flows", encoded.flows.as_slice()),
        ] {
            ensure!(
                bytes.len() as u64 == part.compressed_bytes
                    && blake3::hash(bytes).to_hex().as_str() == part.digest,
                "Encoded graph snapshot object does not match its manifest"
            );
            write_object(&self.root, &part.path(name), bytes)?;
        }
        File::open(self.root.join("objects"))?.sync_all()?;
        Ok(())
    }
}

/// Open descriptors keep the selected bundle available after eviction.
pub struct CapturedSnapshot {
    bytes: Vec<u8>,
    manifest: Manifest,
    structure: File,
    flows: File,
}

impl CapturedSnapshot {
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn complete(self) -> Result<Artifact> {
        let mut parts = [self.structure, self.flows].into_iter();
        self.manifest
            .complete_from(|_| parts.next().context("Missing captured graph object"))
    }
}

fn system_cache() -> Result<PathBuf> {
    if cfg!(target_os = "windows") {
        return absolute_env("LOCALAPPDATA").context("Cannot locate LOCALAPPDATA");
    }
    let home = || absolute_env("HOME").context("Cannot locate home directory");
    if cfg!(target_os = "macos") {
        return Ok(home()?.join("Library/Caches"));
    }
    match absolute_env("XDG_CACHE_HOME") {
        Some(cache) => Ok(cache),
        None => Ok(home()?.join(".cache")),
    }
}

fn absolute_env(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(Ordering::Acquire),
        "Local graph snapshot retention cancelled"
    );
    Ok(())
}

fn real_directory(path: &Path) -> Result<()> {
    ensure!(
        fs::symlink_metadata(path)?.file_type().is_dir(),
        "Snapshot cache location must be a real directory: {}",
        path.display()
    );
    Ok(())
}

fn private_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    real_directory(path)
}

fn read_guard(root: &Path) -> Result<Option<File>> {
    if read_directory(root)?.is_none() {
        return Ok(None);
    }
    let file = match open_object(&root.join("cache.lock")) {
        Ok(file) => file,
        Err(error) if catalog::missing(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    file.lock_shared()?;
    if read_directory(&root.join("repos"))?.is_none() {
        return Ok(None);
    }
    Ok(Some(file))
}

fn write_guard(root: &Path, cancel: &AtomicBool) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(root.join("cache.lock"))?;
    ensure!(
        file.metadata()?.is_file(),
        "Snapshot cache lock is not a file"
    );
    loop {
        check_cancel(cancel)?;
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) => thread::sleep(Duration::from_millis(10)),
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
    }
}
