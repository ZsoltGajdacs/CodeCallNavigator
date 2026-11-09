use crate::{analyzers::AnalyzerRuntime, app::SavedSession};
use codegraph::{graph::GraphIndex, store::SqliteStore};
use std::{path::PathBuf, sync::Arc};

pub struct LoadedGraph {
    pub graph: Arc<GraphIndex>,
    pub generation: String,
    pub metrics: Option<codegraph::metrics::Metrics>,
}
pub struct Boot {
    pub store: Arc<SqliteStore>,
    pub graph: Option<LoadedGraph>,
    pub session: Option<SavedSession>,
    pub runtime: Arc<AnalyzerRuntime>,
    pub warnings: Vec<String>,
}

pub fn data_directory() -> PathBuf {
    std::env::var_os("CODEGRAPH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join("Library/Application Support/CodeGraph")
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
    let graph = match read_cached_graph(&store) {
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
        runtime: Arc::new(AnalyzerRuntime::discover()?),
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

fn read_cached_graph(store: &SqliteStore) -> anyhow::Result<Option<LoadedGraph>> {
    store
        .active_generation()?
        .map(|generation| {
            let graph = Arc::new(GraphIndex::new(Arc::new(
                store.load_generation(&generation)?,
            ))?);
            Ok(LoadedGraph {
                graph,
                generation,
                metrics: None,
            })
        })
        .transpose()
}
