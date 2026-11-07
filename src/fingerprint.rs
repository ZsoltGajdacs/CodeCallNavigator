use crate::{discovery::excluded_directory, model::FileFingerprint};
use anyhow::{Context, Result};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub fn capture(paths: impl IntoIterator<Item = PathBuf>) -> Result<Vec<FileFingerprint>> {
    paths
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|path| fingerprint(&path))
        .collect()
}

pub fn is_stale(items: &[FileFingerprint]) -> Result<bool> {
    for item in items {
        let path = Path::new(&item.path);
        if !path.exists() {
            return Ok(true);
        }
        let current = fingerprint(path)?;
        if (current.size, current.modified_ns, &current.hash)
            != (item.size, item.modified_ns, &item.hash)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn source_paths(roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut paths = BTreeSet::new();
    for root in roots {
        if root.is_dir() {
            collect_sources(root, &mut paths)?;
        } else if root.is_file() {
            paths.insert(root.clone());
        }
    }
    Ok(paths.into_iter().collect())
}

fn collect_sources(root: &Path, paths: &mut BTreeSet<PathBuf>) -> Result<()> {
    paths.insert(root.into());
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if kind.is_dir() && !excluded_directory(&name) {
            collect_sources(&entry.path(), paths)?;
        } else if kind.is_file() && relevant_file(&entry.path()) {
            paths.insert(entry.path());
        }
    }
    Ok(())
}

fn relevant_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    matches!(
        extension,
        "cs" | "java" | "csproj" | "sln" | "slnx" | "props" | "targets" | "gradle" | "kts"
    ) || matches!(
        name.as_str(),
        "global.json"
            | "nuget.config"
            | "lombok.config"
            | "gradle.properties"
            | "gradle-wrapper.properties"
            | "gradle-wrapper.jar"
    )
}

fn fingerprint(path: &Path) -> Result<FileFingerprint> {
    let metadata =
        fs::metadata(path).with_context(|| format!("Cannot fingerprint {}", path.display()))?;
    let modified_ns = if metadata.is_dir() {
        0
    } else {
        metadata
            .modified()?
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .min(u64::MAX as u128) as u64
    };
    let hash = if metadata.is_dir() {
        directory_hash(path)?
    } else {
        file_hash(path)?
    };
    Ok(FileFingerprint {
        path: path.to_string_lossy().into_owned(),
        size: if metadata.is_dir() { 0 } else { metadata.len() },
        modified_ns,
        hash,
    })
}

fn directory_hash(path: &Path) -> Result<String> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !excluded_directory(&name) {
            names.insert(name);
        }
    }
    Ok(
        blake3::hash(names.into_iter().collect::<Vec<_>>().join("\0").as_bytes())
            .to_hex()
            .to_string(),
    )
}

fn file_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_edits_additions_and_deletions() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("App.java");
        fs::write(&file, "class App {}").unwrap();
        let capture_now = || capture(source_paths(&[directory.path().into()]).unwrap()).unwrap();
        let original = capture_now();
        assert!(!is_stale(&original).unwrap());
        fs::write(&file, "class New {}").unwrap();
        assert!(is_stale(&original).unwrap());
        let before_addition = capture_now();
        fs::write(directory.path().join("New.java"), "class New {}").unwrap();
        assert!(is_stale(&before_addition).unwrap());
        let before_delete = capture_now();
        fs::remove_file(&file).unwrap();
        assert!(is_stale(&before_delete).unwrap());
    }

    #[test]
    fn creating_ignored_build_outputs_does_not_stale_sources() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("App.java"), "class App {}").unwrap();
        let before = capture(source_paths(&[directory.path().into()]).unwrap()).unwrap();
        fs::create_dir(directory.path().join("build")).unwrap();
        fs::write(directory.path().join("build/output.class"), "compiled").unwrap();
        assert!(!is_stale(&before).unwrap());
    }
}
