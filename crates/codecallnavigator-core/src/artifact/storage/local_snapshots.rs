//! One local graph library for checkout generations and verified revisions.
mod catalog;
#[cfg(test)]
mod tests;

use super::{
    Captured, MissingGraph,
    captured::open_object,
    companions,
    publication::{atomic_write, check_cancel, write_object},
};
use crate::artifact::EncodedArtifact;
use anyhow::{Context, Result, ensure};
use catalog::{Entry, hex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env,
    fs::{self, File, OpenOptions, TryLockError},
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    thread,
    time::Duration,
};

const SNAPSHOT_BUDGET: u64 = 2 * 1024 * 1024 * 1024;
const LIBRARY_OVERRIDE: &str = "CODECALLNAVIGATOR_GRAPH_LIBRARY";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotIdentity {
    pub commit: String,
    pub project: String,
    pub analyzer: String,
    pub selection: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id: String,
    pub identity: SnapshotIdentity,
    pub digest: String,
}

pub struct Retention<'a> {
    pub identity: SnapshotIdentity,
    pub encoded: &'a EncodedArtifact,
}

pub struct WorkingPublication<'a> {
    pub encoded: &'a EncodedArtifact,
    pub metadata: &'a Value,
    pub verified: Option<(&'a Path, SnapshotIdentity)>,
}

pub struct VerifiedPublication<'a> {
    pub identity: SnapshotIdentity,
    pub encoded: &'a EncodedArtifact,
    pub metadata: &'a Value,
}

struct GraphWrite<'a> {
    entry: Entry,
    encoded: &'a EncodedArtifact,
    working_project: Option<&'a str>,
}

pub struct Store {
    root: PathBuf,
    budget: u64,
}

