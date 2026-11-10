//! Desktop runtime adds layout tooling to the standalone analyzer resources.
use anyhow::Result;
use codegraph::analyzers::AnalyzerRuntime;
use std::{ops::Deref, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Runtime {
    analyzer: AnalyzerRuntime,
    pub dot: PathBuf,
}

impl Runtime {
    pub fn discover() -> Result<Self> {
        let analyzer = AnalyzerRuntime::discover()?;
        let bundled = analyzer.bundle_root.join("analyzers/graphviz/bin/dot");
        let dot = if bundled.is_file() {
            bundled
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
        Ok(Self { analyzer, dot })
    }
}

impl Deref for Runtime {
    type Target = AnalyzerRuntime;
    fn deref(&self) -> &Self::Target {
        &self.analyzer
    }
}
