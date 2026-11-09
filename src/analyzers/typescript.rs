//! Installed TypeScript compiler bridge. Ownership and binding belong to the Node adapter.
use super::{AnalysisJob, ensure_unchanged, find_tool, process::run_process};
use crate::{
    discovery::BuildTarget,
    fingerprint,
    model::{FileFingerprint, GraphData},
};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::PathBuf,
    process::Command,
};

#[derive(Deserialize)]
struct Metadata {
    configs: Vec<PathBuf>,
    sources: Vec<PathBuf>,
    inputs: Vec<PathBuf>,
    watch_roots: Vec<PathBuf>,
    #[serde(default)]
    output_roots: Vec<PathBuf>,
    versions: serde_json::Value,
    owners: BTreeMap<PathBuf, PathBuf>,
}

pub(super) struct PreparedTypeScript {
    metadata: Metadata,
    before: Vec<FileFingerprint>,
}

fn adapter(context: &AnalysisJob<'_>) -> Result<PathBuf> {
    super::choose_artifact(
        &context.runtime.bundle_root,
        &[
            "analyzers/typescript/analyze.js",
            "tools/typescript/analyze.js",
        ],
    )
}

fn invoke(build: &BuildTarget, context: &AnalysisJob<'_>, mode: &str) -> Result<PathBuf> {
    let node = find_tool("node")
        .context("TypeScript analysis requires installed Node 24+ and project TypeScript 6.0.x")?;
    let output = context.path.join(format!("typescript-{mode}.json"));
    run_process(
        Command::new(node)
            .arg(adapter(context)?)
            .arg(mode)
            .arg(&build.path)
            .arg(&output),
        context.path,
        &format!("typescript-{mode}"),
        context.cancel,
    )?;
    Ok(output)
}

pub(super) fn prepare(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
) -> Result<PreparedTypeScript> {
    let output = invoke(build, context, "prepare")?;
    let metadata: Metadata = serde_json::from_slice(&fs::read(output)?)?;
    ensure!(
        !metadata.configs.is_empty(),
        "TypeScript emitted no configurations"
    );
    for path in metadata
        .configs
        .iter()
        .chain(&metadata.sources)
        .chain(&metadata.inputs)
        .chain(&metadata.watch_roots)
    {
        ensure!(
            path.is_absolute(),
            "TypeScript input is not absolute: {}",
            path.display()
        );
    }
    ensure!(
        metadata
            .sources
            .iter()
            .all(|source| metadata.owners.contains_key(source)),
        "TypeScript source ownership is incomplete"
    );
    let paths = metadata
        .inputs
        .iter()
        .chain(&metadata.sources)
        .chain(&metadata.configs)
        .chain(&metadata.watch_roots)
        .cloned()
        .collect::<BTreeSet<_>>();
    let before = fingerprint::capture_excluding(paths, &metadata.output_roots)?;
    Ok(PreparedTypeScript { metadata, before })
}

/// Identical referenced configurations share ownership; different configurations cannot own a file.
pub(super) fn reject_overlaps<'a>(
    prepared: impl IntoIterator<Item = &'a PreparedTypeScript>,
) -> Result<()> {
    let mut owners = BTreeMap::new();
    for prepared in prepared {
        for (source, owner) in &prepared.metadata.owners {
            if let Some(previous) = owners.insert(source, owner) {
                ensure!(
                    previous == owner,
                    "Selected TypeScript configurations overlap at {}",
                    source.display()
                );
            }
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct Output {
    schema: String,
    excluded_methods: HashSet<String>,
    #[serde(flatten)]
    graph: GraphData,
    versions: serde_json::Value,
}

pub(super) fn analyze(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
    prepared: PreparedTypeScript,
) -> Result<GraphData> {
    ensure_unchanged(&prepared.before)?;
    let output = invoke(build, context, "analyze")?;
    let result: Output = serde_json::from_slice(&fs::read(output)?)?;
    ensure!(
        result.schema == "codegraph.typescript.v2",
        "Unsupported TypeScript adapter schema"
    );
    ensure!(
        result.versions == prepared.metadata.versions,
        "TypeScript toolchain changed during analysis"
    );
    ensure_unchanged(&prepared.before)?;
    let mut graph = result.graph;
    if graph.nodes.iter().any(|node| node.language != "typescript") {
        bail!("TypeScript adapter emitted a foreign language symbol");
    }
    graph.validate()?;
    crate::method_policy::prune(&mut graph, &result.excluded_methods)?;
    finalize_candidates(&mut graph);
    graph.validate()?;
    graph.fingerprints = prepared.before;
    graph.engine_versions = result.versions.to_string();
    Ok(graph)
}

/// TypeScript reference relationships do not make a callable an execution target.
fn finalize_candidates(graph: &mut GraphData) {
    let called = graph
        .edges
        .iter()
        .filter(|edge| edge.kind != "reference")
        .map(|edge| edge.to)
        .collect::<HashSet<_>>();
    let candidates = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, node)| !called.contains(&(*index as u32)) && !node.body_unavailable)
        .map(|(_, node)| node.id.clone())
        .collect::<Vec<_>>();
    if let Some(entries) = &mut graph.entry_points {
        entries.candidates = candidates;
        entries.candidates.sort();
    }
}
