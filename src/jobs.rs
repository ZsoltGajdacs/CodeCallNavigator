pub(crate) use crate::workspace::{Boot, LoadedGraph, data_directory, load_boot};
use crate::{
    app::{RESULT_PAGE_SIZE, State, TargetChoice},
    discovery, engine_client, fingerprint, projection,
};
use codecallnavigator::graph::{Direction, GraphIndex};
use rune::{Latest, Tx, Waker};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};

type Fallible<T> = Result<T, String>;
pub struct DiscoveryOutput {
    pub root: String,
    pub result: Fallible<Vec<discovery::BuildTarget>>,
}
pub struct NeighborOutput {
    pub epoch: u64,
    pub revision: u64,
    pub selected: u32,
    pub incoming: Vec<u32>,
    pub outgoing: Vec<u32>,
    pub relationships: Vec<u32>,
}

pub struct Jobs {
    pub boot: Latest<Fallible<Boot>>,
    pub source: Latest<crate::source_popup::SourceOutput>,
    pub discovery: Latest<DiscoveryOutput>,
    pub analysis: Latest<Fallible<LoadedGraph>>,
    pub cache_clear: Latest<crate::cache_jobs::Output>,
    pub layout: Latest<(u64, u64, Fallible<projection::GraphView>)>,
    pub search: Latest<crate::search::SearchOutput>,
    pub search_index: Latest<crate::search::IndexOutput>,
    pub neighbors: Latest<NeighborOutput>,
    pub stale: Latest<(u64, Fallible<bool>)>,
    pub persistence: Latest<Fallible<()>>,
    pub progress_sender: mpsc::Sender<String>,
    pub progress_receiver: mpsc::Receiver<String>,
    pub waker: Option<Waker>,
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
            search_index: Latest::default(),
            neighbors: Latest::default(),
            stale: Latest::default(),
            persistence: Latest::default().debounce(Duration::from_millis(300)),
            progress_sender,
            progress_receiver,
            waker: None,
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
    state.jobs.search_index.set_waker(waker.clone());
    state.jobs.neighbors.set_waker(waker.clone());
    state.jobs.stale.set_waker(waker.clone());
    state.jobs.persistence.set_waker(waker.clone());
    state.jobs.waker = Some(waker);
    if state.scene || state.deterministic {
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
                state.saved_builds.clear();
                state.saved_targets = None;
                state.reset_architecture();
                state.discover_root();
            }
            Ok(None) => {}
            Err(error) => state.warning = format!("Could not open the folder chooser: {error}"),
        });
    }

    pub fn discover_root(&mut self) {
        let root = PathBuf::from(self.root.text.trim());
        self.targets.clear();
        self.status = "Finding solutions, Java builds and TypeScript projects…".into();
        let runtime = self.runtime.clone();
        self.jobs.discovery.request(root, move |path| {
            let root = path.to_string_lossy().into_owned();
            DiscoveryOutput {
                root,
                result: (|| {
                    let runtime = match runtime {
                        Some(runtime) => runtime,
                        None => Arc::new(crate::runtime::Runtime::discover()?),
                    };
                    engine_client::discover(&runtime, &path)
                })()
                .map_err(|error: anyhow::Error| error.to_string()),
            }
        });
    }

    pub fn analyze_selected(&mut self) {
        if self.project_job_busy() {
            return;
        }
        let targets: Vec<_> = self
            .targets
            .iter()
            .filter(|choice| choice.selected)
            .map(|choice| choice.target.clone())
            .collect();
        let (Some(runtime), Some(store)) = (self.runtime.clone(), self.store.clone()) else {
            return;
        };
        if targets.is_empty() {
            self.warning =
                "Select at least one solution, project, Java build or TypeScript configuration."
                    .into();
            return;
        }
        self.cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let progress = self.jobs.progress_sender.clone();
        let wake = self.jobs.waker.clone();
        self.analyzing = true;
        self.warning.clear();
        self.status = if self.graph.is_some() {
            "Analyzing selected builds · showing the last successful graph"
        } else {
            "Preparing project analysis…"
        }
        .into();
        let project = PathBuf::from(self.root.text.trim());
        self.jobs.analysis.request(targets, move |targets| {
            let metrics = codecallnavigator::metrics::Metrics::default();
            let _scope = metrics.enter();
            let report = |message| {
                let _ = progress.send(message);
                if let Some(wake) = &wake {
                    wake();
                }
            };
            let result = (|| {
                let data = Arc::new(engine_client::analyze(
                    &runtime,
                    engine_client::Analysis { project, targets },
                    &cancel,
                    &report,
                )?);
                let graph = {
                    let _timer = codecallnavigator::metrics::stage("indexing");
                    Arc::new(GraphIndex::new(data.clone())?)
                };
                let generation = {
                    let _timer = codecallnavigator::metrics::stage("publication");
                    store.publish(&data, &cancel)?
                };
                Ok::<_, anyhow::Error>(LoadedGraph {
                    graph,
                    generation,
                    metrics: Some(metrics),
                })
            })();
            result.map_err(|error| format!("{error:#}"))
        });
    }

    pub fn request_layout(&mut self) {
        self.needs_layout = false;
        let (Some(graph), Some(request)) = (self.graph.clone(), self.request()) else {
            return;
        };
        if self.scene {
            let mut view = projection::project(&graph, &request);
            crate::scenes::position(&mut view);
            self.complete_graph_layout(view);
            return;
        }
        let Some(runtime) = &self.runtime else {
            return;
        };
        let dot = runtime.dot.clone();
        let epoch = self.epoch;
        let revision = self.revision;
        let first_layout = self.visible_revision.is_none();
        let metrics = first_layout
            .then(|| self.analysis_metrics.clone())
            .flatten();
        self.jobs.layout.request(request, move |request| {
            let _scope = metrics.as_ref().map(codecallnavigator::metrics::Metrics::enter);
            let _timer = codecallnavigator::metrics::stage(if first_layout {
                "initial_layout"
            } else {
                "layout"
            });
            let result = projection::layout(projection::project(&graph, &request), &dot)
                .map_err(|error| error.to_string());
            (epoch, revision, result)
        });
    }

    pub fn request_neighbors(&mut self) {
        self.neighbors = (Vec::new(), Vec::new());
        self.relationships.clear();
        let (Some(graph), Some(node)) = (self.graph.clone(), self.selected()) else {
            return;
        };
        let filter = self.nav.filter.clone();
        let epoch = self.epoch;
        let revision = self.revision;
        self.jobs
            .neighbors
            .request(node, move |node| NeighborOutput {
                epoch,
                revision,
                selected: node,
                incoming: graph.neighbors(node, Direction::Incoming, &filter),
                outgoing: graph.neighbors(node, Direction::Outgoing, &filter),
                relationships: graph.incident_edges(node, Direction::Both, &filter),
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
            store
                .save_session(&session)
                .map_err(|error| error.to_string())
        });
    }
}

