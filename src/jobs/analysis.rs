//! Analysis installs CLI snapshots and retains their derived presentation cache.
use crate::{app::State, engine_client, workspace::LoadedGraph};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Purpose {
    #[default]
    MainGraph,
    CompareWorktree(crate::flow_comparison::RefreshTicket),
}

impl Purpose {
    pub(crate) fn failed(self, state: &mut State, message: String) {
        if let Self::CompareWorktree(ticket) = self {
            state.comparison_worktree_analysis_failed(ticket, message);
        }
    }
}

pub struct Output {
    purpose: Purpose,
    binding: crate::workspace::Binding,
    selection: crate::analysis_feedback::Selection,
    result: anyhow::Result<LoadedGraph>,
}

impl State {
    pub fn analyze_selected(&mut self) {
        self.analyze_selected_for(Purpose::MainGraph);
    }

    pub(crate) fn analyze_selected_for(&mut self, purpose: Purpose) {
        if self.analysis_job_busy() {
            purpose.failed(self, "An analysis is already running".into());
            return;
        }
        let Some(selection) = self.selected_analysis() else {
            self.warning =
                "Select at least one solution, project, Java build or TypeScript configuration."
                    .into();
            purpose.failed(self, self.warning.clone());
            return;
        };
        if let Purpose::CompareWorktree(ticket) = purpose
            && !self.start_comparison_worktree_analysis(ticket)
        {
            return;
        }
        self.analysis_feedback.failure = None;
        if self.defer_manual_analysis_for(&selection, purpose) {
            return;
        }
        self.start_selected_analysis(selection, purpose);
    }

    pub(crate) fn selected_analysis(&self) -> Option<engine_client::Analysis> {
        let project = PathBuf::from(self.project_open.folder.as_deref()?);
        let targets: Vec<_> = self.selected_builds().cloned().collect();
        (!targets.is_empty()).then_some(engine_client::Analysis { project, targets })
    }

    pub(crate) fn selected_builds(&self) -> impl Iterator<Item = &crate::discovery::BuildTarget> {
        self.targets
            .iter()
            .filter(|choice| choice.selected)
            .map(|choice| &choice.target)
    }

    pub(crate) fn start_selected_analysis(
        &mut self,
        selection: engine_client::Analysis,
        purpose: Purpose,
    ) {
        let Some(runtime) = self.runtime.clone() else {
            purpose.failed(self, "The standalone CLI is not ready".into());
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
            let selection_binding = crate::analysis_feedback::Selection::for_analysis(&selection);
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
                purpose,
                binding,
                selection: selection_binding,
                result,
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
        output
            .purpose
            .failed(state, "The active project changed during analysis".into());
        return true;
    }
    match output.result {
        Ok(graph) => {
            let generation = graph.generation.clone();
            install_graph(state, graph);
            if let Purpose::CompareWorktree(ticket) = output.purpose {
                let project = state
                    .query_context
                    .as_ref()
                    .map(|context| context.descriptor.project.clone());
                state.comparison_worktree_published(ticket, project.as_deref(), generation);
            }
        }
        Err(error) => report_analysis_failure(state, output.purpose, output.selection, error),
    }
    true
}

fn report_analysis_failure(
    state: &mut State,
    purpose: Purpose,
    selection: crate::analysis_feedback::Selection,
    error: anyhow::Error,
) {
    if !selection.matches(state) {
        purpose.failed(state, "Build selection changed during analysis".into());
        return;
    }
    let failure = error
        .downcast::<engine_client::AnalysisFailure>()
        .unwrap_or_else(|error| engine_client::AnalysisFailure {
            code: None,
            message: format!("{error:#}"),
            target: None,
        });
    if state.cancel.load(std::sync::atomic::Ordering::Acquire) {
        state.warning = failure.to_string();
        state.status = "Analysis cancelled".into();
        purpose.failed(state, state.status.clone());
        return;
    }
    purpose.failed(state, failure.to_string());
    state.analysis_feedback.failure = Some(crate::analysis_feedback::FailedSelection {
        selection,
        error: failure,
    });
    state.status = if state.graph.is_some() {
        "Analysis did not complete; the previous graph is preserved"
    } else {
        "Analysis did not complete"
    }
    .into();
}

pub(super) fn install_graph(state: &mut State, loaded: LoadedGraph) {
    let mode = crate::graph_install::mode(state, &loaded);
    crate::graph_install::install(state, loaded, mode);
}
