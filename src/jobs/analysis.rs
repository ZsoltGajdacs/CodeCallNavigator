//! Analysis installs CLI snapshots and retains their derived presentation cache.
use crate::{app::State, engine_client, workspace::LoadedGraph};
use std::{path::PathBuf, sync::Arc};

pub struct Output {
    binding: crate::workspace::Binding,
    result: Result<LoadedGraph, String>,
}

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
        if targets.is_empty() {
            self.warning =
                "Select at least one solution, project, Java build or TypeScript configuration."
                    .into();
            return;
        }
        let selection = engine_client::Analysis {
            project: PathBuf::from(self.root.text.trim()),
            targets,
        };
        if self.defer_manual_analysis(&selection) {
            return;
        }
        self.start_selected_analysis(selection);
    }

    pub(crate) fn start_selected_analysis(&mut self, selection: engine_client::Analysis) {
        let Some(runtime) = self.runtime.clone() else {
            return;
        };
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
        let epoch = self.epoch;
        let binding = crate::workspace::Binding::capture(self);
        self.jobs.analysis.request(selection, move |selection| {
            let metrics = codecallnavigator::metrics::Metrics::default();
            let _scope = metrics.enter();
            let report = |message| {
                let _ = progress.send((epoch, message));
                if let Some(wake) = &wake {
                    wake();
                }
            };
            let result = (|| {
                let snapshot =
                    engine_client::analyze_snapshot(&runtime, selection, &cancel, &report)?;
                let mut loaded = crate::workspace::from_snapshot(snapshot);
                loaded.metrics = Some(metrics);
                Ok::<_, anyhow::Error>(loaded)
            })();
            Output {
                binding,
                result: result.map_err(|error| format!("{error:#}")),
            }
        });
    }
}

pub(super) fn poll_analysis(state: &mut State) -> bool {
    let Some(output) = state.jobs.analysis.take() else {
        return false;
    };
    state.analyzing = false;
    if state.closing || !output.binding.matches(state) {
        return true;
    }
    match output.result {
        Ok(graph) => {
            install_graph(state, graph);
        }
        Err(error) => {
            state.warning = error;
            state.status = "Analysis did not complete; the previous graph is preserved".into();
        }
    }
    true
}

pub(super) fn install_graph(state: &mut State, loaded: LoadedGraph) {
    let mode = crate::graph_install::mode(state, &loaded);
    crate::graph_install::install(state, loaded, mode);
}
