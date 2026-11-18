//! Shared, validated build ownership and preparation stages for Java analyzers.
use super::{compile_observation, gradle, maven};
use crate::{
    analyzers::{AnalysisJob, ensure_unchanged},
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
    pub compile_observation_version: u32,
    pub configuration_inputs: Vec<PathBuf>,
    pub modules: Vec<PathBuf>,
    pub source_sets: Vec<JavaSourceSet>,
    pub build_inputs: Vec<PathBuf>,
    pub output_roots: Vec<PathBuf>,
}

pub(in crate::analyzers) struct PreparedJava {
    metadata: JavaBuildMetadata,
    pub(super) configuration_before: Vec<FileFingerprint>,
}

pub(super) struct JavaCompilation {
    pub metadata: JavaBuildMetadata,
    pub observations: compile_observation::Observations,
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
    let before = configuration_snapshot(build)?;
    let metadata = match build.kind.as_str() {
        "gradle" => gradle::metadata(build, context)?,
        "maven" => maven::metadata(build, context)?,
        other => bail!("Unsupported Java build kind: {other}"),
    };
    ensure_unchanged(&before)?;
    validate(&metadata)?;
    let known = before
        .iter()
        .map(|input| PathBuf::from(&input.path))
        .collect::<BTreeSet<_>>();
    let mut configuration_before = before;
    configuration_before.extend(fingerprint::capture_excluding(
        metadata
            .configuration_inputs
            .iter()
            .filter(|path| !known.contains(*path))
            .cloned(),
        &[],
    )?);
    ensure_unchanged(&configuration_before)?;
    Ok(PreparedJava {
        metadata,
        configuration_before,
    })
}

fn configuration_snapshot(build: &BuildTarget) -> Result<Vec<FileFingerprint>> {
    let mut paths = match build.kind.as_str() {
        "maven" => {
            let mut paths = vec![build.path.join("pom.xml")];
            paths.extend(maven::configuration_paths(&build.path)?);
            paths
        }
        "gradle" => gradle::configuration_paths(&build.path)?,
        other => bail!("Unsupported Java build kind: {other}"),
    };
    // Only explicit configuration files enter the guard. In particular, the
    // project directory's membership must not freeze generators out of a build.
    paths.sort();
    paths.dedup();
    fingerprint::capture_excluding(paths, &[])
}

pub(super) fn compile(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<JavaCompilation> {
    let _stage = crate::metrics::stage("java.compilation");
    let metadata = match build.kind.as_str() {
        "gradle" => gradle::compile(build, context)?,
        "maven" => maven::compile(build, context)?,
        other => bail!("Unsupported Java build kind: {other}"),
    };
    validate(&metadata.metadata)?;
    Ok(metadata)
}

fn validate(metadata: &JavaBuildMetadata) -> Result<()> {
    ensure!(
        metadata.compile_observation_version == compile_observation::VERSION,
        "Java adapter lacks current compile observation capability; rebuild matching analyzer resources"
    );
    let modules = validate_modules(&metadata.modules)?;
    validate_source_sets(&metadata.source_sets, &modules)?;
    for path in metadata
        .build_inputs
        .iter()
        .chain(&metadata.output_roots)
        .chain(&metadata.configuration_inputs)
    {
        ensure!(
            path.is_absolute(),
            "Build input/output path must be absolute: {}",
            path.display()
        );
    }
    Ok(())
}

fn validate_modules(paths: &[PathBuf]) -> Result<BTreeSet<&PathBuf>> {
    ensure!(!paths.is_empty(), "Build metadata contains no modules");
    let modules = paths.iter().collect::<BTreeSet<_>>();
    ensure!(
        modules.len() == paths.len(),
        "Build metadata contains duplicate module ownership"
    );
    for module in paths {
        ensure!(
            module.is_absolute() && module.is_dir(),
            "Invalid module directory: {}",
            module.display()
        );
    }
    Ok(modules)
}

fn validate_source_sets(sources: &[JavaSourceSet], modules: &BTreeSet<&PathBuf>) -> Result<()> {
    let mut outputs = BTreeMap::new();
    for source in sources {
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
    pub(super) fn verify_compiled(
        &self,
        build: &BuildTarget,
        metadata: &JavaBuildMetadata,
    ) -> Result<()> {
        ensure_unchanged(&self.configuration_before)?;
        let before_configuration = self
            .metadata
            .configuration_inputs
            .iter()
            .collect::<BTreeSet<_>>();
        let after_configuration = metadata
            .configuration_inputs
            .iter()
            .collect::<BTreeSet<_>>();
        if before_configuration != after_configuration {
            return Err(fingerprint::InputsChanged::new(
                "Build configuration inputs changed during compilation.",
                build.path.to_string_lossy().into_owned(),
                "evaluated configuration membership changed",
            )
            .into());
        }
        let before = self.metadata.modules.iter().collect::<BTreeSet<_>>();
        let after = metadata.modules.iter().collect::<BTreeSet<_>>();
        if before != after {
            return Err(fingerprint::InputsChanged::new(
                "Build module membership changed during compilation",
                build.path.to_string_lossy().into_owned(),
                "evaluated build module membership changed",
            )
            .into());
        }
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
