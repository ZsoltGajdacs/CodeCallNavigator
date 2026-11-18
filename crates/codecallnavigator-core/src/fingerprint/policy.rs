//! Saved input-selection semantics; legacy artifacts keep their original hashes.
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputPolicy {
    #[default]
    Legacy,
    ScopedV1,
}

/// Broad roots discover incidental children; required roots are selected or declared
/// by a compiler/build and may deliberately enter hidden or linked directories.
pub enum ScanRoot {
    Broad(PathBuf),
    Required(PathBuf),
}
impl ScanRoot {
    pub fn path(&self) -> &Path {
        match self {
            Self::Broad(path) | Self::Required(path) => path,
        }
    }
    pub(super) fn eligible(&self) -> Result<bool> {
        match self {
            Self::Required(_) => Ok(true),
            Self::Broad(path) => match fs::symlink_metadata(path) {
                Ok(metadata) => Ok(!incidental(metadata.file_type(), || {
                    path.file_name().is_some_and(hidden_name)
                })),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
                Err(error) => Err(error)
                    .with_context(|| format!("Cannot inspect scan root {}", path.display())),
            },
        }
    }
}
impl InputPolicy {
    pub(crate) fn excludes(self, entry: &fs::DirEntry) -> Result<bool> {
        if self == Self::Legacy {
            return Ok(false);
        }
        Ok(incidental(entry.file_type()?, || {
            hidden_name(&entry.file_name())
        }))
    }

    /// Include every lexical ancestor link, so retargeting a required directory
    /// also invalidates its explicit descendants. Only digests enter provenance.
    pub(super) fn bind_links(self, path: &Path, hash: String) -> Result<String> {
        if self == Self::Legacy {
            return Ok(hash);
        }
        let links = required_link_bindings(path)?;
        if links.is_empty() {
            return Ok(hash);
        }
        let mut digest = blake3::Hasher::new();
        digest.update(hash.as_bytes());
        for link in links {
            let bytes = link.target.as_os_str().as_encoded_bytes();
            digest.update(&(bytes.len() as u64).to_le_bytes());
            digest.update(bytes);
        }
        Ok(digest.finalize().to_hex().to_string())
    }
}

fn incidental(kind: fs::FileType, hidden: impl FnOnce() -> bool) -> bool {
    kind.is_symlink() || (kind.is_dir() && hidden())
}

fn hidden_name(name: &OsStr) -> bool {
    name.as_encoded_bytes().starts_with(b".")
}

/// One lexical link used to resolve a required input, including indirect targets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkBinding {
    pub path: PathBuf,
    pub target: PathBuf,
}

/// Observe lexical ancestors and their target chains without discarding aliases.
/// Broken/cyclic links fail with path context; ordinary missing inputs are allowed.
pub fn required_link_bindings(path: &Path) -> Result<Vec<LinkBinding>> {
    let mut links = Vec::new();
    let mut pending = path.ancestors().map(Path::to_path_buf).collect::<Vec<_>>();
    let mut visited = BTreeSet::new();
    while let Some(path) = pending.pop() {
        if !visited.insert(path.clone()) {
            continue;
        }
        let Some(target) = required_link_target(&path)? else {
            continue;
        };
        let resolved = resolved_link(&path, &target);
        links.push(LinkBinding { path, target });
        pending.extend(resolved.ancestors().map(Path::to_path_buf));
    }
    Ok(links)
}

fn resolved_link(link: &Path, target: &Path) -> PathBuf {
    if target.is_absolute() {
        target.to_path_buf()
    } else {
        link.parent().unwrap_or(Path::new("")).join(target)
    }
}

fn required_link_target(path: &Path) -> Result<Option<PathBuf>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_symlink() => {
            fs::metadata(path)
                .with_context(|| format!("Cannot resolve required link {}", path.display()))?;
            fs::read_link(path)
                .map(Some)
                .with_context(|| format!("Cannot read required link {}", path.display()))
        }
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("Cannot inspect required input {}", path.display()))
        }
    }
}
