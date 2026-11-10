//! Explicit deployment relationships. An analyzed checkout is not an HTTP origin.
use crate::{
    discovery::{self, BuildTarget},
    fingerprint,
    model::FileFingerprint,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub version: u32,
    #[serde(default)]
    pub http: Vec<HttpScope>,
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            version: 1,
            http: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpScope {
    pub client: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_application: Option<String>,
    pub servers: Vec<Server>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Server {
    Target(String),
    Application { target: String, application: String },
}
impl Server {
    pub fn target(&self) -> &str {
        match self {
            Self::Target(target) | Self::Application { target, .. } => target,
        }
    }
    pub fn application(&self) -> Option<&str> {
        match self {
            Self::Target(_) => None,
            Self::Application { application, .. } => Some(application),
        }
    }
}

impl Configuration {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 1,
            "Unsupported boundary configuration version"
        );
        for (index, scope) in self.http.iter().enumerate() {
            ensure!(!scope.servers.is_empty(), "Boundary scope needs a server");
            ensure!(
                scope
                    .client_application
                    .as_ref()
                    .is_none_or(|app| !app.trim().is_empty()),
                "Empty boundary client application"
            );
            ensure!(
                !self.http[..index].iter().any(|other| overlap(scope, other)),
                "Overlapping boundary client scopes"
            );
        }
        Ok(())
    }

    pub fn load(project: &Path, targets: &[BuildTarget]) -> Result<(Self, Vec<FileFingerprint>)> {
        let path = project.join(".codegraph/boundaries.yml");
        let before = fingerprint::capture_excluding([path.clone()], &[])?;
        let mut config: Self = if path.exists() {
            serde_yaml::from_str(&std::fs::read_to_string(&path)?)
                .context("Invalid boundaries.yml")?
        } else {
            Self::default()
        };
        ensure!(
            config.version == 1,
            "Unsupported boundary configuration version"
        );
        for scope in &mut config.http {
            scope.client = selected(project, &scope.client, targets)?;
            ensure!(!scope.servers.is_empty(), "Boundary scope needs a server");
            for server in &mut scope.servers {
                let normalized = selected(project, server.target(), targets)?;
                match server {
                    Server::Target(target) | Server::Application { target, .. } => {
                        *target = normalized
                    }
                }
            }
        }
        config.validate()?;
        ensure!(
            !fingerprint::is_stale(&before)?,
            "Boundary configuration changed during loading"
        );
        Ok((config, before))
    }
}
fn selected(project: &Path, selector: &str, targets: &[BuildTarget]) -> Result<String> {
    let path = project
        .join(selector)
        .canonicalize()
        .context("Cannot open boundary target")?;
    let target = if path.is_dir() {
        targets
            .iter()
            .find(|target| target.path == path)
            .cloned()
            .context("Boundary directory is not selected")?
    } else if matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("build.gradle" | "build.gradle.kts" | "settings.gradle" | "settings.gradle.kts")
    ) {
        let parent = path
            .parent()
            .context("Boundary build file has no directory")?;
        targets
            .iter()
            .find(|target| target.path == parent)
            .cloned()
            .context("Boundary Gradle target is not selected")?
    } else {
        discovery::target_from_file(&path)?
    };
    ensure!(
        targets
            .iter()
            .any(|selected| selected.identity() == target.identity()),
        "Boundary target must be selected: {selector}"
    );
    Ok(target.path.to_string_lossy().into_owned())
}

fn overlap(left: &HttpScope, right: &HttpScope) -> bool {
    if left.client != right.client {
        return false;
    }
    match (&left.client_application, &right.client_application) {
        (Some(left), Some(right)) => left == right,
        _ => true,
    }
}
