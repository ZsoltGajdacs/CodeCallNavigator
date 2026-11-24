//! Loaded project ownership and manual correspondence remain independent of graph adoption.
use super::tests::{project, selected, settle, state};
use super::*;
use crate::{
    engine_client::{Descriptor, QueryContext},
    workspace::LoadedGraph,
};
use codecallnavigator::graph::GraphIndex;
fn loaded(state: &crate::app::State, root: &std::path::Path) -> LoadedGraph {
    let graph = Arc::new(GraphIndex::new(Arc::new(crate::flow_scenes::data())).unwrap());
    LoadedGraph {
        navigation: Arc::new(crate::projection::NavigationInventory::new(&graph)),
        graph,
        generation: "another-current-generation".into(),
        metrics: None,
        presentation_cache: None,
        retention: None,
        context: Some(QueryContext::new(
            state.runtime.as_ref().unwrap(),
            Descriptor {
                project: root.canonicalize().unwrap(),
                roots: Vec::new(),
                generation: Some("another-current-generation".into()),
            },
        )),
    }
}
#[test]
fn loaded_context_wins_over_edited_root_and_only_an_actual_project_change_closes_history() {
    let root = tempfile::tempdir().unwrap();
    project(root.path());
    let mut state = state(root.path());
    state.query_context = loaded(&state, root.path()).context;
    state.root = rune::TextEdit::new("/edited/root/not/the/loaded/project");
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    let catalog = state.flow_comparison.catalog.clone().unwrap();
    assert_eq!(
        state.flow_comparison.descriptor().unwrap().project,
        root.path().canonicalize().unwrap()
    );
    assert_eq!(
        state
            .query_context
            .as_ref()
            .unwrap()
            .descriptor
            .generation
            .as_deref(),
        Some("another-current-generation")
    );
    let replacement = loaded(&state, root.path());
    crate::graph_install::install(&mut state, replacement, crate::graph_install::Mode::Adopt);
    assert!(Arc::ptr_eq(
        &catalog,
        state.flow_comparison.catalog.as_ref().unwrap()
    ));
    let other = tempfile::tempdir().unwrap();
    let replacement = loaded(&state, other.path());
    crate::graph_install::install(
        &mut state,
        replacement,
        crate::graph_install::Mode::NewWorkspace,
    );
    assert!(matches!(state.flow_comparison.phase, Phase::Closed));
    assert!(state.flow_comparison.descriptor().is_none());
}
#[test]
fn manual_pairing_can_reset_and_target_navigation_is_endpoint_bound() {
    let root = tempfile::tempdir().unwrap();
    project(root.path());
    let mut state = state(root.path());
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    let symbol = selected(&state);
    let other = state
        .flow_comparison
        .catalog
        .as_ref()
        .unwrap()
        .records
        .iter()
        .find(|row| row.symbol != symbol)
        .unwrap()
        .symbol
        .clone();
    state
        .flow_comparison
        .select(Selection {
            symbol: symbol.clone(),
            head_symbol: Some(other),
        })
        .unwrap();
    settle(&mut state);
    assert!(matches!(
        state
            .flow_comparison
            .current
            .as_ref()
            .unwrap()
            .correspondence
            .kind,
        crate::engine_client::flow_comparison::CorrespondenceKind::Manual
    ));
    assert!(
        state
            .flow_comparison
            .follow(SideId::After, "unowned-target")
            .is_err()
    );
    state.flow_comparison.reset_counterpart().unwrap();
    settle(&mut state);
    let pair = state.flow_comparison.current.as_ref().unwrap();
    assert_eq!(pair.selection.symbol, symbol);
    assert!(pair.selection.head_symbol.is_none());
    assert!(matches!(
        pair.correspondence.kind,
        crate::engine_client::flow_comparison::CorrespondenceKind::Exact
    ));
    assert!(
        !state.flow_comparison.back(),
        "An explicit change selection must not create call navigation history"
    );
}