pub fn poll(state: &mut State, time: f64, tx: &mut Tx) -> bool {
    let mut changed = crate::source_popup::poll(state, time, tx)
        | crate::graph_interaction::poll(state, time, tx)
        | poll_progress(state)
        | poll_boot(state)
        | poll_discovery(state)
        | poll_analysis(state)
        | crate::cache_jobs::poll(state)
        | poll_graph(state)
        | poll_persistence(state)
        | poll_staleness(state, time, tx)
        | crate::architecture_workspace::poll(state, time, tx)
        | crate::flow_workspace::poll(state, tx);
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

fn poll_discovery(state: &mut State) -> bool {
    let Some(output) = state.jobs.discovery.take() else {
        return false;
    };
    if output.root != state.root.text.trim() {
        return false;
    }
    match output.result {
        Ok(targets) => {
            let defaults = discovery::default_selection(&targets);
            state.targets = targets
                .into_iter()
                .map(|target| {
                    let selected = if let Some(saved) = &state.saved_targets {
                        saved.contains(&target.identity())
                    } else if state.saved_builds.is_empty() {
                        defaults.contains(&target.identity())
                    } else {
                        target.kind != "maven" && state.saved_builds.contains(&target.path)
                    };
                    TargetChoice { target, selected }
                })
                .collect();
            state.status = format!("{} build targets found", state.targets.len());
        }
        Err(error) => {
            state.warning = error;
            state.status = "Could not inspect this folder".into();
        }
    }
    true
}

fn poll_analysis(state: &mut State) -> bool {
    let Some(result) = state.jobs.analysis.take() else {
        return false;
    };
    state.analyzing = false;
    match result {
        Ok(graph) => {
            install_graph(state, graph);
            state.stale = false;
            state.auto_fit = true;
            state.changed();
        }
        Err(error) => {
            state.warning = error;
            state.status = "Analysis did not complete; the previous graph is preserved".into();
        }
    }
    true
}

fn install_graph(state: &mut State, loaded: LoadedGraph) {
    state.close_source();
    state.epoch += 1;
    state.nav.retain_known(&loaded.graph);
    state.graph = Some(loaded.graph);
    state.analysis_metrics = loaded.metrics;
    state.generation = Some(loaded.generation);
    state.next_stale_check = 0.0;
    state.architecture = None;
    state.architecture_workspace.invalidate();
    state.selected_edge = None;
    state.replace_graph_view(projection::GraphView::default());
    state.visible_revision = None;
    state.visual_selection = None;
    state.cancel_graph_press();
    state.status = format!(
        "{} symbols · {} relationships",
        state.graph.as_ref().unwrap().data.nodes.len(),
        state.graph.as_ref().unwrap().data.edges.len()
    );
    state.back.clear();
    state.forward.clear();
    state.needs_layout = true;
    state.request_neighbors();
}

fn poll_graph(state: &mut State) -> bool {
    poll_layout(state) | crate::search::poll(state) | poll_neighbors(state)
}

fn poll_layout(state: &mut State) -> bool {
    let Some((epoch, revision, result)) = state.jobs.layout.take() else {
        return false;
    };
    if epoch != state.epoch || revision != state.revision {
        return false;
    }
    if state.visible_revision.is_none()
        && let Some(metrics) = &state.analysis_metrics
    {
        metrics.finish();
    }
    match result {
        Ok(view) => {
            state.complete_graph_layout(view);
        }
        Err(error) => state.warning = error,
    }
    true
}

fn poll_neighbors(state: &mut State) -> bool {
    let Some(output) = state.jobs.neighbors.take() else {
        return false;
    };
    if output.epoch != state.epoch {
        return false;
    }
    if output.revision != state.revision {
        return false;
    }
    if Some(output.selected) != state.selected() {
        return false;
    }
    state.neighbors = (output.incoming, output.outgoing);
    state.relationships = output.relationships;
    state.neighbor_pages[0] =
        state.neighbor_pages[0].min(state.neighbors.0.len().saturating_sub(1) / RESULT_PAGE_SIZE);
    state.neighbor_pages[1] =
        state.neighbor_pages[1].min(state.neighbors.1.len().saturating_sub(1) / RESULT_PAGE_SIZE);
    state.relationship_page = state
        .relationship_page
        .min(state.relationships.len().saturating_sub(1) / RESULT_PAGE_SIZE);
    state.selected_edge = state
        .selected_edge
        .filter(|edge| state.relationships.contains(edge));
    true
}

fn poll_staleness(state: &mut State, time: f64, tx: &mut Tx) -> bool {
    let changed = poll_stale_result(state);
    if state.scene || state.closing {
        return changed;
    }
    if time >= state.next_stale_check {
        request_stale_check(state, time);
    }
    tx.wake_after(Duration::from_secs(30));
    changed
}

fn poll_stale_result(state: &mut State) -> bool {
    let Some((epoch, result)) = state.jobs.stale.take() else {
        return false;
    };
    if epoch != state.epoch {
        return false;
    }
    match result {
        Ok(stale) => state.stale = stale,
        Err(error) => {
            state.stale = true;
            state.warning = format!("Could not verify cached source files: {error}");
        }
    }
    true
}

fn request_stale_check(state: &mut State, time: f64) {
    state.next_stale_check = time + 30.0;
    let Some(graph) = &state.graph else {
        return;
    };
    let fingerprints = graph.data.fingerprints.clone();
    let epoch = state.epoch;
    state.jobs.stale.request(fingerprints, move |items| {
        (
            epoch,
            fingerprint::is_stale(&items).map_err(|error| error.to_string()),
        )
    });
}

pub fn close(state: &mut State, _: &mut Tx) {
    state.closing = true;
    state.cancel.store(true, Ordering::Release);
    state.save_session();
}
