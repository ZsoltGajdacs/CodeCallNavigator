//! Sequential JVM engine execution, sharing the application's cancellation policy.
use super::{JavaInput, metadata::unescape};
use crate::analyzers::{AnalysisJob, process::run_process};
use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) fn run(
    inputs: &[JavaInput],
    owner: &super::build::JavaSourceSet,
    root: &Path,
    job: &AnalysisJob<'_>,
) -> Result<PathBuf> {
    fs::create_dir_all(root)?;
    let java = job
        .runtime
        .java_home
        .as_ref()
        .context("Java analysis requires JDK 25")?;
    let output = root.join("bytecode");
    let projects = inputs
        .iter()
        .enumerate()
        .filter(|(_, input)| {
            input.source_set.as_ref().is_some_and(|source| {
                source.project_dir == owner.project_dir && source.source_set == owner.source_set
            })
        })
        .map(|(index, _)| (index + 1).to_string())
        .collect::<Vec<_>>()
        .join(",");
    run_process(
        Command::new(java.join("bin/java"))
            .arg("-Xmx8g")
            .arg("-cp")
            .arg(job.runtime.bytecode_lib()?.join("*"))
            .arg("codegraph.bytecode.Analyzer")
            .arg(&output)
            .arg("25")
            .arg("--project-inputs")
            .arg(projects)
            .args(inputs.iter().map(|input| &input.path)),
        root,
        "bytecode",
        job.cancel,
    )?;
    Ok(output)
}

// Parse the Java Properties subset emitted by the adapter.
pub(super) fn status(path: &Path) -> Result<HashMap<String, String>> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_owned(), unescape(value)))
        .collect())
}
