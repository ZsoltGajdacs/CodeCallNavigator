//! Filesystem primitives for durable immutable graph files.
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub(super) fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Analysis cancelled before graph publication"
    );
    Ok(())
}

pub(super) fn write_object(directory: &Path, path: &str, bytes: &[u8]) -> Result<()> {
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

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
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

fn persist(path: &Path, temporary: tempfile::NamedTempFile) -> Result<()> {
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

pub(super) fn owned_object(name: &str) -> bool {
    let Some((digest, suffix)) = name.split_once('.') else {
        return false;
    };
    digest.len() == 64
        && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        && matches!(suffix, "structure.json.zst" | "flows.json.zst")
}
