//! Read checkout entries relative to the selected project without following links.
use anyhow::{Context, Result, ensure};
use std::{fs::File, path::Path};

pub(super) enum Entry {
    Regular { file: File, executable: bool },
    Link(Vec<u8>),
    Directory,
    Special,
}

pub(super) fn read(project: &Path, path: &[u8]) -> Result<Option<Entry>> {
    let parts: Vec<_> = path.split(|&byte| byte == b'/').collect();
    ensure!(
        parts
            .iter()
            .all(|part| !matches!(*part, b"" | b"." | b"..") && !part.contains(&0)),
        "Invalid path in Git diff"
    );
    platform::read_entry(project, &parts).with_context(|| {
        format!(
            "Cannot capture worktree path {:?}",
            String::from_utf8_lossy(path)
        )
    })
}

#[cfg(unix)]
mod platform {
    use super::{Context, Entry, File, Path, Result};
    use std::{
        ffi::{CStr, CString},
        io,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::PermissionsExt,
        },
    };

    pub(super) fn read_entry(project: &Path, parts: &[&[u8]]) -> Result<Option<Entry>> {
        let (name, parents) = parts.split_last().context("Git path has no leaf")?;
        let Some(parent) = open_parent(File::open(project)?, parents)? else {
            return Ok(None);
        };
        read_leaf(&parent, &CString::new(*name)?)
    }

    fn open_parent(mut parent: File, parts: &[&[u8]]) -> Result<Option<File>> {
        for component in parts {
            match open(&parent, &CString::new(*component)?, libc::O_DIRECTORY) {
                Ok(directory) => parent = directory,
                Err(error)
                    if matches!(
                        error.raw_os_error(),
                        Some(libc::ENOENT | libc::ENOTDIR | libc::ELOOP)
                    ) =>
                {
                    return Ok(None);
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(Some(parent))
    }

    fn read_leaf(parent: &File, name: &CStr) -> Result<Option<Entry>> {
        match open(parent, name, 0) {
            Ok(file) => classify(file).map(Some),
            Err(error) if error.raw_os_error() == Some(libc::ELOOP) => {
                Ok(Some(Entry::Link(link(parent, name)?)))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) if error.raw_os_error() == Some(libc::ENXIO) => Ok(Some(Entry::Special)),
            Err(error) => Err(error.into()),
        }
    }

    fn classify(file: File) -> Result<Entry> {
        let metadata = file.metadata()?;
        Ok(if metadata.is_file() {
            let executable = metadata.permissions().mode() & 0o100 != 0;
            Entry::Regular { file, executable }
        } else if metadata.is_dir() {
            Entry::Directory
        } else {
            Entry::Special
        })
    }

    fn open(parent: &File, name: &CStr, flags: libc::c_int) -> io::Result<File> {
        // Every component is a single name under a directory already opened
        // without following links. O_NONBLOCK protects against raced FIFOs.
        let descriptor = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                flags | libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }

    fn link(parent: &File, name: &CStr) -> io::Result<Vec<u8>> {
        let mut bytes = vec![0_u8; libc::PATH_MAX as usize + 1];
        let size = unsafe {
            libc::readlinkat(
                parent.as_raw_fd(),
                name.as_ptr(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
            )
        };
        if size < 0 {
            return Err(io::Error::last_os_error());
        }
        if size as usize == bytes.len() {
            return Err(io::Error::other(
                "Symbolic link target exceeds the path limit",
            ));
        }
        bytes.truncate(size as usize);
        Ok(bytes)
    }
}

#[cfg(not(unix))]
mod platform {
    use super::{Context, Entry, File, Path, Result, ensure};
    use std::{fs, io};

    pub(super) fn read_entry(project: &Path, parts: &[&[u8]]) -> Result<Option<Entry>> {
        let (name, parents) = parts.split_last().context("Git path has no leaf")?;
        let mut path = project.to_owned();
        for component in parents {
            path.push(component_name(component)?);
            if !metadata(&path)?.is_some_and(|metadata| metadata.is_dir()) {
                return Ok(None);
            }
        }
        path.push(component_name(name)?);
        metadata(&path)?
            .map(|metadata| classify(&path, &metadata))
            .transpose()
    }

    fn component_name(component: &[u8]) -> Result<&str> {
        let component = std::str::from_utf8(component)?;
        ensure!(
            !component.contains(['\\', ':']),
            "Invalid worktree path component"
        );
        Ok(component)
    }

    fn metadata(path: &Path) -> Result<Option<fs::Metadata>> {
        match fs::symlink_metadata(path) {
            Ok(metadata) => Ok(Some(metadata)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn classify(path: &Path, metadata: &fs::Metadata) -> Result<Entry> {
        Ok(if metadata.is_symlink() {
            Entry::Link(fs::read_link(path)?.as_os_str().as_encoded_bytes().to_vec())
        } else if metadata.is_file() {
            Entry::Regular {
                file: File::open(path)?,
                executable: false,
            }
        } else if metadata.is_dir() {
            Entry::Directory
        } else {
            Entry::Special
        })
    }
}

/// Identify the observed regular file when a complete content hash is unavailable.
#[cfg(unix)]
pub(super) fn metadata_stamp(metadata: &std::fs::Metadata) -> blake3::Hash {
    use std::os::unix::fs::MetadataExt;

    let mut digest = blake3::Hasher::new();
    for value in [
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        u64::from(metadata.mode()),
    ] {
        digest.update(&value.to_le_bytes());
    }
    for value in [
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    ] {
        digest.update(&value.to_le_bytes());
    }
    digest.finalize()
}

#[cfg(not(unix))]
pub(super) fn metadata_stamp(metadata: &std::fs::Metadata) -> blake3::Hash {
    use std::time::UNIX_EPOCH;

    let mut digest = blake3::Hasher::new();
    digest.update(&metadata.len().to_le_bytes());
    digest.update(&[u8::from(metadata.permissions().readonly())]);
    for result in [metadata.modified(), metadata.created()] {
        match result.map(|time| time.duration_since(UNIX_EPOCH)) {
            Ok(Ok(duration)) => {
                digest.update(&[1]);
                digest.update(&duration.as_nanos().to_le_bytes());
            }
            Ok(Err(error)) => {
                digest.update(&[2]);
                digest.update(&error.duration().as_nanos().to_le_bytes());
            }
            Err(_) => {
                digest.update(&[0]);
            }
        }
    }
    digest.finalize()
}
