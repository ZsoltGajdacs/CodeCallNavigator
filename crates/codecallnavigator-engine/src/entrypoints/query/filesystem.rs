//! Explicit live filesystem access, separate from analyzed generation facts.
use super::Loaded;
use codecallnavigator_core::query::{Error, Result};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

fn confined(roots: &[(String, PathBuf)], path: &Path) -> Result<PathBuf> {
    let path = path
        .canonicalize()
        .map_err(|e| Error::new("path_unavailable", e.to_string()))?;
    if !roots
        .iter()
        .any(|(_, root)| root.canonicalize().is_ok_and(|root| path.starts_with(root)))
    {
        return Err(Error::new(
            "outside_project",
            "Live filesystem access requires a configured project root",
        ));
    }
    Ok(path)
}

pub(super) fn source(loaded: &Loaded, symbol: &str) -> Result<Value> {
    let graph = loaded.graph()?;
    let index = graph
        .node_index(symbol)
        .ok_or_else(|| Error::new("symbol_missing", "No symbol in this graph generation"))?;
    let node = &graph.data.nodes[index as usize];
    let mut result = source_file(&loaded.roots, Path::new(&node.file))?;
    result["symbol"] = json!(symbol);
    result["line"] = json!(node.line);
    Ok(result)
}

pub(super) fn source_file(roots: &[(String, PathBuf)], path: &Path) -> Result<Value> {
    let path = confined(roots, path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options
        .open(&path)
        .map_err(|e| Error::new("source_unavailable", e.to_string()))?;
    if !file
        .metadata()
        .map_err(|e| Error::new("source_unavailable", e.to_string()))?
        .is_file()
    {
        return Err(Error::new(
            "source_unavailable",
            "Source is not a regular file",
        ));
    }
    let mut bytes = Vec::new();
    file.take(codecallnavigator_core::query::MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::new("source_unavailable", e.to_string()))?;
    if bytes.len() as u64 > codecallnavigator_core::query::MAX_SOURCE_BYTES {
        return Err(Error::new(
            "source_too_large",
            "Source exceeds 16 MiB preview limit",
        ));
    }
    let content = String::from_utf8(bytes)
        .map_err(|_| Error::new("source_encoding", "Source is not UTF-8"))?
        .replace("\r\n", "\n");
    Ok(json!({"origin":"live","file":path,"content":content}))
}

pub(super) fn directory(
    roots: &[(String, PathBuf)],
    path: &Path,
    offset: usize,
    limit: usize,
) -> Result<Value> {
    if limit == 0 || limit > 10000 {
        return Err(Error::new(
            "invalid_request",
            "Directory limit must be 1..=10000",
        ));
    }
    let path = confined(roots, path)?;
    let mut entries = Vec::new();
    for entry in
        fs::read_dir(&path).map_err(|e| Error::new("directory_unavailable", e.to_string()))?
    {
        if crate::entrypoints::cancellation::CANCELLED.load(Ordering::Relaxed) {
            return Err(Error::new("cancelled", "Directory query cancelled"));
        }
        let entry = entry.map_err(|e| Error::new("directory_unavailable", e.to_string()))?;
        let kind = entry
            .file_type()
            .map_err(|e| Error::new("directory_unavailable", e.to_string()))?;
        let kind = if kind.is_symlink() {
            "symlink"
        } else if kind.is_dir() {
            "directory"
        } else {
            "file"
        };
        entries.push((
            kind != "directory",
            entry.file_name().to_string_lossy().into_owned(),
            kind,
            entry.path(),
        ));
    }
    entries.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    let total = entries.len();
    let entries = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(_, label, kind, path)| json!({"label":label,"kind":kind,"path":path}))
        .collect::<Vec<_>>();
    Ok(
        json!({"origin":"live","path":path,"more":total.saturating_sub(offset)>limit,"total":total,"offset":offset,"limit":limit,"entries":entries}),
    )
}