#[test]
fn following_a_target_from_a_manual_pair_restores_exact_correspondence() {
    let root = tempfile::tempdir().unwrap();
    project(root.path());
    let mut state = state(root.path());
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    let symbol = selected(&state);
    let other = state
        .flow_comparison
        .catalog
        .as_ref()
        .unwrap()
        .records
        .iter()
        .find(|row| row.symbol != symbol)
        .unwrap()
        .symbol
        .clone();
    state
        .flow_comparison
        .select(Selection {
            symbol,
            head_symbol: Some(other),
        })
        .unwrap();
    settle(&mut state);
    let Side::Ready(before) = &state.flow_comparison.current.as_ref().unwrap().before else {
        panic!()
    };
    let target = before.targets[0].symbol.clone();
    state
        .flow_comparison
        .follow(SideId::Before, &target)
        .unwrap();
    settle(&mut state);
    let pair = state.flow_comparison.current.as_ref().unwrap();
    assert_eq!(pair.selection.symbol, target);
    assert!(pair.selection.head_symbol.is_none());
    assert!(matches!(
        pair.correspondence.kind,
        crate::engine_client::flow_comparison::CorrespondenceKind::Exact
    ));
    assert!(state.flow_comparison.back());
    assert!(matches!(
        state
            .flow_comparison
            .current
            .as_ref()
            .unwrap()
            .correspondence
            .kind,
        crate::engine_client::flow_comparison::CorrespondenceKind::Manual
    ));
}

#[test]
fn symlink_capture_survives_adoption_of_the_same_canonical_project() {
    let root = tempfile::tempdir().unwrap();
    let canonical = root.path().canonicalize().unwrap();
    project(&canonical);
    let aliases = tempfile::tempdir().unwrap();
    let alias = aliases.path().join("workspace alias");
    std::os::unix::fs::symlink(&canonical, &alias).unwrap();
    let mut state = state(&alias);
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    let catalog = state.flow_comparison.catalog.clone().unwrap();
    let selected = selected(&state);
    state
        .flow_comparison
        .select(Selection {
            symbol: selected.clone(),
            head_symbol: None,
        })
        .unwrap();
    settle(&mut state);
    let replacement = loaded(&state, &canonical);
    crate::graph_install::install(&mut state, replacement, crate::graph_install::Mode::Adopt);
    assert!(
        state
            .flow_comparison
            .catalog
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, &catalog)),
        "Adopting the same physical project must preserve the captured comparison catalog"
    );
    assert_eq!(
        state
            .flow_comparison
            .current
            .as_ref()
            .map(|pair| pair.selection.symbol.as_str()),
        Some(selected.as_str())
    );
}

#[test]
fn retargeting_a_project_alias_cannot_redirect_bound_detail_or_explicit_reload() {
    let root = tempfile::tempdir().unwrap();
    let canonical = root.path().canonicalize().unwrap();
    project(&canonical);
    let aliases = tempfile::tempdir().unwrap();
    let alias = aliases.path().join("alias");
    std::os::unix::fs::symlink(&canonical, &alias).unwrap();
    let mut state = state(&alias);
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    let symbol = selected(&state);
    let other = tempfile::tempdir().unwrap();
    std::fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink(other.path(), &alias).unwrap();
    state
        .flow_comparison
        .select(Selection {
            symbol,
            head_symbol: None,
        })
        .unwrap();
    settle(&mut state);
    assert!(
        state.flow_comparison.current.is_some(),
        "{:?}",
        state.flow_comparison.phase
    );
    let revision = "refs/tags/base";
    state
        .flow_comparison
        .revisions
        .set_for_test(crate::flow_comparison::SnapshotSide::Base, revision);
    state.lookup_comparison_snapshot(snapshots::Side::Base);
    settle(&mut state);
    let lookup = state
        .flow_comparison
        .snapshots
        .lookup(snapshots::Side::Base, revision);
    assert_eq!(lookup.map(|lookup| lookup.status.snapshots.len()), Some(1));
    state.flow_comparison.reload().unwrap();
    settle(&mut state);
    assert!(
        state.flow_comparison.catalog.is_some(),
        "{:?}",
        state.flow_comparison.phase
    );
    assert_eq!(
        state.flow_comparison.identity.captured(),
        Some(canonical.as_path())
    );
}
