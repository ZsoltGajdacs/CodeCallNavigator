//! Desktop executable/resource discovery, independent of analyzer implementation.
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Runtime {
    pub engine: PathBuf,
    pub resources: PathBuf,
    pub dot: PathBuf,
}

impl Runtime {
    pub fn discover() -> Result<Self> {
        let executable = std::env::current_exe()?;
        let directory = executable.parent().context("Executable has no directory")?;
        let engine = engine_path(directory)?;
        let bundled = directory.parent().unwrap_or(directory).join("Resources");
        let resources = std::env::var_os("CODEGRAPH_RESOURCES")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                if bundled.join("analyzers").is_dir() {
                    bundled
                } else {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                }
            });
        ensure!(
            resources.is_dir(),
            "Analyzer resources are missing: {}",
            resources.display()
        );
        let bundled_dot = resources.join("analyzers/graphviz/bin/dot");
        let dot = if bundled_dot.is_file() {
            bundled_dot
        } else {
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .chain([
                    PathBuf::from("/opt/homebrew/bin"),
                    PathBuf::from("/usr/local/bin"),
                    PathBuf::from("/usr/bin"),
                ])
                .map(|directory| directory.join("dot"))
                .find(|path| path.is_file())
                .unwrap_or_default()
        };
        Ok(Self {
            engine,
            resources,
            dot,
        })
    }
}

fn engine_path(directory: &Path) -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("CODEGRAPH_ENGINE") {
        let path = PathBuf::from(path);
        ensure!(
            path.is_file(),
            "CODEGRAPH_ENGINE is not a file: {}",
            path.display()
        );
        return path.canonicalize().context("Cannot open configured engine");
    }
    let sibling = directory.join("codegraph-engine");
    let development = directory
        .parent()
        .unwrap_or(directory)
        .join("codegraph-engine");
    [sibling, development]
        .into_iter()
        .find(|path| path.is_file())
        .context(
            "Cannot find codegraph-engine beside CodeGraph; build or install the matching engine",
        )
}