impl Store {
    pub fn system() -> Result<Self> {
        if let Some(value) = env::var_os(LIBRARY_OVERRIDE) {
            let path = PathBuf::from(value);
            ensure!(
                path.is_absolute(),
                "{LIBRARY_OVERRIDE} must be an absolute path"
            );
            return Ok(Self::at(path));
        }
        Ok(Self::at(user_data()?.join("CodeCallNavigator/graphs/v1")))
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

    pub fn repository_path(&self, common_dir: &Path) -> PathBuf {
        self.root.join("repos").join(path_key(common_dir))
    }

    pub fn snapshot_manifest_path(
        &self,
        common_dir: &Path,
        record: &SnapshotRecord,
    ) -> Result<PathBuf> {
        Ok(self
            .root
            .join(catalog::revision_path(&path_key(common_dir), record)?))
    }

    pub fn list(
        &self,
        common_dir: &Path,
        commit: &str,
        project: &str,
    ) -> Result<Vec<SnapshotRecord>> {
        let Some(_guard) = self.read_guard()? else {
            return Ok(Vec::new());
        };
        let db = catalog::open(&self.root, false)?;
        catalog::variants(&db, &path_key(common_dir), commit, project)
    }

    pub fn capture(&self, common_dir: &Path, record: &SnapshotRecord) -> Result<Captured> {
        let _guard = self
            .read_guard()?
            .context("Local graph library is missing")?;
        let db = catalog::open(&self.root, false)?;
        let entry = catalog::snapshot(&db, &path_key(common_dir), record)?;
        self.capture_entry(&entry, &record.id)
    }

    fn capture_entry(&self, entry: &Entry, id: &str) -> Result<Captured> {
        let path = entry.path(id)?;
        self.check_directories(&path, false)?;
        let mut bytes = Vec::new();
        open_object(&self.root.join(&path))?
            .take(entry.bytes().len() as u64 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes == entry.bytes(),
            "Saved graph manifest differs from the library catalog"
        );
        Captured::from_manifest(&self.root, bytes, entry.metadata.clone())
    }

    pub(crate) fn inspect_working(
        &self,
        project: &Path,
    ) -> Result<(PathBuf, Vec<u8>, Result<Captured>)> {
        let _guard = self.read_guard()?.ok_or(MissingGraph)?;
        let project = project
            .canonicalize()
            .context("Cannot open graph project")?;
        let db = catalog::open(&self.root, false)?;
        let (id, entry) = catalog::working(&db, &path_key(&project))?;
        let path = self.root.join(entry.path(&id)?);
        let captured = self.capture_entry(&entry, &id);
        Ok((path, entry.bytes().to_vec(), captured))
    }

    pub(crate) fn working_manifest_path(&self, project: &Path) -> Result<PathBuf> {
        let _guard = self.read_guard()?.ok_or(MissingGraph)?;
        let project = project
            .canonicalize()
            .context("Cannot open graph project")?;
        let db = catalog::open(&self.root, false)?;
        let (id, entry) = catalog::working(&db, &path_key(&project))?;
        Ok(self.root.join(entry.path(&id)?))
    }

    pub(crate) fn state_directory(&self, project: &Path) -> Result<PathBuf> {
        let project = project
            .canonicalize()
            .context("Cannot open graph project")?;
        let relative = Path::new("state").join(path_key(&project));
        let path = self.root.join(&relative);
        for directory in [&self.root, &self.root.join("state"), &path] {
            match real_directory(directory) {
                Ok(()) => (),
                Err(error) if missing(&error) => break,
                Err(error) => return Err(error),
            }
        }
        Ok(path)
    }

    pub(crate) fn ensure_state_directory(&self, project: &Path) -> Result<PathBuf> {
        let project = project
            .canonicalize()
            .context("Cannot open graph project")?;
        let relative = Path::new("state").join(path_key(&project));
        private_directory(&self.root)?;
        self.check_directories(&relative.join("state-file"), true)?;
        Ok(self.root.join(relative))
    }

    pub fn retain(
        &self,
        common_dir: &Path,
        retention: Retention<'_>,
        cancel: &AtomicBool,
    ) -> Result<SnapshotRecord> {
        let Retention { identity, encoded } = retention;
        self.retain_verified(
            common_dir,
            VerifiedPublication {
                identity,
                encoded,
                metadata: &Value::Null,
            },
            cancel,
        )
    }

    pub fn retain_verified(
        &self,
        common_dir: &Path,
        publication: VerifiedPublication<'_>,
        cancel: &AtomicBool,
    ) -> Result<SnapshotRecord> {
        let VerifiedPublication {
            identity,
            encoded,
            metadata,
        } = publication;
        let entry = Entry::snapshot(path_key(common_dir), identity, metadata, encoded.as_ref())?;
        let record = entry
            .snapshot_record()
            .context("Missing verified record")?
            .clone();
        self.publish(
            GraphWrite {
                working_project: None,
                entry,
                encoded,
            },
            cancel,
        )?;
        Ok(record)
    }

    pub(crate) fn publish_working(
        &self,
        project: &Path,
        publication: WorkingPublication<'_>,
        cancel: &AtomicBool,
    ) -> Result<Option<SnapshotRecord>> {
        let WorkingPublication {
            encoded,
            metadata,
            verified,
        } = publication;
        let project_key = path_key(project);
        let entry = match verified {
            Some((common_dir, identity)) => {
                Entry::snapshot(path_key(common_dir), identity, metadata, encoded.as_ref())?
            }
            None => Entry::working(
                path_key(checkout(project)),
                project_key.clone(),
                metadata,
                encoded.as_ref(),
            )?,
        };
        let record = entry.snapshot_record().cloned();
        self.publish(
            GraphWrite {
                working_project: Some(&project_key),
                entry,
                encoded,
            },
            cancel,
        )?;
        Ok(record)
    }

    fn publish(&self, write: GraphWrite<'_>, cancel: &AtomicBool) -> Result<()> {
        let GraphWrite {
            working_project,
            entry,
            encoded,
        } = write;
        check_cancel(cancel)?;
        let id = entry.id()?;
        let relative = entry.path(&id)?;
        private_directory(&self.root)?;
        let _guard = self.write_guard(cancel)?;
        let mut db = catalog::open(&self.root, true)?;
        self.check_directories(&relative, true)?;
        private_directory(&self.root.join("objects"))?;
        let tx = db.transaction()?;
        catalog::insert(&tx, &id, &entry, entry.snapshot_record().is_some())?;
        if let Some(project) = working_project {
            catalog::set_current(&tx, project, &id)?;
        }
        catalog::evict(&tx, &id, self.budget)?;
        self.write_bundle(&relative, encoded, cancel)?;
        check_cancel(cancel)?;
        tx.commit()?;
        // Publication has succeeded. Unreferenced files can safely be retried
        // at the next publication if housekeeping fails.
        if let Err(error) = catalog::prune(&self.root, &db) {
            eprintln!("Graph library cleanup deferred: {error:#}");
        }
        Ok(())
    }

    fn write_bundle(
        &self,
        relative: &Path,
        encoded: &EncodedArtifact,
        cancel: &AtomicBool,
    ) -> Result<()> {
        for (part, name, bytes) in [
            (
                &encoded.manifest.structure,
                "structure",
                encoded.structure.as_slice(),
            ),
            (&encoded.manifest.flows, "flows", encoded.flows.as_slice()),
        ] {
            check_cancel(cancel)?;
            ensure!(
                bytes.len() as u64 == part.compressed_bytes && hex(bytes) == part.digest,
                "Encoded graph object does not match its manifest"
            );
            write_object(&self.root, &part.path(name), bytes)?;
        }
        File::open(self.root.join("objects"))?.sync_all()?;
        companions::publish(&self.root, cancel)?;
        let manifest = self.root.join(relative);
        atomic_write(&manifest, encoded.as_ref())?;
        sync_parents(
            manifest.parent().context("Missing manifest directory")?,
            &self.root,
        )
    }

    fn check_directories(&self, relative_file: &Path, create: bool) -> Result<()> {
        real_directory(&self.root)?;
        let mut directory = self.root.clone();
        for component in relative_file
            .parent()
            .context("Missing graph directory")?
            .components()
        {
            directory.push(component);
            if create {
                private_directory(&directory)?;
            } else {
                real_directory(&directory)?;
            }
        }
        Ok(())
    }

    fn read_guard(&self) -> Result<Option<File>> {
        match real_directory(&self.root) {
            Ok(()) => (),
            Err(error) if missing(&error) => return Ok(None),
            Err(error) => return Err(error),
        }
        let file = match open_object(&self.root.join("library.lock")) {
            Ok(file) => file,
            Err(error) if missing(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        file.lock_shared()?;
        match fs::symlink_metadata(self.root.join("catalog.sqlite3")) {
            Ok(_) => Ok(Some(file)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn write_guard(&self, cancel: &AtomicBool) -> Result<File> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = options.open(self.root.join("library.lock"))?;
        ensure!(
            file.metadata()?.is_file(),
            "Graph library lock is not a regular file"
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
}

fn checkout(project: &Path) -> &Path {
    project
        .ancestors()
        .find(|directory| directory.join(".git").exists())
        .unwrap_or(project)
}

fn path_key(path: &Path) -> String {
    hex(path.as_os_str().as_encoded_bytes())
}

fn user_data() -> Result<PathBuf> {
    if cfg!(target_os = "windows") {
        return absolute_env("LOCALAPPDATA").context("Cannot locate LOCALAPPDATA");
    }
    let home = || absolute_env("HOME").context("Cannot locate home directory");
    if cfg!(target_os = "macos") {
        return Ok(home()?.join("Library/Application Support"));
    }
    match absolute_env("XDG_DATA_HOME") {
        Some(data) => Ok(data),
        None => Ok(home()?.join(".local/share")),
    }
}

fn absolute_env(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn real_directory(path: &Path) -> Result<()> {
    ensure!(
        fs::symlink_metadata(path)?.file_type().is_dir(),
        "Graph library location must be a real directory: {}",
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
    builder
        .create(path)
        .with_context(|| format!("Cannot create graph library directory {}", path.display()))?;
    real_directory(path)
}

fn missing(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound)
}

fn sync_parents(directory: &Path, root: &Path) -> Result<()> {
    for path in directory.ancestors() {
        File::open(path)?.sync_all()?;
        if path == root {
            break;
        }
    }
    Ok(())
}
