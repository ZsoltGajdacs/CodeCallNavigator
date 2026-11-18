use crate::{
    app::SavedSession,
    engine_client::{Descriptor, QueryContext, QueryFailure},
    runtime::Runtime as AnalyzerRuntime,
};
use codecallnavigator::{graph::GraphIndex, store::SqliteStore};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

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
    pub active_folder: Option<String>,
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
    let saved = store.load_session()?.unwrap_or(Value::Null);
    let mut warnings = Vec::new();
    let session = match serde_json::from_value::<Option<SavedSession>>(saved.clone()) {
        Ok(session) => session,
        Err(error) => {
            warnings.push(format!(
                "Saved navigation could not be restored; the graph is preserved: {error}"
            ));
            None
        }
    };
    let editable_root = saved.get("root").and_then(Value::as_str).map(str::to_owned);
    // The graph descriptor identifies the active project in legacy sessions whose
    // editable path was a draft. Read addressing separately from navigation.
    let active_folder = saved
        .get("active_folder")
        .and_then(Value::as_str)
        .or_else(|| saved.get("graph_context")?.get("project")?.as_str())
        .or(editable_root.as_deref())
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .map(str::to_owned);
    let runtime = Arc::new(AnalyzerRuntime::discover()?);
    let graph = match read_graph(&runtime, &saved, active_folder.as_deref()) {
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
        active_folder,
        runtime,
        warnings,
    })
}

fn read_graph(
    runtime: &AnalyzerRuntime,
    saved: &Value,
    active_folder: Option<&str>,
) -> anyhow::Result<Option<LoadedGraph>> {
    let descriptor = match saved.get("graph_context").filter(|value| !value.is_null()) {
        Some(context) => serde_json::from_value(context.clone())?,
        None => {
            let Some(folder) = active_folder else {
                return Ok(None);
            };
            Descriptor {
                project: folder.into(),
                roots: saved
                    .get("linked_roots")
                    .and_then(|roots| roots.get(crate::source_path::identity(folder)))
                    .cloned()
                    .map(serde_json::from_value)
                    .transpose()?
                    .unwrap_or_default(),
                ..Default::default()
            }
        }
    };
    load_saved_graph(runtime, descriptor, &AtomicBool::new(false))
}

pub(crate) fn load_saved_graph(
    runtime: &AnalyzerRuntime,
    mut descriptor: Descriptor,
    cancel: &AtomicBool,
) -> anyhow::Result<Option<LoadedGraph>> {
    // Opening restores the latest saved graph. Generation-specific reloads use
    // the original descriptor with its expected digest.
    descriptor.generation = None;
    match QueryContext::new(runtime, descriptor).snapshot(cancel) {
        Ok(snapshot) => Ok(Some(from_snapshot(snapshot))),
        Err(error)
            if error
                .downcast_ref::<QueryFailure>()
                .is_some_and(|failure| failure.0.code == "graph_missing") =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
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
