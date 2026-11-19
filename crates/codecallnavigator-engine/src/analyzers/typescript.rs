//! Installed TypeScript compiler bridge. Ownership and binding belong to the Node adapter.
use super::{AnalysisJob, ensure_unchanged};
mod session_worker;
use crate::{
    discovery::BuildTarget,
    fingerprint,
    model::{FileFingerprint, GraphData},
};
use anyhow::{Result, bail, ensure};
use serde::Deserialize;
use session_worker::{CompilerWorker, Diagnostic, Operation, Reply, Status};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::PathBuf,
};

#[derive(Deserialize, PartialEq, Eq)]
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
    worker: Option<CompilerWorker>,
    stale: bool,
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

/// Validate fresh resource compatibility independently of process preparation.
fn validate_capabilities(versions: &serde_json::Value) -> Result<()> {
    ensure!(
        versions.get("adapter").and_then(|value| value.as_str()) == Some("7"),
        "TypeScript adapter lacks compiler execution/invocation facts version 7; rebuild matching analyzer resources"
    );
    ensure!(
        versions
            .get("input_policy")
            .and_then(|value| value.as_str())
            == Some("scoped_v1"),
        "TypeScript adapter lacks scoped input policy; rebuild matching analyzer resources"
    );
    ensure!(
        versions
            .get("boundary_rules")
            .and_then(|value| value.as_str())
            == Some("1"),
        "TypeScript adapter lacks boundary inventory support; rebuild matching analyzer resources"
    );
    Ok(())
}

pub(super) fn prepare(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
) -> Result<PreparedTypeScript> {
    let _timing = crate::metrics::stage("typescript.prepare");
    (context.progress)(format!("Preparing TypeScript project {}…", build.label));
    let (mut worker, metadata) = CompilerWorker::start(build, context)?;
    let before = match prepare_inputs(&metadata) {
        Ok(inputs) => inputs,
        Err(error) => return worker.stop(Err(error)),
    };
    Ok(PreparedTypeScript {
        metadata,
        before,
        worker: Some(worker),
        stale: false,
    })
}

fn prepare_inputs(metadata: &Metadata) -> Result<Vec<FileFingerprint>> {
    validate_capabilities(&metadata.versions)?;
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
    fingerprint::capture_excluding(paths, &metadata.output_roots)
}

impl PreparedTypeScript {
    pub(super) fn release_worker(&mut self) -> Result<()> {
        match self.worker.take() {
            Some(mut worker) => worker.close(),
            None => Ok(()),
        }
    }

    pub(super) fn invalidate_for_update(&mut self) {
        self.stale = true;
    }

    pub(super) fn needs_update(&self) -> bool {
        self.stale
    }

    pub(super) fn verify(&self) -> Result<()> {
        ensure_unchanged(&self.before)
    }

    pub(super) fn update(&mut self, build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<()> {
        let Some(mut worker) = self.worker.take() else {
            *self = prepare(build, context)?;
            return Ok(());
        };
        (context.progress)(format!("Updating TypeScript project {}…", build.label));
        let reply = match worker.request(Operation::Update, context) {
            Ok(reply) => reply,
            Err(error) => return worker.stop(Err(error)),
        };
        match reply.status {
            Status::Success => {
                let result = reply.into_metadata().and_then(|metadata| {
                    let before = prepare_inputs(&metadata)?;
                    Ok((metadata, before))
                });
                let (metadata, before) = match result {
                    Ok(inputs) => inputs,
                    Err(error) => return worker.stop(Err(error)),
                };
                self.metadata = metadata;
                self.before = before;
                self.worker = Some(worker);
                self.stale = false;
            }
            Status::Error => {
                self.worker = Some(worker);
                return Err(native_error(reply, build));
            }
            Status::Restart => {
                worker.stop(Ok(()))?;
                *self = prepare(build, context)?;
            }
        }
        Ok(())
    }

    fn output(&mut self, build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<Output> {
        self.verify()?;
        let mut worker = self.resume(build, context)?;
        let reply = match worker.request(Operation::Analyze, context) {
            Ok(reply) => reply,
            Err(error) => return worker.stop(Err(error)),
        };
        match reply.status {
            Status::Success => {
                self.worker = Some(worker);
                self.verify()?;
                reply.into_graph()
            }
            Status::Error => {
                self.worker = Some(worker);
                Err(native_error(reply, build))
            }
            Status::Restart => {
                self.stale = true;
                worker.stop(Err(native_error(reply, build)))
            }
        }
    }

    fn resume(&mut self, build: &BuildTarget, context: &AnalysisJob<'_>) -> Result<CompilerWorker> {
        if let Some(worker) = self.worker.take() {
            return Ok(worker);
        }
        let _timing = crate::metrics::stage("typescript.reprepare");
        ensure_unchanged(&self.before)?;
        let (mut worker, metadata) = CompilerWorker::start(build, context)?;
        let result = self.check_resume(build, metadata);
        match result {
            Ok(()) => Ok(worker),
            Err(error) => worker.stop(Err(error)),
        }
    }

    fn check_resume(&self, build: &BuildTarget, metadata: Metadata) -> Result<()> {
        validate_capabilities(&metadata.versions)?;
        if metadata != self.metadata {
            return Err(fingerprint::InputsChanged::new(
                "TypeScript preparation changed before extraction",
                build.path.to_string_lossy().into_owned(),
                "compiler input membership or toolchain metadata changed",
            )
            .into());
        }
        ensure_unchanged(&self.before)
    }
}

fn native_error(reply: Reply, build: &BuildTarget) -> anyhow::Error {
    let Diagnostic {
        code,
        message,
        path,
    } = reply.error.expect("validated failure response");
    if matches!(code.as_str(), "inputs_changed" | "restart_required") {
        let path = path.as_deref().unwrap_or(&build.path);
        return anyhow::Error::new(fingerprint::InputsChanged::new(
            "TypeScript inputs changed during analysis.",
            path.to_string_lossy(),
            "compiler input changed",
        ))
        .context(message);
    }
    anyhow::anyhow!("{message}")
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

pub(super) fn extract(
    build: &BuildTarget,
    context: &AnalysisJob<'_>,
    prepared: &mut PreparedTypeScript,
) -> Result<GraphData> {
    let _timing = crate::metrics::stage("typescript.extract");
    let result = prepared.output(build, context)?;
    ensure!(
        result.schema == "codecallnavigator.typescript.v2",
        "Unsupported TypeScript adapter schema"
    );
    ensure!(
        result.versions == prepared.metadata.versions,
        "TypeScript toolchain changed during analysis"
    );
    ensure_unchanged(&prepared.before)?;
    let mut graph = result.graph;
    ensure!(
        graph.boundaries.is_some(),
        "TypeScript adapter omitted boundary inventory"
    );
    if graph.nodes.iter().any(|node| node.language != "typescript") {
        bail!("TypeScript adapter emitted a foreign language symbol");
    }
    graph.validate()?;
    crate::method_policy::prune(&mut graph, &result.excluded_methods)?;
    finalize_candidates(&mut graph);
    graph.validate()?;
    graph.fingerprints = prepared.before.clone();
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

#[cfg(test)]
#[path = "typescript_tests.rs"]
mod tests;
