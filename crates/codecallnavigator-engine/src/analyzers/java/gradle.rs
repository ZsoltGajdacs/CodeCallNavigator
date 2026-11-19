//! Gradle build execution and ownership metadata; no call resolution.
use super::{
    build::{JavaBuildMetadata, JavaCompilation, JavaSourceSet},
    compile_observation::Attempt,
};
use crate::{
    analyzers::{
        AnalysisJob, build_worker_count, choose_artifact, find_tool, process::run_process,
    },
    discovery::BuildTarget,
    fingerprint,
};
use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::Ordering,
};

pub(super) fn metadata(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
) -> Result<JavaBuildMetadata> {
    (context.progress)(format!(
        "Reading Gradle source-set ownership for {}…",
        build.label
    ));
    task(build, context, "codecallnavigatorExportMetadata", None)?;
    read_metadata(build, context)
}

fn read_metadata(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<JavaBuildMetadata> {
    let mut metadata: JavaBuildMetadata =
        serde_json::from_slice(&fs::read(context.path.join("gradle-metadata.json"))?)
            .context("Invalid Gradle ownership/compile-observation metadata; rebuild matching analyzer resources")?;
    for source in &mut metadata.source_sets {
        label_project(source, &build.label);
    }
    Ok(metadata)
}

pub(super) fn configuration_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = fingerprint::source_paths(&[root.to_path_buf()])?
        .into_iter()
        .filter(|path| {
            path.to_string_lossy().ends_with(".gradle")
                || path.to_string_lossy().ends_with(".gradle.kts")
                || path
                    .file_name()
                    .is_some_and(|name| name == "gradle.properties")
        })
        .collect::<Vec<_>>();
    paths.extend(
        [
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
            "gradle.properties",
            "gradlew",
            "gradlew.bat",
            "gradle/wrapper/gradle-wrapper.properties",
            "gradle/wrapper/gradle-wrapper.jar",
        ]
        .map(|path| root.join(path)),
    );
    if let Some(home) = std::env::var_os("GRADLE_USER_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".gradle")))
    {
        paths.extend([
            home.join("gradle.properties"),
            home.join("init.gradle"),
            home.join("init.gradle.kts"),
        ]);
    }
    Ok(paths)
}

fn task(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
    task: &str,
    observation: Option<&Attempt>,
) -> Result<()> {
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
    let helper = choose_artifact(
        &runtime.bundle_root,
        &[
            "analyzers/maven-metadata.jar",
            "tools/maven-metadata/build/libs/codecallnavigator-maven-metadata.jar",
        ],
    )?;
    let mut command = Command::new(gradle_command(&build.path)?);
    command
        .current_dir(&build.path)
        .env("JAVA_HOME", java_home)
        .arg("--no-daemon")
        // Public task-boundary listeners cannot use Gradle's configuration cache.
        // Compiler incrementality and the native build cache remain enabled.
        .arg("--no-configuration-cache")
        .arg("--console=plain")
        .arg("-I")
        .arg(script)
        .arg(format!(
            "-Dcodecallnavigator.observerJar={}",
            helper.display()
        ))
        .arg(format!(
            "-PcodecallnavigatorMetadataFile={}",
            metadata.display()
        ));
    if let Some(observation) = observation {
        command
            .args(["--parallel", "--max-workers"])
            .arg(build_worker_count().to_string());
        observation.configure(&mut command);
    }
    command.arg(task);
    run_process(&mut command, context.path, task, context.cancel)?;
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

pub(super) fn compile(build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<JavaCompilation> {
    let metadata = context.path.join("gradle-metadata.json");
    if metadata.exists() {
        fs::remove_file(&metadata)?;
    }
    let attempt = Attempt::new(context.path)?;
    let native = task(
        build,
        context,
        "codecallnavigatorCompileAndExport",
        Some(&attempt),
    );
    let observations = attempt.finish(native, context.cancel.load(Ordering::Relaxed))?;
    Ok(JavaCompilation {
        metadata: read_metadata(build, context)?,
        observations,
    })
}

fn label_project(source: &mut JavaSourceSet, label: &str) {
    source.application = Some(source.project.clone());
    source.project = if source.project == ":" {
        label.into()
    } else {
        format!("{label}{}", source.project)
    };
}

#[cfg(test)]
mod application_tests {
    use super::*;
    #[test]
    fn application_project_paths_are_independent_of_checkout_display_labels() {
        let source = |project: &str| {
            serde_json::from_value::<JavaSourceSet>(serde_json::json!({"project":project,"sourceSet":"main","compileClasspath":[],"projectDir":"/tmp/app","isTest":false,"sourceRoots":[],"generatedRoots":[],"classDirs":[]})).unwrap()
        };
        for application in [":", ":sub"] {
            let mut first = source(application);
            let mut moved = source(application);
            label_project(&mut first, "first");
            label_project(&mut moved, "renamed");
            assert_ne!(first.project, moved.project);
            assert_eq!(first.application.as_deref(), Some(application));
            assert_eq!(first.application, moved.application);
        }
    }
}
