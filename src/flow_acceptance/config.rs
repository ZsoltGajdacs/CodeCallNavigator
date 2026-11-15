use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Ordinary {
    pub version: u32,
    pub project: PathBuf,
    pub root_symbol: String,
    pub target_symbol: String,
    pub source: String,
    pub expected_viewport: [u32; 2],
    #[serde(default = "timeout_seconds")]
    pub timeout_seconds: u64,
}

pub(super) enum Configuration {
    Single(Ordinary),
    Comparison(Box<super::comparison::config::Configuration>),
}
impl Configuration {
    pub fn timeout(&self) -> Duration {
        match self {
            Self::Single(config) => config.timeout(),
            Self::Comparison(config) => config.timeout(),
        }
    }
    pub fn load(directory: &Path) -> Result<Self> {
        let mut bytes = Vec::new();
        File::open(directory.join("config.json"))?
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 4 * 1024 * 1024,
            "Flow acceptance configuration exceeds 4 MiB"
        );
        let version = serde_json::from_slice::<serde_json::Value>(&bytes)?
            .get("version")
            .and_then(|v| v.as_u64());
        match version {
            Some(1) => Ordinary::decode(&bytes).map(Self::Single),
            Some(2) => super::comparison::config::Configuration::decode(&bytes)
                .map(Box::new)
                .map(Self::Comparison),
            _ => anyhow::bail!("Unsupported flow acceptance configuration version"),
        }
    }
}

fn timeout_seconds() -> u64 {
    120
}

impl Ordinary {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut configuration: Self =
            serde_json::from_slice(bytes).context("Invalid flow acceptance configuration")?;
        configuration.validate()?;
        configuration.project = configuration
            .project
            .canonicalize()
            .context("Cannot open flow acceptance project")?;
        ensure!(
            configuration.project.is_dir(),
            "Flow acceptance project must be a directory"
        );
        Ok(configuration)
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1,
            "Unsupported flow acceptance configuration version"
        );
        ensure!(
            self.project.is_absolute(),
            "Flow acceptance project must be absolute"
        );
        ensure!(
            !self.root_symbol.trim().is_empty() && !self.target_symbol.trim().is_empty(),
            "Flow acceptance symbols must be explicit"
        );
        ensure!(
            self.root_symbol != self.target_symbol,
            "Flow acceptance requires a distinct callee"
        );
        ensure!(
            !self.source.is_empty(),
            "Flow acceptance requires exact analyzed source"
        );
        ensure!(
            self.expected_viewport.into_iter().all(|size| size > 0),
            "Flow acceptance viewport dimensions must be positive"
        );
        ensure!(
            (1..=300).contains(&self.timeout_seconds),
            "Flow acceptance timeout must be 1..300 seconds"
        );
        Ok(())
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

pub(super) fn validate_launch(
    headless: bool,
    arguments: &[String],
    pan_profile: bool,
) -> Result<()> {
    ensure!(
        !headless,
        "Flow acceptance requires an actual native window"
    );
    ensure!(
        arguments.iter().any(|value| value == "--native"),
        "Flow acceptance requires --native"
    );
    ensure!(
        arguments
            .windows(2)
            .any(|pair| pair[0] == "--trace" && !pair[1].starts_with('-')),
        "Flow acceptance requires --trace PATH"
    );
    ensure!(
        !arguments.iter().any(|value| value == "--scene"),
        "Flow acceptance requires normal CLI workspace boot, not a fixture scene"
    );
    ensure!(
        !pan_profile,
        "Flow acceptance and pan profiling cannot install competing compositors"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).into()).collect()
    }

    #[test]
    fn comparison_configuration_dispatches_version_two_without_using_single_flow_fields() {
        let project = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let configuration = super::super::comparison::config::tests::value(project.path());
        std::fs::write(
            output.path().join("config.json"),
            serde_json::to_vec(&configuration).unwrap(),
        )
        .unwrap();
        let result = Configuration::load(output.path());
        assert!(
            result.is_ok(),
            "Comparison launch configuration must dispatch independently: {}",
            result.err().unwrap()
        );
    }

    #[test]
    fn acceptance_requires_an_explicit_expected_viewport() {
        let configuration = serde_json::json!({
            "version": 1, "project": "/tmp/project", "root_symbol": "Run",
            "target_symbol": "Leaf", "source": "class Example {}"
        });
        assert!(serde_json::from_value::<Ordinary>(configuration).is_err());
    }

    #[test]
    fn viewport_gate_rejects_the_observed_oversized_native_window() {
        let configuration: Ordinary = serde_json::from_value(serde_json::json!({
            "version": 1, "project": "/tmp/project", "root_symbol": "Run",
            "target_symbol": "Leaf", "source": "class Example {}",
            "expected_viewport": [1050, 680]
        }))
        .unwrap();
        assert!(configuration.viewport_matches([1050.0, 680.0]));
        assert!(configuration.viewport_matches([1049.0, 681.0]));
        assert!(!configuration.viewport_matches([1147.0, 1344.0]));
        assert!(!configuration.viewport_matches([1050.0, 682.0]));
        assert!(!configuration.viewport_matches([f32::NAN, 680.0]));
        assert!(configuration.validate().is_ok());
        let invalid = Ordinary {
            expected_viewport: [0, 680],
            ..configuration
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn acceptance_requires_native_trace_and_rejects_fixture_or_competing_modes() {
        let native = arguments(&["app", "--native", "--trace", "native.jsonl"]);
        assert!(validate_launch(false, &native, false).is_ok());
        assert!(validate_launch(true, &native, false).is_err());
        assert!(validate_launch(false, &native, true).is_err());
        assert!(validate_launch(false, &arguments(&["app", "--native"]), false).is_err());
        assert!(
            validate_launch(false, &arguments(&["app", "--trace", "trace.jsonl"]), false).is_err()
        );
        let mut scene = native;
        scene.extend(arguments(&["--scene", "function-flow"]));
        assert!(validate_launch(false, &scene, false).is_err());
    }
}
