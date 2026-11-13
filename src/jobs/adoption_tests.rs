//! Pure desktop installation checks complement the real compiler/Rune journey.
use crate::{
    app::{Navigation, State},
    engine_client::{Descriptor, QueryContext},
    workspace::LoadedGraph,
};
use std::{path::PathBuf, sync::Arc};

fn context(project: &str, generation: &str) -> QueryContext {
    QueryContext::new(
        &crate::runtime::Runtime {
            engine: PathBuf::new(),
            resources: PathBuf::new(),
            dot: PathBuf::new(),
        },
        Descriptor {
            project: project.into(),
            roots: Vec::new(),
            generation: Some(generation.into()),
        },
    )
}

fn state() -> State {
    let mut state = State::fixture();
    crate::scenes::install(&mut state);
    state.query_context = Some(context("/fixture/project", "old"));
    state.generation = Some("old".into());
    state.nav.selected = Some("fixture/project/Start".into());
    state.nav.camera.pan = [117.0, -89.0];
    state.nav.camera.zoom = 0.63;
    state.back.push(state.nav.clone());
    state.forward.push(Navigation::default());
    state
}

fn replacement(state: &State, project: &str) -> LoadedGraph {
    LoadedGraph {
        navigation: Arc::new(crate::projection::NavigationInventory::new(
            state.graph.as_ref().unwrap(),
        )),
        graph: Arc::clone(state.graph.as_ref().unwrap()),
        generation: "new".into(),
        metrics: None,
        context: Some(context(project, "new")),
        presentation_cache: None,
    }
}

#[test]
fn same_project_generation_adoption_preserves_valid_history_and_camera() {
    let mut state = state();
    let loaded = replacement(&state, "/fixture/project");
    super::analysis::install_graph(&mut state, loaded);
    assert_eq!(
        state.back.len(),
        1,
        "Same-project refresh must preserve valid navigation history"
    );
    assert_eq!(state.forward.len(), 1);
    assert_eq!(state.nav.selected.as_deref(), Some("fixture/project/Start"));
    assert_eq!(state.nav.camera.pan, [117.0, -89.0]);
    assert_eq!(state.nav.camera.zoom, 0.63);
}

#[test]
fn linked_root_update_keeps_same_project_context_but_other_project_resets() {
    let mut state = state();
    let epoch = state.epoch;
    let mut loaded = replacement(&state, "/fixture/project");
    loaded
        .context
        .as_mut()
        .unwrap()
        .descriptor
        .roots
        .push(("shared".into(), "/updated/shared".into()));
    super::analysis::install_graph(&mut state, loaded);
    assert_eq!(state.back.len(), 1);
    assert!(state.epoch > epoch);
    assert_eq!(
        state.query_context.as_ref().unwrap().descriptor.roots[0].1,
        PathBuf::from("/updated/shared")
    );
    let loaded = replacement(&state, "/another/project");
    super::analysis::install_graph(&mut state, loaded);
    assert!(state.back.is_empty() && state.forward.is_empty());
    assert!(state.nav.selected.is_none() && state.nav.focus_target.is_none());
    assert!(state.auto_fit);
    assert_ne!(state.nav.camera.pan, [117.0, -89.0]);
}

#[test]
fn every_focus_kind_and_expansion_survives_only_while_its_presentation_identity_exists() {
    use crate::projection::FocusTarget;
    let mut state = state();
    let node = &state.graph.as_ref().unwrap().data.nodes[0];
    let project = serde_json::to_string(&["project", &node.build, &node.project]).unwrap();
    let class =
        crate::projection::class_key(&codecallnavigator::search::ClassIdentity::from_node(node));
    let file = crate::source_path::identity(&node.file);
    for target in [
        FocusTarget::Method(node.id.clone()),
        FocusTarget::Group(project.clone()),
        FocusTarget::GroupChildren(project.clone()),
        FocusTarget::ClassMethods(class.clone()),
        FocusTarget::FileClasses(file),
    ] {
        state.nav.focus = None;
        state.nav.focus_target = Some(target.clone());
        state.nav.expanded = [project.clone(), class.clone(), "removed-group".into()]
            .into_iter()
            .collect();
        let loaded = replacement(&state, "/fixture/project");
        super::analysis::install_graph(&mut state, loaded);
        assert_eq!(state.nav.focus_target.as_ref(), Some(&target));
        assert_eq!(state.nav.expanded.len(), 2);
    }
    let mut loaded = replacement(&state, "/fixture/project");
    loaded.graph =
        Arc::new(codecallnavigator::graph::GraphIndex::new(Arc::new(Default::default())).unwrap());
    loaded.navigation = Arc::new(crate::projection::NavigationInventory::new(&loaded.graph));
    super::analysis::install_graph(&mut state, loaded);
    assert!(state.nav.focus_target.is_none());
    assert!(state.nav.selected.is_none());
    assert!(state.nav.expanded.is_empty());
    assert!(
        state
            .back
            .iter()
            .all(|navigation| navigation.selected.is_none() && navigation.focus.is_none())
    );
    assert_eq!(state.nav.camera.pan, [117.0, -89.0]);
}

#[test]
fn late_focus_fallback_and_viewport_changes_do_not_override_adoption_camera() {
    let mut state = state();
    let loaded = replacement(&state, "/fixture/project");
    super::analysis::install_graph(&mut state, loaded);
    state.nav.focus_target = Some(crate::projection::FocusTarget::Method(
        "fixture/project/Start".into(),
    ));
    state.auto_fit = true;
    state.graph_interaction.center_after_layout = true;
    state.complete_graph_layout(crate::projection::GraphView::default());
    assert!(state.nav.focus_target.is_none());
    state.complete_graph_layout(crate::projection::GraphView::default());
    state.resize_graph_viewport([999.0, 777.0]);
    state.set_graph_filter_height(180.0);
    assert_eq!(state.nav.camera.pan, [117.0, -89.0]);
    assert_eq!(state.nav.camera.zoom, 0.63);
    assert!(!state.auto_fit && !state.graph_interaction.center_after_layout);
}
