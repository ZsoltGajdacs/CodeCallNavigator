//! Real compiler/CLI/Graphviz ownership is necessary to prove generation-bound Rune expansion.
mod support;
use super::{cli_backend::settle, click};
use codecallnavigator::{artifact, flow};
use std::{fs, sync::Arc};
use support::*;

#[test]
#[ignore = "requires the real .NET analyzer, CLI and Graphviz"]
fn expansion_mode_is_explicit_and_collapse_all_restores_the_ordinary_session() {
    let (project, mut runtime) = open();
    scroll(&mut runtime, 120.0);
    runtime.update(|state, _| {
        state.flow_workspace.current.as_mut().unwrap().camera.pan = [17.0, 23.0]
    });
    let ordinary = session(&runtime).clone();
    let saved_scroll = ordinary.scroll.get();
    assert!((rendered_scroll(&runtime) - 120.0).abs() < 1.0);
    for name in ["Root.cs", "Helpers.cs"] {
        fs::remove_file(project.path().join(name)).unwrap();
    }
    click(&mut runtime, "flow-expand-mode");
    ready(&mut runtime);
    assert_eq!(result(&runtime).occurrences.len(), 1);
    let first = root_call(&runtime, 0);
    expand(&mut runtime, first);
    assert_eq!(expanded(&runtime), 1);
    scroll(&mut runtime, 120.0);
    let second = root_call(&runtime, 1);
    select(&mut runtime, second);
    highlight(&runtime, "Root after first expansion");
    let first_path = result(&runtime).occurrences[1].path.clone();
    assert_sources_and_back(&mut runtime, &first_path);
    assert_cycle_and_independent_sibling(&mut runtime, &first_path);
    click(&mut runtime, "flow-collapse-all");
    frames(&mut runtime);
    let restored = session(&runtime);
    assert!(Arc::ptr_eq(
        restored.ready.as_ref().unwrap(),
        ordinary.ready.as_ref().unwrap()
    ));
    assert_eq!(restored.camera.pan, ordinary.camera.pan);
    assert_eq!(restored.selected, ordinary.selected);
    assert_eq!(restored.scroll.get(), saved_scroll);
    assert!((rendered_scroll(&runtime) - saved_scroll[1]).abs() < 1.0);
    assert_stale_request_retains_ordinary(&mut runtime, project.path());
}
fn assert_sources_and_back(
    runtime: &mut rune::Runtime<crate::app::State>,
    path: &flow::OccurrencePath,
) {
    let step = child_call(runtime, path);
    select(runtime, step);
    assert_helper_source(runtime);
    highlight(runtime, "Helper source switch");
    scroll(runtime, 120.0);
    let ready = session(runtime).ready.as_ref().unwrap().clone();
    let before = session(runtime).scroll.get();
    runtime.update(|state, _| {
        state.flow_workspace.current.as_mut().unwrap().camera.pan = [31.0, 41.0]
    });
    click(runtime, "flow-target-0-0");
    settle(runtime, |state| {
        state
            .flow_workspace
            .current
            .as_ref()
            .is_some_and(|session| session.ready.is_some() && !session.expansion_mode())
    });
    click(runtime, "flow-back");
    frames(runtime);
    assert!(Arc::ptr_eq(
        session(runtime).ready.as_ref().unwrap(),
        &ready
    ));
    assert_eq!(session(runtime).camera.pan, [31.0, 41.0]);
    assert_eq!(session(runtime).scroll.get(), before);
    assert!((rendered_scroll(runtime) - before[1]).abs() < 1.0);
    assert_helper_source(runtime);
}
fn assert_cycle_and_independent_sibling(
    runtime: &mut rune::Runtime<crate::app::State>,
    path: &flow::OccurrencePath,
) {
    let child = child_call(runtime, path);
    expand(runtime, child);
    let stopped = result(runtime)
        .occurrences
        .iter()
        .find(|occurrence| {
            matches!(
                occurrence.outcome,
                flow::OccurrenceOutcome::Stopped {
                    reason: flow::ExpansionStop::Cycle,
                    ..
                }
            )
        })
        .unwrap();
    let marker = stopped.boundaries.marker.unwrap();
    select(runtime, marker);
    let badge = format!("flow-step-kind-{marker}");
    assert!(runtime.elements().iter().any(|element| {
        element.key.as_deref() == Some(&badge)
            && element
                .text
                .as_deref()
                .is_some_and(|text| text.contains("Cycle"))
    }));
    assert!(runtime.elements().iter().any(|element| {
        element
            .text
            .as_deref()
            .is_some_and(|text| text.contains("Recursive target is not expanded"))
    }));
    click(runtime, "flow-collapse-occurrence");
    ready(runtime);
    assert_eq!(expanded(runtime), 1);
    assert_eq!(result(runtime).occurrences.len(), 2);
    let second = root_call(runtime, 1);
    expand(runtime, second);
    assert_eq!(expanded(runtime), 2);
    assert_eq!(result(runtime).sources.len(), 2);
    let child = child_call(runtime, path);
    select(runtime, child);
    click(runtime, "flow-collapse-occurrence");
    ready(runtime);
    assert_eq!(expanded(runtime), 1);
    assert_ne!(result(runtime).occurrences[1].path, path.clone());
    let call = root_call(runtime, 1);
    select(runtime, call);
    minimum(runtime);
}
fn assert_stale_request_retains_ordinary(
    runtime: &mut rune::Runtime<crate::app::State>,
    project: &std::path::Path,
) {
    let mut saved = artifact::load(project).unwrap();
    saved
        .extensions
        .insert("expansion_stale_probe".into(), serde_json::json!(true));
    artifact::publish(project, &saved, &std::sync::atomic::AtomicBool::new(false)).unwrap();
    let ordinary = session(runtime).ready.as_ref().unwrap().clone();
    click(runtime, "flow-expand-mode");
    settle(runtime, |state| !state.flow_workspace.expansion_pending());
    assert!(session(runtime).message.contains("generation_mismatch"));
    assert!(Arc::ptr_eq(
        session(runtime).ready.as_ref().unwrap(),
        &ordinary
    ));
    assert!(session(runtime).expanded_flow().is_none());
    click(runtime, "flow-collapse-all");
    frames(runtime);
    assert!(Arc::ptr_eq(
        session(runtime).ready.as_ref().unwrap(),
        &ordinary
    ));
}
