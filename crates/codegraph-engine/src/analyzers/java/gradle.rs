//! Gradle build execution and ownership metadata; no call resolution.
use super::build::{JavaBuildMetadata, JavaSourceSet};
use crate::{
    analyzers::{AnalysisJob, choose_artifact, find_tool, process::run_process},
    discovery::BuildTarget,
};
use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) fn metadata(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
) -> Result<JavaBuildMetadata> {
    (context.progress)(format!(
        "Reading Gradle source-set ownership for {}…",
        build.label
    ));
    task(build, context, "codegraphExportMetadata")?;
    read_metadata(build, context)
}

fn read_metadata(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<JavaBuildMetadata> {
    let mut sets: Vec<JavaSourceSet> =
        serde_json::from_slice(&fs::read(context.path.join("gradle-metadata.json"))?)?;
    for source in &mut sets {
        source.project = if source.project == ":" {
            build.label.clone()
        } else {
            format!("{}{}", build.label, source.project)
        };
    }
    Ok(JavaBuildMetadata {
        modules: sets
            .iter()
            .map(|source| source.project_dir.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
        source_sets: sets,
        build_inputs: Vec::new(),
        output_roots: vec![build.path.join("build")],
    })
}

fn task(build: &BuildTarget, context: &AnalysisJob<'_>, task: &str) -> Result<()> {
    let runtime = context.runtime;
    let java_home = runtime
        .java_home
        .as_ref()
        .context("Java analysis requires an installed JDK 25")?;
    let metadata = context.path.join("gradle-metadata.json");
    let script = choose_artifact(
        &runtime.bundle_root,
        &[
            "tools/gradle-metadata.gradle",
            "analyzers/gradle-metadata.gradle",
        ],
    )?;
    run_process(
        Command::new(gradle_command(&build.path)?)
            .current_dir(&build.path)
            .env("JAVA_HOME", java_home)
            .arg("--no-daemon")
            .arg("--console=plain")
            .arg("-I")
            .arg(script)
            .arg(format!("-PcodegraphMetadataFile={}", metadata.display()))
            .arg(task),
        context.path,
        task,
        context.cancel,
    )?;
    Ok(())
}

fn gradle_command(root: &Path) -> Result<PathBuf> {
    let wrapper = root.join("gradlew");
    if wrapper.is_file() {
        return Ok(wrapper);
    }
    if let Some(path) = find_tool("gradle") {
        return Ok(path);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("No home folder for Gradle discovery")?;
    let versions = home.join(".gradle/wrapper/dists/gradle-9.4.0-bin");
    for entry in fs::read_dir(versions).context("No Gradle wrapper or installed Gradle found")? {
        let path = entry?.path().join("gradle-9.4.0/bin/gradle");
        if path.is_file() {
            return Ok(path);
        }
    }
    bail!("No Gradle wrapper or installed Gradle found")
}

pub(super) fn compile(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<JavaBuildMetadata> {
    let metadata = context.path.join("gradle-metadata.json");
    if metadata.exists() {
        fs::remove_file(&metadata)?;
    }
    task(build, context, "codegraphCompileAndExport")?;
    read_metadata(build, context)
}
