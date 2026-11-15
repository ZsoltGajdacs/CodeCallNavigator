//! Expected immutable identities supplied by the real-CLI fixture helper.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Configuration {
    pub version: u32,
    pub project: PathBuf,
    pub root_symbol: String,
    pub helper_symbol: String,
    pub recursive_symbol: String,
    pub sources: Vec<Source>,
    pub generation: String,
    pub expected_viewport: [u32; 2],
    pub timeout_seconds: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Source {
    pub symbol: String,
    pub path: String,
    pub content: String,
    pub sha256: String,
}
impl Configuration {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut config: Self =
            serde_json::from_slice(bytes).context("Invalid expansion probe configuration")?;
        config.validate()?;
        config.project = config
            .project
            .canonicalize()
            .context("Cannot open expansion fixture project")?;
        ensure!(
            config.project.is_dir(),
            "Expansion fixture project is not a directory"
        );
        Ok(config)
    }
    fn validate(&self) -> Result<()> {
        ensure!(self.version == 3, "Unsupported expansion probe version");
        ensure!(
            self.project.is_absolute(),
            "Expansion project must be absolute"
        );
        ensure!(
            self.expected_viewport == [1050, 680],
            "Expansion acceptance requires minimum viewport"
        );
        ensure!(
            (30..=300).contains(&self.timeout_seconds),
            "Expansion timeout must be 30..300 seconds"
        );
        ensure!(hex(&self.generation), "Invalid expansion graph generation");
        ensure!(
            !self.root_symbol.is_empty() && !self.helper_symbol.is_empty(),
            "Expansion symbols are required"
        );
        ensure!(
            self.root_symbol != self.helper_symbol && self.recursive_symbol == self.helper_symbol,
            "Expected distinct root and self-recursive helper"
        );
        ensure!(
            self.sources.len() == 2,
            "Expansion fixture requires two source owners"
        );
        for symbol in [&self.root_symbol, &self.helper_symbol] {
            ensure!(
                self.sources
                    .iter()
                    .filter(|source| source.symbol == *symbol)
                    .count()
                    == 1,
                "Expected one exact source for each callable"
            );
        }
        for source in &self.sources {
            ensure!(
                hex(&source.sha256) && !source.content.is_empty(),
                "Invalid expected source snapshot"
            );
            let path = Path::new(&source.path);
            ensure!(
                path.is_absolute()
                    && path.starts_with(&self.project)
                    && path
                        .components()
                        .all(|part| !matches!(part, std::path::Component::ParentDir)),
                "Source owner path must belong to the owned fixture project"
            );
        }
        ensure!(
            self.sources[0].path != self.sources[1].path,
            "Expansion must exercise two files"
        );
        Ok(())
    }
    pub fn source(&self, symbol: &str) -> Result<&Source> {
        self.sources
            .iter()
            .find(|source| source.symbol == symbol)
            .context("Probe has no source for this symbol")
    }
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }
    pub fn viewport_matches(&self, actual: [f32; 2]) -> bool {
        actual
            .into_iter()
            .zip(self.expected_viewport)
            .all(|(actual, expected)| (actual - expected as f32).abs() <= 1.0)
    }
}
fn hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn configuration() -> Configuration {
        serde_json::from_value(serde_json::json!({
            "version":3,"project":"/owned/project","root_symbol":"Root.Run","helper_symbol":"Helpers.Step","recursive_symbol":"Helpers.Step",
            "generation":"a".repeat(64),"expected_viewport":[1050,680],"timeout_seconds":120,
            "sources":[{"symbol":"Root.Run","path":"/owned/project/Root.cs","content":"root","sha256":"b".repeat(64)},
            {"symbol":"Helpers.Step","path":"/owned/project/Helpers.cs","content":"helper","sha256":"c".repeat(64)}]
        })).unwrap()
    }
    #[test]
    fn requires_distinct_source_owners_and_exact_recursive_target() {
        let mut config = configuration();
        assert!(config.validate().is_ok());
        config.recursive_symbol = config.root_symbol.clone();
        assert!(config.validate().is_err());
        config.recursive_symbol = config.helper_symbol.clone();
        config.sources[1].path = config.sources[0].path.clone();
        assert!(config.validate().is_err());
    }
    #[test]
    fn rejects_unowned_source_paths_and_unbounded_native_setup() {
        let mut config = configuration();
        config.sources[0].path = "/owned/project/../outside.cs".into();
        assert!(config.validate().is_err());
        config.sources[0].path = "/other/Root.cs".into();
        assert!(config.validate().is_err());
        config.sources[0].path = "/owned/project/Root.cs".into();
        config.timeout_seconds = 301;
        assert!(config.validate().is_err());
        config.timeout_seconds = 120;
        config.expected_viewport = [1147, 1344];
        assert!(config.validate().is_err());
    }
    #[test]
    fn minimum_viewport_tolerance_rejects_nonfinite_and_restored_large_windows() {
        let config = configuration();
        assert!(config.viewport_matches([1050.5, 679.5]));
        assert!(!config.viewport_matches([1147.0, 1344.0]));
        assert!(!config.viewport_matches([f32::NAN, 680.0]));
        assert!(!config.viewport_matches([1050.0, f32::INFINITY]));
    }
}
