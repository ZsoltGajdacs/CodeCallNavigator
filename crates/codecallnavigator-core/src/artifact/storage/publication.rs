use super::{EncodedParts, Manifest};
use crate::artifact::GRAPH_PATH;
use anyhow::{Context, Result, ensure};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

const GUARD_NAME: &str = ".codecallnavigator/publication.lock";
const PREVIOUS: &str = ".codecallnavigator/previous.json";

/// Short-lived coordination only: never held during analysis or decompression.
pub(super) struct Guard(Option<File>);

impl Guard {
    pub(super) fn coordinated(&self) -> bool {
        self.0.is_some()
    }

    pub(super) fn exists(project: &Path) -> bool {
        project.join(GUARD_NAME).exists()
    }

    pub(super) fn read(project: &Path) -> Result<Self> {
        match File::open(project.join(GUARD_NAME)) {
            Ok(file) => {
                file.lock_shared()?;
                Ok(Self(Some(file)))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self(None)),
            Err(error) => Err(error).context("Cannot open publication guard"),
        }
    }

    fn write(project: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(project.join(GUARD_NAME))?;
        file.lock()?;
        Ok(Self(Some(file)))
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(file) = &self.0 {
            let _ = file.unlock();
        }
    }
}

pub(crate) fn publish(project: &Path, encoded: &EncodedParts, cancel: &AtomicBool) -> Result<()> {
    check_cancel(cancel)?;
    let directory = project.join(".codecallnavigator");
    ensure!(
        fs::symlink_metadata(&directory)?.file_type().is_dir(),
        "Graph configuration directory must be a real directory"
    );
    fs::create_dir_all(directory.join("objects"))?;
    validate_object_directory(&directory)?;
    write_object(
        &directory,
        &encoded.manifest.structure.path("structure"),
        &encoded.structure,
    )?;
    check_cancel(cancel)?;
    write_object(
        &directory,
        &encoded.manifest.flows.path("flows"),
        &encoded.flows,
    )?;
    File::open(directory.join("objects"))?.sync_all()?;
    let _guard = Guard::write(project)?;
    check_cancel(cancel)?;
    let previous = previous(project, encoded.as_ref())?;
    // All fallible cleanup happens before the manifest's commit point.
    prune(&directory, &encoded.manifest, previous.as_deref())?;
    if let Some(bytes) = &previous {
        atomic_write(&project.join(PREVIOUS), bytes)?;
    } else if project.join(PREVIOUS).exists() {
        fs::remove_file(project.join(PREVIOUS))
            .context("Cannot remove obsolete previous manifest")?;
    }
    let path = project.join(GRAPH_PATH);
    let prepared = prepare_write(&path, encoded.as_ref())?;
    commit_manifest(&path, prepared, cancel)?;
    File::open(&directory)?.sync_all()?;
    Ok(())
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Analysis cancelled before graph publication"
    );
    Ok(())
}

fn previous(project: &Path, current: &[u8]) -> Result<Option<Vec<u8>>> {
    match fs::read(project.join(GRAPH_PATH)) {
        Ok(bytes) if bytes == current => read_previous(project),
        Ok(bytes) => Ok(Manifest::from_bytes(&bytes).ok().map(|_| bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("Cannot preserve previous graph manifest"),
    }
}

fn read_previous(project: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(project.join(PREVIOUS)) {
        Ok(bytes) => Ok(Manifest::from_bytes(&bytes).ok().map(|_| bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("Cannot read previous graph manifest"),
    }
}

fn write_object(directory: &Path, path: &str, bytes: &[u8]) -> Result<()> {
    let path = directory.join(path);
    let reader = match super::captured::open_object(&path) {
        Ok(file) => file,
        Err(error) if missing(&error) => return atomic_write(&path, bytes),
        Err(error) => return Err(error).context("Cannot inspect existing graph object"),
    };
    let mut existing = Vec::new();
    reader
        .take(bytes.len() as u64 + 1)
        .read_to_end(&mut existing)?;
    if existing == bytes {
        return Ok(());
    }
    // Repair only this reserved object entry; readers holding its old inode
    // remain safe and may reopen the repaired bytes using their same manifest.
    atomic_write(&path, bytes)
}

fn missing(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound)
}

pub(super) fn validate_object_directory(directory: &Path) -> Result<()> {
    for path in [directory.to_owned(), directory.join("objects")] {
        ensure!(
            fs::symlink_metadata(&path)?.file_type().is_dir(),
            "Graph object directory must be a real directory: {}",
            path.display()
        );
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    persist(path, prepare_write(path, bytes)?)
}

fn prepare_write(path: &Path, bytes: &[u8]) -> Result<tempfile::NamedTempFile> {
    let directory = path.parent().context("Missing publication directory")?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".graph-publish-")
        .tempfile_in(directory)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    Ok(temporary)
}

fn commit_manifest(
    path: &Path,
    temporary: tempfile::NamedTempFile,
    cancel: &AtomicBool,
) -> Result<()> {
    // This is the publication commit point. Cancellation during pruning,
    // sidecar updates or temporary-file preparation must preserve graph.json.
    check_cancel(cancel)?;
    persist(path, temporary)
}

fn persist(path: &Path, temporary: tempfile::NamedTempFile) -> Result<()> {
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn prune(directory: &Path, current: &Manifest, previous: Option<&[u8]>) -> Result<()> {
    let mut retained = HashSet::new();
    retain(&mut retained, current);
    if let Some(previous) = previous {
        retain(&mut retained, &Manifest::from_bytes(previous)?);
    }
    for entry in fs::read_dir(directory.join("objects"))? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !obsolete(name, &retained) {
            continue;
        }
        if !entry.file_type()?.is_file() {
            continue;
        }
        fs::remove_file(entry.path()).context("Cannot prune obsolete graph object")?;
    }
    Ok(())
}

fn obsolete(name: &str, retained: &HashSet<String>) -> bool {
    owned_object(name) && !retained.contains(name)
}

fn retain(retained: &mut HashSet<String>, manifest: &Manifest) {
    for path in [
        manifest.structure.path("structure"),
        manifest.flows.path("flows"),
    ] {
        retained.insert(path.trim_start_matches("objects/").into());
    }
}

fn owned_object(name: &str) -> bool {
    let Some((digest, suffix)) = name.split_once('.') else {
        return false;
    };
    digest.len() == 64
        && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        && matches!(suffix, "structure.json.zst" | "flows.json.zst")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Only a real temporary file can prove that cancellation after flushing
    // prepared bytes leaves the active manifest intact and removes the temp.
    #[test]
    fn cancellation_after_preparation_preserves_active_manifest() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("graph.json");
        fs::write(&path, b"previous successful generation").unwrap();
        let cancel = AtomicBool::new(false);
        let temporary = prepare_write(&path, b"replacement generation").unwrap();
        let temporary_path = temporary.path().to_owned();
        assert!(temporary_path.exists());
        cancel.store(true, Ordering::Release);
        assert!(commit_manifest(&path, temporary, &cancel).is_err());
        assert_eq!(fs::read(path).unwrap(), b"previous successful generation");
        assert!(!temporary_path.exists());
    }
}
