//! Shared, validated build ownership and preparation stages for Java analyzers.
use super::{gradle, maven};
use crate::{
    analyzers::{AnalysisJob, ensure_unchanged, source_snapshot},
    discovery::BuildTarget,
    fingerprint,
    model::FileFingerprint,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct JavaSourceSet {
    pub project: String,
    #[serde(default)]
    pub input_policy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<String>,
    pub source_set: String,
    pub compile_classpath: Vec<PathBuf>,
    #[serde(default)]
    pub classpath_aliases: Vec<PathBuf>,
    pub project_dir: PathBuf,
    pub is_test: bool,
    pub source_roots: Vec<PathBuf>,
    pub generated_roots: Vec<PathBuf>,
    pub class_dirs: Vec<PathBuf>,
    #[serde(default)]
    pub raw_class_dirs: Option<Vec<PathBuf>>,
    #[serde(default)]
    pub build_inputs: Vec<PathBuf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct JavaBuildMetadata {
    pub modules: Vec<PathBuf>,
    pub source_sets: Vec<JavaSourceSet>,
    pub build_inputs: Vec<PathBuf>,
    pub output_roots: Vec<PathBuf>,
}

pub(in crate::analyzers) struct PreparedJava {
    metadata: JavaBuildMetadata,
    pub(super) authored_before: Vec<FileFingerprint>,
}

pub(in crate::analyzers) fn prepare(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
) -> Result<PreparedJava> {
    let _stage = crate::metrics::stage("java.build_preparation");
    ensure!(
        context.runtime.java_home.is_some(),
        "Java analysis requires an installed JDK 25; configure JAVA_HOME or install JDK 25"
    );
    let mut before = source_snapshot(std::slice::from_ref(&build.path))?;
    if build.kind == "maven" {
        before.extend(maven::configuration_snapshot(&build.path)?);
    }
    let metadata = match build.kind.as_str() {
        "gradle" => gradle::metadata(build, context)?,
        "maven" => maven::metadata(build, context)?,
        other => bail!("Unsupported Java build kind: {other}"),
    };
    ensure_unchanged(&before)?;
    validate(&metadata)?;
    let authored_before = snapshot(build, &metadata, false)?;
    Ok(PreparedJava {
        metadata,
        authored_before,
    })
}

pub(super) fn compile(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<JavaBuildMetadata> {
    let _stage = crate::metrics::stage("java.compilation");
    let metadata = match build.kind.as_str() {
        "gradle" => gradle::compile(build, context)?,
        "maven" => maven::compile(build, context)?,
        other => bail!("Unsupported Java build kind: {other}"),
    };
    validate(&metadata)?;
    Ok(metadata)
}

fn validate(metadata: &JavaBuildMetadata) -> Result<()> {
    ensure!(
        !metadata.modules.is_empty(),
        "Build metadata contains no modules"
    );
    let modules = metadata.modules.iter().collect::<BTreeSet<_>>();
    ensure!(
        modules.len() == metadata.modules.len(),
        "Build metadata contains duplicate module ownership"
    );
    for module in &metadata.modules {
        ensure!(
            module.is_absolute() && module.is_dir(),
            "Invalid module directory: {}",
            module.display()
        );
    }
    let mut outputs = BTreeMap::new();
    for source in &metadata.source_sets {
        ensure!(
            source.input_policy == "scoped_v1",
            "Java adapter lacks scoped input policy; rebuild matching analyzer resources"
        );
        ensure!(
            modules.contains(&source.project_dir),
            "Source-set module ownership is missing"
        );
        ensure!(
            !source.project.is_empty() && !source.source_set.is_empty(),
            "Source-set project name is missing"
        );
        for path in source
            .source_roots
            .iter()
            .chain(&source.generated_roots)
            .chain(&source.build_inputs)
            .chain(&source.class_dirs)
        {
            ensure!(
                path.is_absolute(),
                "Build metadata path must be absolute: {}",
                path.display()
            );
        }
        for path in &source.class_dirs {
            ensure!(
                outputs.insert(path, &source.project).is_none(),
                "Class output has ambiguous ownership: {}",
                path.display()
            );
        }
    }
    for path in metadata.build_inputs.iter().chain(&metadata.output_roots) {
        ensure!(
            path.is_absolute(),
            "Build input/output path must be absolute: {}",
            path.display()
        );
    }
    Ok(())
}

pub(super) fn snapshot(
    build: &BuildTarget,
    metadata: &JavaBuildMetadata,
    generated: bool,
) -> Result<Vec<FileFingerprint>> {
    let mut roots = vec![fingerprint::ScanRoot::Required(build.path.clone())];
    roots.extend(
        metadata
            .modules
            .iter()
            .cloned()
            .map(fingerprint::ScanRoot::Required),
    );
    for source in &metadata.source_sets {
        roots.extend(
            source
                .source_roots
                .iter()
                .filter(|root| {
                    generated
                        || !source
                            .generated_roots
                            .iter()
                            .any(|output| root.starts_with(output))
                })
                .cloned()
                .map(fingerprint::ScanRoot::Required),
        );
        if generated {
            roots.extend(
                source
                    .generated_roots
                    .iter()
                    .cloned()
                    .map(fingerprint::ScanRoot::Required),
            );
        }
    }
    let mut paths = fingerprint::collect_inputs(&roots, &metadata.output_roots, true)?;
    // Build-declared inputs include resources and configuration of every type,
    // independently of broad source discovery exclusions.
    let inputs = metadata
        .build_inputs
        .iter()
        .chain(
            metadata
                .source_sets
                .iter()
                .flat_map(|source| &source.build_inputs),
        )
        .cloned()
        .map(fingerprint::ScanRoot::Required)
        .collect::<Vec<_>>();
    paths.extend(fingerprint::collect_inputs(
        &inputs,
        &metadata.output_roots,
        false,
    )?);
    fingerprint::capture_excluding(paths, &metadata.output_roots)
}

impl PreparedJava {
    pub(super) fn verify_compiled(&self, metadata: &JavaBuildMetadata) -> Result<()> {
        ensure_unchanged(&self.authored_before)?;
        let before = self.metadata.modules.iter().collect::<BTreeSet<_>>();
        let after = metadata.modules.iter().collect::<BTreeSet<_>>();
        ensure!(
            before == after,
            "Build module membership changed during compilation"
        );
        for source in &metadata.source_sets {
            let java = fingerprint::source_paths(&source.source_roots)?
                .iter()
                .any(|path| path.extension().is_some_and(|ext| ext == "java"));
            if java {
                let complete =
                    source
                        .class_dirs
                        .iter()
                        .map(|path| {
                            Ok(path.join("module-info.class").is_file()
                                || super::contains_class(path)?)
                        })
                        .collect::<Result<Vec<_>>>()?;
                ensure!(
                    complete.iter().any(|found| *found),
                    "No compiled Java output for {} ({})",
                    source.project,
                    if source.is_test { "test" } else { "production" }
                );
            }
        }
        Ok(())
    }
}

pub(in crate::analyzers) fn reject_overlaps<'a>(
    prepared: impl IntoIterator<Item = (&'a BuildTarget, &'a PreparedJava)>,
) -> Result<()> {
    let mut owners = BTreeMap::new();
    for (build, prepared) in prepared {
        for module in &prepared.metadata.modules {
            let canonical = module
                .canonicalize()
                .context("Cannot resolve module ownership")?;
            if let Some(previous) = owners.insert(canonical, build) {
                bail!(
                    "Selected builds {} and {} overlap at {}. Select only one owner of this module.",
                    previous.label,
                    build.label,
                    module.display()
                );
            }
        }
    }
    Ok(())
}
