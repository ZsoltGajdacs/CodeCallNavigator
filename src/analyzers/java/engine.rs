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
    source_inventories(inputs, &output, job)?;
    Ok(output)
}

pub(super) fn source_inventories(
    inputs: &[JavaInput],
    output: &Path,
    job: &AnalysisJob<'_>,
) -> Result<()> {
    let java = job
        .runtime
        .java_home
        .as_ref()
        .context("Source inventory requires JDK 25")?;
    let cache = job.path.join("source-inventories");
    fs::create_dir_all(&cache)?;
    for (index, input) in inputs.iter().enumerate() {
        let Some(source) = &input.source_set else {
            continue;
        };
        let classpath =
            std::env::join_paths(source.class_dirs.iter().chain(&source.compile_classpath))?;
        let key = blake3::hash(&serde_json::to_vec(&serde_json::json!([
            source.project_dir,
            source.source_set
        ]))?)
        .to_hex()
        .to_string();
        let inventory = cache.join(format!("{key}.tsv"));
        if !inventory.is_file() {
            run_process(
                Command::new(java.join("bin/java"))
                    .arg("-Xmx2g")
                    .arg("-cp")
                    .arg(job.runtime.bytecode_lib()?.join("*"))
                    .arg("codegraph.bytecode.SourceInventory")
                    .arg(&inventory)
                    .arg(classpath)
                    .arg(std::env::join_paths(&source.class_dirs)?)
                    .args(source.source_roots.iter().chain(&source.generated_roots)),
                output,
                &format!("source-inventory-{}", index + 1),
                job.cancel,
            )?;
        }
        fs::copy(inventory, output.join(format!("source-{}.tsv", index + 1)))?;
    }
    Ok(())
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
