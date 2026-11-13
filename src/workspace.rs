use crate::{
    app::SavedSession,
    engine_client::{Descriptor, QueryContext},
    runtime::Runtime as AnalyzerRuntime,
};
use codecallnavigator::{graph::GraphIndex, store::SqliteStore};
use std::{path::PathBuf, sync::Arc};

/// Identity of the currently displayed workspace, independent of the folder editor.
#[derive(Clone)]
pub(crate) struct Binding {
    epoch: u64,
    context: Option<Descriptor>,
}

impl Binding {
    pub fn capture(state: &crate::app::State) -> Self {
        Self {
            epoch: state.epoch,
            context: state
                .query_context
                .as_ref()
                .map(|context| context.descriptor.clone()),
        }
    }
    pub fn matches(&self, state: &crate::app::State) -> bool {
        self.epoch == state.epoch
            && self.context.as_ref()
                == state
                    .query_context
                    .as_ref()
                    .map(|context| &context.descriptor)
    }
}

pub struct LoadedGraph {
    pub navigation: Arc<crate::projection::NavigationInventory>,
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
    pub editable_root: Option<String>,
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
    let editable_root = store
        .load_session()?
        .and_then(|saved| saved.get("root")?.as_str().map(str::to_owned));
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
        editable_root,
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
    Ok(Some(from_snapshot(snapshot)))
}

pub(crate) fn from_snapshot(snapshot: crate::engine_client::Snapshot) -> LoadedGraph {
    let generation = snapshot
        .context
        .descriptor
        .generation
        .clone()
        .expect("validated snapshot generation");
    LoadedGraph {
        navigation: snapshot.navigation,
        graph: snapshot.graph,
        generation,
        context: Some(snapshot.context),
        presentation_cache: None,
        metrics: None,
    }
}

/// Retain opaque recovered settings while typed navigation becomes usable again.
pub(crate) fn save_session(
    store: &SqliteStore,
    mut session: serde_json::Value,
) -> anyhow::Result<()> {
    let mut saved = store
        .load_session()?
        .unwrap_or_else(|| serde_json::json!({}));
    if !saved.is_object() {
        saved = serde_json::json!({"recovered_session": saved});
    }
    if serde_json::from_value::<SavedSession>(saved.clone()).is_err()
        && saved.get("recovered_session").is_none()
    {
        saved["recovered_session"] = saved.clone();
    }
    if let Some(values) = session.as_object_mut() {
        saved
            .as_object_mut()
            .expect("normalized settings object")
            .append(values);
    }
    store.save_session(&saved)
}
