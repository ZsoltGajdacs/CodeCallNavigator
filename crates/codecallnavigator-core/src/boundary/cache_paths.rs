//! SQLite may touch sidecars, so validate all owned paths before opening it.
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};
pub(super) fn validate(path: &Path) -> Result<()> {
    regular_or_missing(path)?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        regular_or_missing(&PathBuf::from(sidecar))?;
    }
    Ok(())
}
fn regular_or_missing(path: &Path) -> Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    ensure!(
        metadata.file_type().is_file(),
        "Inference cache path is not a regular file: {}",
        path.display()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        ensure!(
            metadata.nlink() == 1,
            "Inference cache path is hard-linked: {}",
            path.display()
        );
    }
    Ok(())
}

/// Immutable reads cannot create WAL/SHM files before graph publication.
pub(super) fn readonly_uri(path: &Path) -> Result<String> {
    let path = std::path::absolute(path)?;
    let text = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Cache path is not UTF-8"))?;
    let encoded = text.bytes().map(uri_byte).collect::<String>();
    Ok(format!("file:{encoded}?immutable=1"))
}
fn uri_byte(byte: u8) -> String {
    if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'.' | b'_' | b'~') {
        (byte as char).to_string()
    } else {
        format!("%{byte:02X}")
    }
}
