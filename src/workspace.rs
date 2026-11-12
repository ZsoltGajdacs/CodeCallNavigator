use crate::{
    app::SavedSession,
    engine_client::{Descriptor, QueryContext},
    runtime::Runtime as AnalyzerRuntime,
};
use codecallnavigator::{graph::GraphIndex, store::SqliteStore};
use std::{path::PathBuf, sync::Arc};

pub struct LoadedGraph {
    pub graph: Arc<GraphIndex>,
    pub generation: String,
    pub metrics: Option<codecallnavigator::metrics::Metrics>,
    pub context: Option<QueryContext>,
    pub presentation_cache: Option<crate::presentation_cache::PresentationCache>,
}
pub struct Boot {
    pub store: Arc<SqliteStore>,
    pub graph: Option<LoadedGraph>,
    pub session: Option<SavedSession>,
    pub runtime: Arc<AnalyzerRuntime>,
    pub warnings: Vec<String>,
}

pub fn data_directory() -> PathBuf {
    std::env::var_os("CODECALLNAVIGATOR_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join("Library/Application Support/CodeCallNavigator")
        })
}

pub(crate) fn load_boot(directory: PathBuf) -> anyhow::Result<Boot> {
    let store = Arc::new(SqliteStore::open(directory)?);
    let mut warnings = Vec::new();
    let session = match read_session(&store) {
        Ok(session) => session,
        Err(error) => {
            warnings.push(format!(
                "Saved navigation could not be restored; the graph is preserved: {error}"
            ));
            None
        }
    };
    let runtime = Arc::new(AnalyzerRuntime::discover()?);
    let graph = match read_graph(&runtime, &store) {
        Ok(graph) => graph,
        Err(error) => {
            warnings.push(format!(
                "The cached graph could not be opened. Choose a folder to rebuild it: {error}"
            ));
            None
        }
    };
    Ok(Boot {
        store,
        graph,
        session,
        runtime,
        warnings,
    })
}

fn read_session(store: &SqliteStore) -> anyhow::Result<Option<SavedSession>> {
    store
        .load_session()?
        .map(serde_json::from_value)
        .transpose()
        .map_err(Into::into)
}

fn saved_descriptor(store: &SqliteStore) -> anyhow::Result<Option<Descriptor>> {
    let Some(saved) = store.load_session()? else {
        return Ok(None);
    };
    if let Some(context) = saved.get("graph_context").filter(|value| !value.is_null()) {
        return Ok(Some(serde_json::from_value(context.clone())?));
    }
    Ok(saved
        .get("root")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|root| !root.is_empty())
        .map(|root| Descriptor {
            project: root.into(),
            ..Default::default()
        }))
}

fn read_graph(
    runtime: &AnalyzerRuntime,
    store: &SqliteStore,
) -> anyhow::Result<Option<LoadedGraph>> {
    // Addressing survives corrupt navigation. A graph descriptor is preferred
    // over the editable legacy root, which may refer to another pending project.
    let descriptor = saved_descriptor(store)?;
    let Some(mut descriptor) = descriptor else {
        return Ok(None);
    };
    // Restore the latest published graph for the saved project; persisted navigation
    // is retained only for symbols present in this returned immutable generation.
    descriptor.generation = None;
    let snapshot = QueryContext::new(runtime, descriptor)
        .snapshot(&std::sync::atomic::AtomicBool::new(false))?;
    Ok(Some(retain_snapshot(
        store,
        snapshot,
        &std::sync::atomic::AtomicBool::new(false),
    )?))
}

pub(crate) fn retain_snapshot(
    store: &SqliteStore,
    snapshot: crate::engine_client::Snapshot,
    cancel: &std::sync::atomic::AtomicBool,
) -> anyhow::Result<LoadedGraph> {
    let presentation_cache = crate::presentation_cache::retain(store, &snapshot, cancel)?;
    let generation = snapshot
        .context
        .descriptor
        .generation
        .clone()
        .expect("validated snapshot generation");
    Ok(LoadedGraph {
        graph: snapshot.graph,
        generation,
        context: Some(snapshot.context),
        presentation_cache: Some(presentation_cache),
        metrics: None,
    })
}

/// Retain opaque recovered settings while typed navigation becomes usable again.
pub(crate) fn save_session(
    store: &SqliteStore,
    mut session: serde_json::Value,
) -> anyhow::Result<()> {
    if let Some(recovered) = store
        .load_session()?
        .and_then(|saved| saved.get("recovered_session").cloned())
    {
        session["recovered_session"] = recovered;
    }
    store.save_session(&session)
}
