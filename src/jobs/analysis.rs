//! Analysis installs CLI snapshots and retains their derived presentation cache.
use crate::{app::State, engine_client, projection, workspace::LoadedGraph};
use std::{path::PathBuf, sync::Arc};

impl State {
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
        let cache_lock = self.jobs.save_lock.clone();
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
                let snapshot = engine_client::analyze_snapshot(
                    &runtime,
                    engine_client::Analysis { project, targets },
                    &cancel,
                    &report,
                )?;
                let mut loaded = {
                    let _guard = cache_lock
                        .lock()
                        .map_err(|_| anyhow::anyhow!("Presentation cache writer failed"))?;
                    crate::workspace::retain_snapshot(&store, snapshot, &cancel)?
                };
                loaded.metrics = Some(metrics);
                Ok::<_, anyhow::Error>(loaded)
            })();
            result.map_err(|error| format!("{error:#}"))
        });
    }
}

pub(super) fn poll_analysis(state: &mut State) -> bool {
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

pub(super) fn install_graph(state: &mut State, loaded: LoadedGraph) {
    state.close_source();
    state.epoch += 1;
    state.nav.retain_known(&loaded.graph);
    state.query_context = loaded.context;
    state.presentation_cache = loaded.presentation_cache;
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
