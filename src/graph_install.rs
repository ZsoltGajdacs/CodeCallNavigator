//! Installing a validated generation is distinct from opening a different workspace.
use crate::{app::State, navigation::Navigation, projection, workspace::LoadedGraph};
use std::sync::atomic::Ordering;

#[derive(Clone, Copy)]
pub(crate) enum Mode {
    Restore,
    NewWorkspace,
    Adopt,
}

pub(crate) fn mode(state: &State, loaded: &LoadedGraph) -> Mode {
    match (&state.query_context, &loaded.context) {
        (Some(previous), Some(next)) if previous.descriptor.project == next.descriptor.project => {
            Mode::Adopt
        }
        _ => Mode::NewWorkspace,
    }
}

pub(crate) fn install(state: &mut State, loaded: LoadedGraph, mode: Mode) {
    reset_generation(state);
    match mode {
        Mode::NewWorkspace => reset_navigation(state),
        Mode::Adopt => {
            state.auto_fit = false;
        }
        Mode::Restore => (),
    }
    state.preserve_adoption_camera = !state.auto_fit;
    for navigation in std::iter::once(&mut state.nav)
        .chain(state.back.iter_mut())
        .chain(state.forward.iter_mut())
    {
        navigation.retain_with_inventory(&loaded.graph, &loaded.navigation);
    }
    state.flow_comparison.project_installed(
        loaded
            .context
            .as_ref()
            .map(|context| context.descriptor.project.as_path()),
    );
    state.query_context = loaded.context;
    state.remember_linked_roots();
    state.freshness.installed(
        state
            .query_context
            .as_ref()
            .map(|context| &context.descriptor),
    );
    state.presentation_cache = loaded.presentation_cache;
    state.graph = Some(loaded.graph);
    state.analysis_metrics = loaded.metrics;
    state.analysis_feedback.retention = loaded.retention;
    state.generation = Some(loaded.generation);
    let graph = state.graph.as_ref().expect("installed graph");
    state.status = format!(
        "{} symbols · {} relationships",
        graph.data.nodes.len(),
        graph.data.edges.len()
    );
    state.needs_layout = true;
    state.request_neighbors();
    crate::cache_retention::request(state, loaded.navigation);
}

pub(crate) fn clear(state: &mut State) {
    reset_generation(state);
    reset_navigation(state);
    state.preserve_adoption_camera = false;
    state.flow_comparison.project_installed(None);
    state.freshness.clear_project();
    state.query_context = None;
    state.presentation_cache = None;
    state.graph = None;
    state.analysis_metrics = None;
    state.analysis_feedback.retention = None;
    state.generation = None;
    state.needs_layout = false;
}

fn reset_generation(state: &mut State) {
    state.analysis_feedback.failure = None;
    state.flow_residency.retire(state.query_context.clone());
    invalidate(state);
    state.epoch += 1;
    state.revision += 1;
    state.architecture = None;
    state.selected_edge = None;
    state.replace_graph_view(projection::GraphView::default());
    state.visible_revision = None;
    state.visual_selection = None;
    state.graph_interaction.center_after_layout = false;
    state.dirty_session = true;
}

fn reset_navigation(state: &mut State) {
    state.nav = Navigation::default();
    state.back.clear();
    state.forward.clear();
    state.neighbors = (Vec::new(), Vec::new());
    state.neighbor_pages = [0, 0];
    state.relationships.clear();
    state.relationship_classifications.clear();
    state.relationship_page = 0;
    state.auto_fit = true;
}

fn invalidate(state: &mut State) {
    state.close_source();
    state.jobs.source.cancel();
    state.close_search();
    state.jobs.search.cancel();
    state.jobs.search_activation.cancel();
    state.flow_workspace.invalidate();
    state.architecture_workspace.invalidate();
    state.jobs.layout_cancel.store(true, Ordering::Release);
    state.jobs.layout.cancel();
    state.jobs.neighbors_cancel.store(true, Ordering::Release);
    state.jobs.neighbors.cancel();
    state.jobs.file_focus_cancel.store(true, Ordering::Release);
    state.jobs.file_focus.cancel();
    state.cancel_graph_press();
}
