//! Shared, validated build ownership and preparation stages for Java analyzers.
use super::{gradle, maven};
use crate::{
    analyzers::{AnalysisJob, ensure_unchanged, source_snapshot},
    discovery::BuildTarget,
    fingerprint,
    model::FileFingerprint,
};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct JavaSourceSet {
    pub project: String,
    pub project_dir: PathBuf,
    pub is_test: bool,
    pub source_roots: Vec<PathBuf>,
    pub generated_roots: Vec<PathBuf>,
    pub class_dirs: Vec<PathBuf>,
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
    ensure!(
        context.runtime.java_home.is_some(),
        "Java analysis requires an installed JDK 25; configure JAVA_HOME or install JDK 25"
    );
    let before = source_snapshot(std::slice::from_ref(&build.path))?;
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
            modules.contains(&source.project_dir),
            "Source-set module ownership is missing"
        );
        ensure!(
            !source.project.is_empty(),
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
    let mut roots = vec![build.path.clone()];
    roots.extend(metadata.modules.clone());
    roots.extend(metadata.build_inputs.clone());
    for source in &metadata.source_sets {
        roots.extend(source.build_inputs.clone());
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
                .cloned(),
        );
        if generated {
            roots.extend(source.generated_roots.clone());
        }
    }
    let mut paths = fingerprint::source_paths_excluding(&roots, &metadata.output_roots)?;
    let mut inputs = metadata.build_inputs.clone();
    inputs.extend(
        metadata
            .source_sets
            .iter()
            .flat_map(|source| source.build_inputs.clone()),
    );
    paths.extend(fingerprint::input_paths_excluding(
        &inputs,
        &metadata.output_roots,
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
                let complete = source
                    .class_dirs
                    .iter()
                    .map(|path| super::contains_class(path))
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

pub(in crate::analyzers) fn reject_overlaps(
    builds: &[BuildTarget],
    prepared: &[Option<PreparedJava>],
) -> Result<()> {
    let mut owners = BTreeMap::new();
    for (build, prepared) in builds.iter().zip(prepared) {
        let Some(prepared) = prepared else {
            continue;
        };
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
