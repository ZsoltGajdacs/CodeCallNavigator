//! Filesystem enumeration belongs off the UI thread; directory links are leaves.
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Directory,
    File,
    Symlink,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: String,
    pub label: String,
    pub kind: Kind,
}

pub fn read_fixture(path: &str, cancelled: &AtomicBool) -> Result<Vec<Entry>, String> {
    let entries = fs::read_dir(path).map_err(|error| format!("Could not read {path}: {error}"))?;
    let mut result = Vec::new();
    for entry in entries {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        let entry = entry.map_err(|error| format!("Could not read an entry in {path}: {error}"))?;
        let kind = entry
            .file_type()
            .map_err(|error| format!("Could not inspect {}: {error}", entry.path().display()))?;
        result.push(Entry {
            path: crate::source_path::identity(&entry.path().to_string_lossy()),
            label: entry.file_name().to_string_lossy().into_owned(),
            kind: if kind.is_symlink() {
                Kind::Symlink
            } else if kind.is_dir() {
                Kind::Directory
            } else {
                Kind::File
            },
        });
    }
    result.sort_by(|a, b| {
        (a.kind != Kind::Directory)
            .cmp(&(b.kind != Kind::Directory))
            .then_with(|| a.label.cmp(&b.label))
    });
    Ok(result)
}

pub fn contains(root: &str, path: &str) -> bool {
    Path::new(path).strip_prefix(root).is_ok()
}
