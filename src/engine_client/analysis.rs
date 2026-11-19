//! Run an analysis and load the exact generation and outcome the engine published.
use super::{Descriptor, QueryContext, Snapshot, exchange, require_running};
use crate::runtime::Runtime;
use anyhow::{Context, Result, bail};
use codecallnavigator::{
    discovery::BuildTarget,
    protocol::{Event, Request, VERSION},
};
use std::{path::PathBuf, sync::atomic::AtomicBool};

#[derive(Clone)]
pub struct Analysis {
    pub project: PathBuf,
    pub targets: Vec<BuildTarget>,
}

pub fn analyze_snapshot(
    runtime: &Runtime,
    selection: Analysis,
    cancel: &AtomicBool,
    progress: &dyn Fn(String),
) -> Result<Snapshot> {
    let project = selection
        .project
        .canonicalize()
        .context("Cannot open analysis project")?;
    let request = Request::Analyze {
        version: VERSION,
        project: project.clone(),
        roots: Vec::new(),
        infer_linked_roots: true,
        failure_version: Some(1),
        expected_generation: None,
        targets: selection.targets,
    };
    let Event::Complete {
        roots,
        digest,
        metrics,
        engine_versions,
        retention,
        ..
    } = exchange(runtime, &request, cancel, progress)?
    else {
        bail!("Engine returned discovery results for analysis");
    };
    require_running(cancel)?;
    codecallnavigator::metrics::absorb(&metrics)?;
    let roots = roots
        .into_iter()
        .filter(|(name, _)| name != "repo")
        .collect();
    let mut snapshot = QueryContext::new(
        runtime,
        Descriptor {
            project,
            roots,
            generation: Some(digest),
        },
    )
    .snapshot_with_versions(cancel, Some(engine_versions))?;
    snapshot.retention = retention;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn analyze(
    runtime: &Runtime,
    selection: Analysis,
    cancel: &AtomicBool,
    progress: &dyn Fn(String),
) -> Result<codecallnavigator::model::GraphData> {
    let snapshot = analyze_snapshot(runtime, selection, cancel, progress)?;
    Ok(snapshot.graph.data.to_graph_data())
}
