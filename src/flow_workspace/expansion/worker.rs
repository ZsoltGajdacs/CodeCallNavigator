//! CLI, cancellable Graphviz and snapshot highlighting all run off the UI thread.
use super::model::{Prepared, Published};
use crate::{
    engine_client::{Descriptor, QueryContext},
    flow_workspace::Ready,
    source,
};
use anyhow::{Context, Result, ensure};
use codecallnavigator::{flow, query::FlowExpansionRequest};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
pub(super) fn prepare(
    runtime: Arc<crate::runtime::Runtime>,
    descriptor: Descriptor,
    request: FlowExpansionRequest,
    cancel: &AtomicBool,
) -> Result<Prepared> {
    let start = Instant::now();
    // Each request owns a cold CLI connection, including capability negotiation
    // and teardown in query_time. No process-owning client enters the UI queue.
    let context = QueryContext::new(&runtime, descriptor);
    let result = context.expand_flow(request, cancel);
    drop(context);
    let mut result = result?;
    let query_time = start.elapsed();
    let start = Instant::now();
    result.chart = flow::layout_cancellable(result.chart, &runtime.dot, cancel)?;
    let layout_time = start.elapsed();
    result.validate()?;
    let root_source = result.occurrences[0]
        .source
        .context("Expanded root has no saved source")?;
    let mut sources = Vec::new();
    let mut keys = Vec::new();
    for id in 0..result.sources.len() {
        ensure!(
            !cancel.load(Ordering::Acquire),
            "Expansion source preparation cancelled"
        );
        let snapshot = result.sources.get(flow::SourceId(id)).unwrap();
        let identity = serde_json::to_vec(&(&snapshot.owner, &snapshot.digest))?;
        keys.push(format!(
            "flow-expanded-source-{}",
            blake3::hash(&identity).to_hex()
        ));
        sources.push(Some(source::from_snapshot(
            snapshot.content.clone(),
            Path::new(&snapshot.owner.file),
        )));
    }
    let source = sources[root_source.0].take().unwrap();
    let ready = Ready {
        chart: result.chart.clone(),
        source,
    };
    let published = Arc::new(Published {
        badges: super::badges::collect(&result),
        result,
        sources,
        keys,
        root_source,
        query_time,
        layout_time,
    });
    ensure!(!cancel.load(Ordering::Acquire), "Flow expansion cancelled");
    Ok(Prepared { ready, published })
}
