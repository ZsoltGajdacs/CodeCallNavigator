//! Explicit scene/test adapter. Production query paths never select this backend implicitly.
use crate::{projection, source};
use codecallnavigator::{
    flow,
    graph::{Direction, Filter, GraphIndex},
    search::{NameSearchIndex, NameSearchQuery, SearchPage},
};

pub fn project(graph: &GraphIndex, request: &projection::Request) -> projection::GraphView {
    projection::project_prepared(
        graph,
        request,
        projection::fixture_selection(graph, request),
    )
}

pub fn selection(graph: &GraphIndex, request: &projection::Request) -> projection::Selection {
    projection::fixture_selection(graph, request)
}

pub fn search(graph: &GraphIndex, request: &NameSearchQuery<'_>) -> SearchPage {
    NameSearchIndex::new(&graph.data).search(request)
}

pub fn inspect(graph: &GraphIndex, node: u32, filter: &Filter) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    (
        graph.neighbors(node, Direction::Incoming, filter),
        graph.neighbors(node, Direction::Outgoing, filter),
        graph.incident_edges(node, Direction::Both, filter),
    )
}

pub fn flow(graph: &GraphIndex, symbol: &str) -> anyhow::Result<(flow::FlowChart, String, String)> {
    let function = flow::lookup(&graph.data, symbol).map_err(anyhow::Error::msg)?;
    let facts = graph
        .data
        .architecture
        .as_ref()
        .and_then(|facts| {
            facts.files.iter().find(|file| {
                file.file == function.file
                    && file.build == function.build
                    && file.project == function.project
            })
        })
        .ok_or_else(|| {
            anyhow::anyhow!("Refresh the graph to cache this function's analyzed source")
        })?;
    anyhow::ensure!(
        facts.content.len() as u64 <= source::MAX_SOURCE_BYTES,
        "The analyzed source exceeds the 16 MiB flow source limit"
    );
    anyhow::ensure!(
        function.span.start <= function.span.end
            && facts.content.is_char_boundary(function.span.start)
            && facts.content.is_char_boundary(function.span.end),
        "The flow source range does not match its cached snapshot"
    );
    Ok((
        flow::build(function)?,
        facts.content.clone(),
        function.file.clone(),
    ))
}

/// Saved-canvas benchmarking explicitly snapshots the legacy presentation cache.
pub fn cached_boot(directory: std::path::PathBuf) -> anyhow::Result<crate::workspace::Boot> {
    use std::sync::Arc;
    let store = Arc::new(codecallnavigator::store::SqliteStore::open(directory)?);
    let graph = store
        .active_generation()?
        .map(|generation| -> anyhow::Result<_> {
            Ok(crate::workspace::LoadedGraph {
                graph: Arc::new(GraphIndex::new(Arc::new(
                    store.load_generation(&generation)?,
                ))?),
                generation,
                context: None,
                presentation_cache: None,
                metrics: None,
            })
        })
        .transpose()?;
    let session = store
        .load_session()?
        .map(serde_json::from_value)
        .transpose()?;
    Ok(crate::workspace::Boot {
        store,
        graph,
        session,
        runtime: Arc::new(crate::runtime::Runtime::discover()?),
        warnings: Vec::new(),
    })
}

pub fn flow_supported(language: &str) -> bool {
    ["java", "typescript"].contains(&language)
}
