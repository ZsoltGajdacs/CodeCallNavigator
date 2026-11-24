//! Observers must see the rendered provenance and source, not only a matching controller state.
use super::*;
use crate::{
    flow_acceptance::{
        comparison::{observation, source_observation},
        input::Input,
    },
    flow_comparison::{SideId, SnapshotSide},
};
#[test]
fn provenance_and_source_checks_require_actual_endpoint_widgets() {
    let project = tempfile::tempdir().unwrap();
    crate::flow_comparison::tests::project(project.path());
    let mut state = crate::flow_comparison::tests::state(project.path());
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    crate::flow_comparison::tests::settle(&mut state);
    let header = state.flow_comparison.header().unwrap();
    let base = header.base.commit.clone().unwrap();
    let after = header.head.commit.clone().unwrap();
    state
        .flow_comparison
        .revisions
        .set_for_test(SnapshotSide::Base, &base);
    state
        .flow_comparison
        .revisions
        .set_for_test(SnapshotSide::Head, &after);
    let symbol = crate::flow_comparison::tests::selected(&state);
    state
        .flow_comparison
        .select(crate::flow_comparison::Selection {
            symbol,
            head_symbol: None,
        })
        .unwrap();
    crate::flow_comparison::tests::settle(&mut state);
    let mut runtime = Runtime::new(
        state,
        |state, cx| {
            if state.warning == "hide-provenance" {
                rune::el()
            } else {
                crate::view::view(state, cx)
            }
        },
        (1050.0, 680.0),
        rune::Env::new(true, 1.0),
    );
    runtime.poll = Some(crate::jobs::poll);
    runtime.set_titlebar_inset(rune::tooling::Chrome::DEFAULT.titlebar_inset());
    runtime.frame(0.0);
    runtime.frame(1.0 / 60.0);
    runtime.settle_all();
    runtime.frame(0.0);
    provenance(&runtime).unwrap();
    controls(&runtime).unwrap();
    assert!(source_observation::source_drawn(&runtime, SideId::Before).unwrap());
    let session = observation::session(&runtime.state, SideId::Before).unwrap();
    let index = session
        .ready
        .as_ref()
        .unwrap()
        .chart
        .steps
        .iter()
        .position(|step| !step.calls.is_empty())
        .unwrap();
    click_control(&mut runtime, &format!("comparison-before-step-{index}"));
    assert!(source_observation::highlighted(&runtime, SideId::Before).unwrap());
    runtime.update(|state, _| state.warning = "hide-provenance".into());
    runtime.frame(1.0 / 60.0);
    assert!(runtime.state.flow_comparison.header().is_some());
    assert!(provenance(&runtime).is_err());
    assert!(!source_observation::source_drawn(&runtime, SideId::Before).unwrap());
}

fn click_control(runtime: &mut Runtime<crate::app::State>, key: &str) {
    let mut input = Input::default();
    assert!(input.click(runtime, key), "Control {key} is not visible");
    runtime.frame(1.0 / 60.0);
    input.next(runtime);
    runtime.frame(1.0 / 60.0);
    runtime.frame(1.0 / 60.0);
}
