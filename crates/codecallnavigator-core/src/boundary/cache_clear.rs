//! Explicitly discard only disposable inference decisions, under the analysis lock.
use super::cache_paths;
use crate::artifact::AnalysisLock;
use anyhow::{Context, Result, ensure};
use std::{fs, io::ErrorKind, path::Path};

/// Preserve graph artifacts, user configuration, navigation and build outputs.
/// Corrupt caches are removable too; all paths must be plain, single-link files.
pub fn clear_inference_cache(project: &Path) -> Result<u32> {
    let project = project
        .canonicalize()
        .context("Cannot open cache project")?;
    ensure!(project.is_dir(), "Cache project must be a directory");
    let path = cache_paths::path(&project)?;
    let parent = path.parent().context("Missing inference cache directory")?;
    match fs::symlink_metadata(parent) {
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error).context("Cannot inspect cache directory"),
        Ok(_) => cache_paths::validate(&path)?,
    }
    let _lock = AnalysisLock::acquire(&project)?;
    cache_paths::validate(&path)?;
    let mut removed = 0;
    for suffix in ["-wal", "-shm", "-journal", ""] {
        let mut name = path.as_os_str().to_os_string();
        name.push(suffix);
        match fs::remove_file(Path::new(&name)) {
            Ok(()) => removed += 1,
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "Cannot remove inference cache {}",
                        Path::new(&name).display()
                    )
                });
            }
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clears_corrupt_cache_and_sidecars_but_preserves_project_files() {
        let project = tempfile::tempdir().unwrap();
        let directory = crate::artifact::project_state_directory(project.path()).unwrap();
        assert_eq!(clear_inference_cache(project.path()).unwrap(), 0);
        fs::create_dir_all(&directory).unwrap();
        for suffix in ["", "-wal", "-shm", "-journal"] {
            fs::write(
                directory.join(format!("inference.sqlite3{suffix}")),
                "corrupt",
            )
            .unwrap();
        }
        let legacy = project.path().join(".codecallnavigator");
        fs::create_dir(&legacy).unwrap();
        for file in [
            "graph.json",
            "inference.sqlite3",
            "layers.yml",
            "boundaries.yml",
        ] {
            fs::write(legacy.join(file), file).unwrap();
        }
        assert_eq!(clear_inference_cache(project.path()).unwrap(), 4);
        assert_eq!(clear_inference_cache(project.path()).unwrap(), 0);
        for file in [
            "graph.json",
            "inference.sqlite3",
            "layers.yml",
            "boundaries.yml",
        ] {
            assert_eq!(fs::read_to_string(legacy.join(file)).unwrap(), file);
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn held_analysis_lock_prevents_clearing() {
        let project = tempfile::tempdir().unwrap();
        let lock = AnalysisLock::acquire(project.path()).unwrap();
        let cache = cache_paths::path(project.path()).unwrap();
        fs::write(&cache, "cache").unwrap();
        assert!(clear_inference_cache(project.path()).is_err());
        assert_eq!(fs::read_to_string(cache).unwrap(), "cache");
        drop(lock);
        fs::remove_dir_all(crate::artifact::project_state_directory(project.path()).unwrap())
            .unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn unsafe_paths_fail_before_any_cache_is_removed() {
        use std::os::unix::fs::symlink;
        let project = tempfile::tempdir().unwrap();
        let directory = crate::artifact::project_state_directory(project.path()).unwrap();
        fs::create_dir_all(&directory).unwrap();
        let cache = directory.join("inference.sqlite3");
        fs::write(&cache, "cache").unwrap();
        let outside = project.path().join("outside");
        fs::write(&outside, "preserved").unwrap();
        let sidecar = directory.join("inference.sqlite3-journal");
        symlink(&outside, &sidecar).unwrap();
        assert!(clear_inference_cache(project.path()).is_err());
        assert_eq!(fs::read_to_string(&cache).unwrap(), "cache");
        assert_eq!(fs::read_to_string(&outside).unwrap(), "preserved");
        fs::remove_file(&sidecar).unwrap();
        fs::hard_link(&outside, &sidecar).unwrap();
        assert!(clear_inference_cache(project.path()).is_err());
        fs::remove_file(&sidecar).unwrap();
        fs::remove_dir_all(&directory).unwrap();
        symlink(project.path(), &directory).unwrap();
        assert!(clear_inference_cache(project.path()).is_err());
        assert!(!project.path().join("graph.lock").exists());
        fs::remove_file(directory).unwrap();
    }
}
