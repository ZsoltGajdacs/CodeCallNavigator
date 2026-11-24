//! Entry and revision submission preserve both existing view state and captured project ownership.
use super::{
    click,
    flow_comparisons::{choose_revision, load, project_runtime, settle},
    function_flows::{prefixed_runtime, scroll_source, source_scroll},
};
use crate::{
    engine_client::{Descriptor, QueryContext},
    flow_comparison::SnapshotSide,
};
use std::sync::Arc;
fn selected_single_with_history() -> rune::Runtime<crate::app::State> {
    let mut runtime = prefixed_runtime();
    let step = runtime
        .state
        .flow_workspace
        .current
        .as_ref()
        .unwrap()
        .ready
        .as_ref()
        .unwrap()
        .chart
        .steps
        .iter()
        .position(|step| !step.calls.is_empty())
        .unwrap();
    click(&mut runtime, &format!("flow-step-{step}"));
    click(&mut runtime, "flow-target-0-0");
    super::function_flows::ready(&mut runtime);
    let selected = runtime
        .state
        .flow_workspace
        .current
        .as_ref()
        .unwrap()
        .ready
        .as_ref()
        .unwrap()
        .chart
        .steps
        .iter()
        .position(|step| step.kind == "return")
        .unwrap();
    click(&mut runtime, &format!("flow-step-{selected}"));
    scroll_source(&mut runtime, 160.0);
    runtime.update(|state, _| {
        state.flow_workspace.current.as_mut().unwrap().camera.pan = [17.0, 23.0];
    });
    runtime
}
#[test]
fn entry_cleanup_preserves_the_single_flow_session_history_and_actual_scroll() {
    let mut runtime = selected_single_with_history();
    let saved_scroll = source_scroll(&runtime);
    let current = runtime.state.flow_workspace.current.as_ref().unwrap();
    let ready = current.ready.clone().unwrap();
    let symbol = current.symbol.clone();
    let selection = current.selected;
    let camera = current.camera.pan;
    let history = runtime.state.flow_workspace.back.len();
    let previous = runtime.state.flow_workspace.back.last().unwrap().selected;
    click(&mut runtime, "compare-flows");
    assert!(
        runtime.state.flow_workspace.current.is_some(),
        "Comparison entry discarded the existing single-flow session"
    );
    click(&mut runtime, "comparison-close");
    let current = runtime.state.flow_workspace.current.as_ref().unwrap();
    assert_eq!(current.symbol, symbol);
    assert!(Arc::ptr_eq(current.ready.as_ref().unwrap(), &ready));
    assert_eq!(current.selected, selection);
    assert!(selection.is_some());
    assert_eq!(current.camera.pan, camera);
    assert_eq!(runtime.state.flow_workspace.back.len(), history);
    assert_eq!(source_scroll(&runtime), saved_scroll);
    click(&mut runtime, "flow-back");
    assert_eq!(
        runtime
            .state
            .flow_workspace
            .current
            .as_ref()
            .unwrap()
            .selected,
        previous
    );
}
fn replace_global_context(
    runtime: &mut rune::Runtime<crate::app::State>,
    path: &std::path::Path,
    loaded: bool,
) {
    runtime.update(|state, _| {
        state.root = rune::TextEdit::new(path.to_string_lossy());
        state.root_edited();
        if loaded {
            let mut descriptor = state.query_context.as_ref().unwrap().descriptor.clone();
            descriptor.roots = vec![("changed".into(), path.into())];
            state.query_context = Some(QueryContext::new(
                state.runtime.as_ref().unwrap(),
                descriptor,
            ));
        }
    });
}
#[test]
fn entry_cleanup_submitting_different_refs_keeps_the_captured_project_and_roots() {
    for loaded in [false, true] {
        let (mut runtime, project) = project_runtime(true);
        if loaded {
            runtime.update(|state, _| {
                state.query_context = Some(QueryContext::new(
                    state.runtime.as_ref().unwrap(),
                    Descriptor {
                        project: project.path().into(),
                        roots: vec![("original".into(), project.path().into())],
                        generation: None,
                    },
                ))
            });
        }
        crate::flow_comparison::tests::git(
            project.path(),
            &["commit", "--allow-empty", "-qm", "other"],
        );
        let other = codecallnavigator::artifact::revision::Repository::open(project.path())
            .unwrap()
            .resolve("HEAD")
            .unwrap();
        crate::flow_comparison::tests::retain_scene(
            project.path(),
            &crate::flow_scenes::data(),
            &other,
        );
        load(&mut runtime);
        let captured = runtime.state.flow_comparison.descriptor().unwrap().clone();
        let changed = tempfile::tempdir().unwrap();
        replace_global_context(&mut runtime, changed.path(), loaded);
        choose_revision(&mut runtime, SnapshotSide::Base, &other);
        choose_revision(&mut runtime, SnapshotSide::Head, &other);
        settle(&mut runtime);
        let actual = runtime.state.flow_comparison.descriptor().unwrap();
        assert_eq!(
            actual.project,
            project.path().canonicalize().unwrap(),
            "New refs recaptured the editable project"
        );
        assert_eq!(
            actual.roots, captured.roots,
            "New refs recaptured linked roots"
        );
        assert_eq!(
            runtime.state.flow_comparison.references(),
            Some((other.as_str(), other.as_str()))
        );
        assert!(runtime.state.flow_comparison.catalog.is_some());
    }
}
