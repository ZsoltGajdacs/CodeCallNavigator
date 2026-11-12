mod analysis;
mod queries;
pub(crate) use crate::workspace::{Boot, LoadedGraph, data_directory, load_boot};
use crate::{app::State, projection};
use analysis::{install_graph, poll_analysis};
pub use queries::NeighborOutput;
use queries::{poll_graph, poll_staleness};
use rune::{Latest, Tx, Waker};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

type Fallible<T> = Result<T, String>;

pub struct Jobs {
    pub boot: Latest<Fallible<Boot>>,
    pub source: Latest<crate::source_popup::SourceOutput>,
    pub discovery: Latest<crate::build_discovery::Output>,
    pub analysis: Latest<Fallible<LoadedGraph>>,
    pub cache_clear: Latest<crate::cache_jobs::Output>,
    pub layout: Latest<(u64, u64, Fallible<projection::GraphView>)>,
    pub search: Latest<crate::search::SearchOutput>,
    pub search_activation: Latest<crate::search::ActivationOutput>,
    pub neighbors: Latest<NeighborOutput>,
    pub file_focus: Latest<crate::graph_interaction::FileFocusOutput>,
    pub file_focus_cancel: Arc<std::sync::atomic::AtomicBool>,
    pub stale: Latest<(u64, Fallible<bool>)>,
    pub persistence: Latest<Fallible<()>>,
    pub progress_sender: mpsc::Sender<String>,
    pub progress_receiver: mpsc::Receiver<String>,
    pub waker: Option<Waker>,
    pub layout_cancel: Arc<std::sync::atomic::AtomicBool>,
    pub neighbors_cancel: Arc<std::sync::atomic::AtomicBool>,
    pub stale_cancel: Arc<std::sync::atomic::AtomicBool>,
    save_revision: Arc<AtomicU64>,
    save_lock: Arc<Mutex<()>>,
}

impl Default for Jobs {
    fn default() -> Self {
        let (progress_sender, progress_receiver) = mpsc::channel();
        Self {
            boot: Latest::default(),
            source: Latest::default(),
            discovery: Latest::default(),
            analysis: Latest::default(),
            cache_clear: Latest::default(),
            layout: Latest::default().debounce(Duration::from_millis(50)),
            search: Latest::default().debounce(Duration::from_millis(100)),
            search_activation: Latest::default(),
            neighbors: Latest::default(),
            file_focus: Latest::default(),
            file_focus_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            stale: Latest::default(),
            persistence: Latest::default().debounce(Duration::from_millis(300)),
            progress_sender,
            progress_receiver,
            waker: None,
            layout_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            neighbors_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            stale_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            save_revision: Arc::new(AtomicU64::new(0)),
            save_lock: Arc::new(Mutex::new(())),
        }
    }
}

pub fn setup(state: &mut State, waker: Waker) {
    state.jobs.boot.set_waker(waker.clone());
    state.jobs.source.set_waker(waker.clone());
    state.flow_workspace.job.set_waker(waker.clone());
    state.architecture_workspace.job.set_waker(waker.clone());
    state.jobs.discovery.set_waker(waker.clone());
    state.jobs.analysis.set_waker(waker.clone());
    state.jobs.cache_clear.set_waker(waker.clone());
    state.jobs.layout.set_waker(waker.clone());
    state.jobs.search.set_waker(waker.clone());
    state.jobs.search_activation.set_waker(waker.clone());
    state.jobs.neighbors.set_waker(waker.clone());
    state.jobs.file_focus.set_waker(waker.clone());
    state.jobs.stale.set_waker(waker.clone());
    state.jobs.persistence.set_waker(waker.clone());
    state.jobs.waker = Some(waker);
    if state.uses_fixture_queries() {
        return;
    }
    state.status = "Opening saved workspace…".into();
    state.jobs.boot.request(data_directory(), |directory| {
        load_boot(directory).map_err(|error| error.to_string())
    });
}

impl State {
    pub fn choose_folder(&mut self, tx: &mut Tx) {
        let options = rune::FileDialogOptions {
            title: "Choose a project folder".into(),
            ..Default::default()
        };
        tx.open_folder(options, |state: &mut State, answer, _| match answer {
            Ok(Some(path)) => {
                state.root = rune::TextEdit::new(path.to_string_lossy());
                state.root_edited();
                state.discover_root();
            }
            Ok(None) => {}
            Err(error) => state.warning = format!("Could not open the folder chooser: {error}"),
        });
    }

