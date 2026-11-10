//! Cache lifetime belongs to the output-owning project, under its analysis lock.
use anyhow::Result;
use codegraph_core::{
    boundary::{self, Configuration, DecisionCache},
    metrics,
    model::GraphData,
};
use std::path::Path;

pub fn resolve(
    project: &Path,
    graph: &mut GraphData,
    configuration: &Configuration,
) -> Result<Option<DecisionCache>> {
    let _stage = metrics::stage("inference.boundaries");
    let relationships = graph.edges.len();
    let cache = if std::env::var_os("CODEGRAPH_DISABLE_INFERENCE_CACHE")
        .is_some_and(|value| value == "1")
    {
        metrics::count("inference.cache_disabled", 1);
        boundary::resolve(graph, configuration)?;
        None
    } else {
        let mut cache = DecisionCache::open(project);
        boundary::resolve_cached(graph, configuration, &mut cache)?;
        Some(cache)
    };
    metrics::count(
        "relationships",
        graph.edges.len().saturating_sub(relationships) as u64,
    );
    Ok(cache)
}

/// Called only after successful input verification and atomic graph publication.
pub fn finish(cache: Option<DecisionCache>, progress: &dyn Fn(String)) {
    let Some(mut cache) = cache else {
        return;
    };
    let _stage = metrics::stage("inference.cache_commit");
    cache.commit();
    let stats = cache.stats();
    metrics::count("inference.cache_hits", stats.hits);
    metrics::count("inference.cache_misses", stats.misses);
    metrics::count("inference.cache_rejected", stats.rejected);
    progress(format!(
        "Inference cache: {} hits, {} recomputed, {} rejected",
        stats.hits, stats.misses, stats.rejected
    ));
    for warning in cache.warnings() {
        progress(format!("Inference cache warning: {warning}"));
    }
}
