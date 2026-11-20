//! Resolve evidence roles from Java build and compiler observations.
use super::build::JavaBuildMetadata;
use crate::{
    analyzers::{AnalyzerRuntime, evidence::EvidenceRole},
    discovery::BuildTarget,
    fingerprint::InputKind,
    model::FileFingerprint,
};
use std::{
    env,
    path::{Path, PathBuf},
};

pub(super) struct Roles {
    dependencies: Vec<PathBuf>,
    generated_sources: Vec<PathBuf>,
    classes: Vec<PathBuf>,
}

impl Roles {
    pub fn new(
        build: &BuildTarget,
        runtime: &AnalyzerRuntime,
        metadata: &JavaBuildMetadata,
    ) -> Self {
        let mut dependencies = match build.kind.as_str() {
            "maven" => maven_paths(),
            "gradle" => gradle_paths(),
            _ => Vec::new(),
        };
        dependencies.extend(runtime.java_home.iter().cloned());
        let dependencies = dependencies
            .into_iter()
            .flat_map(|path| {
                let canonical = path.canonicalize().ok();
                std::iter::once(path).chain(canonical)
            })
            .collect();
        Self {
            dependencies,
            generated_sources: metadata
                .source_sets
                .iter()
                .flat_map(|source| source.generated_roots.iter().cloned())
                .collect(),
            classes: metadata
                .source_sets
                .iter()
                .flat_map(|source| source.class_dirs.iter().cloned())
                .collect(),
        }
    }

    pub fn classify(&self, input: &FileFingerprint) -> EvidenceRole {
        let path = Path::new(&input.path);
        if self
            .dependencies
            .iter()
            .any(|dependency| path.starts_with(dependency))
        {
            return EvidenceRole::Dependency;
        }
        if input.kind == InputKind::File && self.owns_product(path) {
            return EvidenceRole::Generated;
        }
        EvidenceRole::Revision
    }
    fn owns_product(&self, path: &Path) -> bool {
        let roots = match path.extension().and_then(|part| part.to_str()) {
            Some("java") => &self.generated_sources,
            Some("class") => &self.classes,
            _ => return false,
        };
        roots.iter().any(|root| path.starts_with(root))
    }
}

fn maven_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = user_directory("MAVEN_USER_HOME", ".m2") {
        paths.extend(
            [
                "repository",
                "wrapper/dists",
                "settings.xml",
                "toolchains.xml",
            ]
            .map(|entry| home.join(entry)),
        );
    }
    let installation = env::var_os("MAVEN_HOME").map(PathBuf::from).or_else(|| {
        super::super::find_tool("mvn")
            .and_then(|path| path.canonicalize().ok())
            .and_then(|path| path.parent()?.parent().map(Path::to_path_buf))
    });
    if let Some(installation) = installation {
        for directory in ["conf", "libexec/conf"] {
            paths.extend(
                ["settings.xml", "toolchains.xml"]
                    .map(|entry| installation.join(directory).join(entry)),
            );
        }
    }
    paths
}

fn gradle_paths() -> Vec<PathBuf> {
    let caches = user_directory("GRADLE_USER_HOME", ".gradle")
        .into_iter()
        .flat_map(|home| {
            [
                "gradle.properties",
                "init.gradle",
                "init.gradle.kts",
                "caches",
                "wrapper/dists",
            ]
            .map(|entry| home.join(entry))
        });
    let maven_local = user_directory("MAVEN_USER_HOME", ".m2").map(|home| home.join("repository"));
    caches.chain(maven_local).collect()
}

fn user_directory(variable: &str, default: &str) -> Option<PathBuf> {
    env::var_os(variable)
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(default)))
}
