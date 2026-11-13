//! Generation-bound graph query jobs and presentation-result installation.
use crate::{
    app::{RESULT_PAGE_SIZE, State},
    projection,
};
use std::sync::{Arc, atomic::Ordering};

pub struct NeighborOutput {
    pub epoch: u64,
    pub revision: u64,
    pub selected: u32,
    pub result: Result<crate::graph_queries::Inspection, String>,
}

impl State {
    pub fn request_layout(&mut self) {
        self.needs_layout = false;
        let (Some(graph), Some(request)) = (self.graph.clone(), self.request()) else {
            return;
        };
        if self.scene {
            let mut view = crate::fixture_queries::project(&graph, &request);
            crate::scenes::position(&mut view);
            self.complete_graph_layout(view);
            return;
        }
        let Some(runtime) = &self.runtime else {
            return;
        };
        let dot = runtime.dot.clone();
        let queries = match self.graph_queries() {
            Ok(queries) => queries,
            Err(error) => {
                self.warning = error;
                return;
            }
        };
        self.jobs.layout_cancel.store(true, Ordering::Release);
        self.jobs.layout_cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel = self.jobs.layout_cancel.clone();
        let epoch = self.epoch;
        let revision = self.revision;
        let first_layout = self.visible_revision.is_none();
        let metrics = first_layout
            .then(|| self.analysis_metrics.clone())
            .flatten();
        self.jobs.layout.request(request, move |request| {
            let _scope = metrics
                .as_ref()
                .map(codecallnavigator::metrics::Metrics::enter);
            let _timer = codecallnavigator::metrics::stage(if first_layout {
                "initial_layout"
            } else {
                "layout"
            });
            let result = queries
                .render(&request, &cancel)
                .and_then(|selection| {
                    projection::layout(
                        projection::project_prepared(&graph, &request, selection),
                        &dot,
                    )
                })
                .map_err(|error| format!("{error:#}"));
            (epoch, revision, result)
        });
    }

    pub fn request_neighbors(&mut self) {
        self.neighbors = (Vec::new(), Vec::new());
        self.relationships.clear();
        self.relationship_classifications.clear();
        let Some(node) = self.selected() else {
            return;
        };
        let filter = self.nav.filter.clone();
        let epoch = self.epoch;
        let revision = self.revision;
        let queries = match self.graph_queries() {
            Ok(queries) => queries,
            Err(error) => {
                self.warning = error;
                return;
            }
        };
        self.jobs.neighbors_cancel.store(true, Ordering::Release);
        self.jobs.neighbors_cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel = self.jobs.neighbors_cancel.clone();
        self.jobs
            .neighbors
            .request(node, move |node| NeighborOutput {
                epoch,
                revision,
                selected: node,
                result: queries
                    .inspect(node, &filter, &cancel)
                    .map_err(|error| format!("{error:#}")),
            });
    }
}

pub(super) fn poll_graph(state: &mut State) -> bool {
    poll_layout(state)
        | crate::search::poll(state)
        | poll_neighbors(state)
        | crate::graph_interaction::poll_file_focus(state)
}

fn poll_layout(state: &mut State) -> bool {
    let Some((epoch, revision, result)) = state.jobs.layout.take() else {
        return false;
    };
    if epoch != state.epoch || revision != state.revision {
        return false;
    }
    finish_initial_metrics(state);
    match result {
        Ok(view) => complete_layout(state, view),
        Err(error) => layout_error(state, error),
    }
    true
}

fn complete_layout(state: &mut State, view: projection::GraphView) {
    if ["architecture_pending:", "architecture_changed:"]
        .iter()
        .any(|prefix| state.warning.starts_with(prefix))
    {
        state.warning.clear();
    }
    state.complete_graph_layout(view);
}

fn finish_initial_metrics(state: &State) {
    if state.visible_revision.is_some() {
        return;
    }
    if let Some(metrics) = &state.analysis_metrics {
        metrics.finish();
    }
}

fn layout_error(state: &mut State, error: String) {
    if error.contains("architecture_changed") {
        state.architecture_workspace.invalidate();
    }
    state.warning = error;
}

fn poll_neighbors(state: &mut State) -> bool {
    let Some(output) = state.jobs.neighbors.take() else {
        return false;
    };
    if (output.epoch, output.revision, Some(output.selected))
        != (state.epoch, state.revision, state.selected())
    {
        return false;
    }
    let result = match output.result {
        Ok(result) => result,
        Err(error) => {
            state.warning = error;
            return true;
        }
    };
    state.neighbors = (result.incoming, result.outgoing);
    state.relationships = result.relationships;
    state.relationship_classifications = result.classifications;
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
