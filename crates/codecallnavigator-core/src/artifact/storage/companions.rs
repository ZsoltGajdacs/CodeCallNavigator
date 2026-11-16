//! Distribution contracts accompany publication without affecting graph identity.
use super::{captured::open_object, publication};
use anyhow::{Context, Result, ensure};
use std::{fs, io::Read, path::Path, sync::atomic::AtomicBool};

pub(super) const FILES: [(&str, &[u8]); 3] = [
    (
        "graph.manifest.schema.json",
        include_bytes!("../../../../../docs/graph.manifest.schema.json"),
    ),
    (
        "graph.schema.json",
        include_bytes!("../../../../../docs/graph.schema.json"),
    ),
    (
        "README.md",
        include_bytes!("../../../../../docs/graph-bundle.md"),
    ),
];

pub(super) fn publish(directory: &Path, cancel: &AtomicBool) -> Result<()> {
    for (name, bytes) in FILES {
        publication::check_cancel(cancel)?;
        let path = directory.join(name);
        replace_if_changed(&path, bytes)
            .with_context(|| format!("Cannot publish graph companion {}", path.display()))?;
    }
    Ok(())
}

fn replace_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(metadata.is_file(), "Graph companion must be a regular file");
            // Use the guarded object opener to reject raced links and avoid
            // blocking if a regular entry is substituted with a pipe.
            let file = open_object(path)?;
            let mut existing = Vec::new();
            file.take(bytes.len() as u64 + 1)
                .read_to_end(&mut existing)?;
            if existing == bytes {
                return Ok(());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    publication::atomic_write(path, bytes)
}
