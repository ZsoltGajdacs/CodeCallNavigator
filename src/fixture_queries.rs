//! Explicit scene/test adapter. Production query paths never select this backend implicitly.
use crate::{app::State, projection};
use codecallnavigator::{
    flow,
    graph::{Direction, Filter, GraphIndex},
    search::{NameSearchIndex, NameSearchQuery, SearchPage},
};
use std::sync::Arc;

/// Install a complete explicit fixture, keeping its inventory paired with its
/// structure. Callers retain ownership of navigation and pending-work policy.
pub fn install(
    state: &mut State,
    mut graph: codecallnavigator::model::GraphData,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        state.uses_fixture_queries(),
        "Fixture installation requires an explicit fixture backend"
    );
    graph.validate()?;
    let inventory = graph.flow_inventory.take().map(Arc::new);
    let structure = Arc::new(GraphIndex::new(Arc::new(graph))?);
    state.fixture_flow_inventory = inventory;
    state.graph = Some(structure);
    Ok(())
}

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

pub fn flow(
    graph: &GraphIndex,
    inventory: Option<&flow::FlowInventory>,
    symbol: &str,
) -> anyhow::Result<(flow::FlowChart, String, String)> {
    use codecallnavigator::artifact::flow::{Selection, View};
    let view = View::from_parts(&graph.data, inventory)?;
    match view.select(symbol) {
        Selection::Ready(ready) => Ok((
            ready.chart,
            ready.content.into(),
            ready.node.owner.file.into(),
        )),
        Selection::Missing => anyhow::bail!("The selected method is no longer in this graph."),
        Selection::Unavailable(error) => Err(error.into()),
    }
}

/// Saved-canvas benchmarking explicitly snapshots the legacy presentation cache.
pub fn cached_boot(directory: std::path::PathBuf) -> anyhow::Result<crate::workspace::Boot> {
    use std::sync::Arc;
    let store = Arc::new(codecallnavigator::store::SqliteStore::open(directory)?);
    let graph = store
        .active_generation()?
        .map(|generation| -> anyhow::Result<_> {
            let graph = Arc::new(GraphIndex::new(Arc::new(
                store.load_generation(&generation)?,
            ))?);
            Ok(crate::workspace::LoadedGraph {
                navigation: Arc::new(projection::NavigationInventory::new(&graph)),
                graph,
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
        editable_root: None,
        runtime: Arc::new(crate::runtime::Runtime::discover()?),
        warnings: Vec::new(),
    })
}

pub fn flow_supported(language: &str) -> bool {
    ["java", "typescript"].contains(&language)
}