    pub fn save_session(&mut self) {
        self.dirty_session = false;
        if self.scene {
            return;
        }
        let Some(store) = self.store.clone() else {
            return;
        };
        let Ok(session) = serde_json::to_value(self.session()) else {
            return;
        };
        let latest = self.jobs.save_revision.clone();
        let revision = latest.fetch_add(1, Ordering::AcqRel) + 1;
        let lock = self.jobs.save_lock.clone();
        self.jobs.persistence.request(session, move |session| {
            let _guard = lock
                .lock()
                .map_err(|_| "Session writer failed".to_string())?;
            if latest.load(Ordering::Acquire) != revision {
                return Ok(());
            }
            crate::workspace::save_session(&store, session).map_err(|error| error.to_string())
        });
    }
}

pub fn poll(state: &mut State, time: f64, tx: &mut Tx) -> bool {
    let mut changed = crate::source_popup::poll(state, time, tx)
        | crate::graph_interaction::poll(state, time, tx)
        | poll_progress(state)
        | poll_boot(state)
        | crate::build_discovery::poll(state)
        | poll_analysis(state)
        | crate::cache_jobs::poll(state)
        | poll_graph(state)
        | poll_persistence(state)
        | poll_staleness(state, time, tx)
        | crate::architecture_workspace::poll(state, time, tx)
        | crate::flow_workspace::poll(state, tx);
    changed |= crate::explorer::poll(state, time, tx);
    changed |= state.constrain_panes(state.ui.window_width);
    if state.needs_layout {
        state.request_layout();
        changed = true;
    }
    if state.dirty_session {
        state.save_session();
    }
    if ready_to_quit(state) {
        tx.quit();
    }
    changed
}

fn poll_progress(state: &mut State) -> bool {
    let mut changed = false;
    for message in state.jobs.progress_receiver.try_iter() {
        state.status = message;
        changed = true;
    }
    changed
}

fn poll_persistence(state: &mut State) -> bool {
    let Some(result) = state.jobs.persistence.take() else {
        return false;
    };
    if let Err(error) = result {
        state.warning = format!("Could not save navigation state: {error}");
    }
    true
}

fn ready_to_quit(state: &State) -> bool {
    state.closing
        && !state.jobs.analysis.is_pending()
        && !state.jobs.cache_clear.is_pending()
        && !state.jobs.persistence.is_pending()
}

fn poll_boot(state: &mut State) -> bool {
    let Some(result) = state.jobs.boot.take() else {
        return false;
    };
    match result {
        Ok(boot) => {
            if !boot.warnings.is_empty() {
                state.warning = boot.warnings.join("\n");
            }
            state.store = Some(boot.store);
            state.runtime = Some(boot.runtime);
            if let Some(session) = boot.session.filter(|_| !state.root_was_edited) {
                state.root = rune::TextEdit::new(session.root);
                state.saved_builds = session.selected_builds;
                state.saved_targets = session.selected_targets;
                state.nav = session.navigation;
                state.architecture_selection = session.architecture_selection;
                state.ui.restore(session.presentation);
                state.auto_fit = false;
            }
            if let Some(graph) = boot.graph {
                install_graph(state, graph);
            } else {
                state.nav.selected = None;
                state.nav.focus = None;
                state.nav.focus_target = None;
                state.nav.path_from = None;
                state.nav.path_to = None;
                state.nav.show_path = false;
                state.status = "Choose a project folder to begin".into();
            }
            if !state.root_was_edited && !state.root.text.is_empty() {
                state.discover_root();
            }
        }
        Err(error) => {
            state.warning = error;
            state.status = "Could not open the saved workspace".into();
        }
    }
    true
}

pub fn close(state: &mut State, _: &mut Tx) {
    state.closing = true;
    state.cancel.store(true, Ordering::Release);
    state.jobs.layout_cancel.store(true, Ordering::Release);
    state.jobs.neighbors_cancel.store(true, Ordering::Release);
    state.jobs.stale_cancel.store(true, Ordering::Release);
    state.jobs.file_focus_cancel.store(true, Ordering::Release);
    state.close_source();
    state.close_search();
    state.close_flow();
    state.save_session();
}
