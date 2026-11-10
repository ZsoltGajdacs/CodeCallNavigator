//! Production flow workspace interactions over an explicit analyzed-source snapshot.
use super::click;
use crate::{app::State, flow_scenes, jobs, view};
use rune::{Env, Runtime};
use std::{thread, time::Duration};

fn runtime() -> Runtime<State> {
    let mut state = State::new();
    flow_scenes::install(&mut state);
    let mut runtime = Runtime::new(state, view::view, (1420.0, 900.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    runtime.on_key = Some(view::key);
    ready(&mut runtime);
    runtime
}
fn ready(runtime: &mut Runtime<State>) {
    for _ in 0..400 {
        runtime.frame(1.0 / 60.0);
        if runtime
            .state
            .flow_workspace
            .current
            .as_ref()
            .is_some_and(|s| s.ready.is_some())
        {
            runtime.frame(1.0 / 60.0);
            runtime.frame(1.0 / 60.0);
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!(
        "Flow did not become ready: {}",
        runtime
            .state
            .flow_workspace
            .current
            .as_ref()
            .map(|s| s.message.as_str())
            .unwrap_or("closed")
    );
}
#[test]
fn flow_step_highlights_snapshot_and_back_preserves_call_graph() {
    let mut runtime = runtime();
    let before = serde_json::to_value(&runtime.state.nav).unwrap();
    let index = session(&runtime)
        .ready
        .as_ref()
        .unwrap()
        .chart
        .steps
        .iter()
        .position(|s| !s.calls.is_empty())
        .unwrap();
    click(&mut runtime, &format!("flow-step-{index}"));
    assert_eq!(session(&runtime).selected, Some(index));
    assert!(runtime.elements().iter().any(|e| {
        e.text
            .as_deref()
            .is_some_and(|t| t.contains("save() · direct"))
    }));
    assert_source_highlight(&mut runtime, 1, 0);
    runtime.update(|state, _| {
        state.flow_workspace.current.as_mut().unwrap().camera.pan = [17.0, 23.0]
    });
    click(&mut runtime, "flow-target-0-0");
    ready(&mut runtime);
    assert_eq!(runtime.state.flow_workspace.back.len(), 1);
    click(&mut runtime, "flow-back");
    assert_eq!(session(&runtime).symbol, "flow/load");
    assert_eq!(session(&runtime).selected, Some(index));
    assert_eq!(session(&runtime).camera.pan, [17.0, 23.0]);
    click(&mut runtime, "flow-close");
    assert!(runtime.state.flow_workspace.current.is_none());
    assert_eq!(serde_json::to_value(&runtime.state.nav).unwrap(), before);
}
#[test]
fn http_boundary_target_is_explicitly_labelled_in_flow_details() {
    let mut runtime = runtime();
    runtime.update(|state, _| {
        state.close_flow();
        let mut graph = flow_scenes::data();
        for function in &mut graph.flow_inventory.as_mut().unwrap().functions {
            codegraph::flow::visit_calls_mut(function, &mut |call| {
                for target in &mut call.targets {
                    target.kind = "http".into();
                    target.certainty = "possible".into();
                }
            });
        }
        state.graph = Some(std::sync::Arc::new(
            codegraph::graph::GraphIndex::new(std::sync::Arc::new(graph)).unwrap(),
        ));
        state.flow_workspace.invalidate();
        state.open_flow("flow/load".into());
    });
    ready(&mut runtime);
    let index = session(&runtime)
        .ready
        .as_ref()
        .unwrap()
        .chart
        .steps
        .iter()
        .position(|step| !step.calls.is_empty())
        .unwrap();
    click(&mut runtime, &format!("flow-step-{index}"));
    assert!(runtime.elements().iter().any(|element| {
        element
            .text
            .as_deref()
            .is_some_and(|text| text.contains("save() · possible · http"))
    }));
}
#[test]
fn flow_reads_cached_source_and_rejects_obsolete_generation() {
    let mut runtime = runtime();
    let session = runtime.state.flow_workspace.current.as_ref().unwrap();
    let source = &session.ready.as_ref().unwrap().source;
    assert!(source.text.contains("function load"));
    assert!(!std::path::Path::new("/snapshot/flow.ts").exists());
    runtime.update(|s, _| s.epoch += 1);
    runtime.frame(1.0 / 60.0);
    assert!(runtime.state.flow_workspace.current.is_none());
}
#[test]
fn old_graph_opens_an_actionable_refresh_message() {
    let mut runtime = runtime();
    runtime.update(|state, _| {
        state.close_flow();
        let mut graph = flow_scenes::data();
        graph.flow_inventory = None;
        state.graph = Some(std::sync::Arc::new(
            codegraph::graph::GraphIndex::new(std::sync::Arc::new(graph)).unwrap(),
        ));
        state.flow_workspace.invalidate();
        state.open_flow("flow/load".into());
    });
    runtime.frame(1.0 / 60.0);
    assert!(
        runtime
            .state
            .flow_workspace
            .current
            .as_ref()
            .unwrap()
            .message
            .contains("Refresh")
    );
    assert!(runtime.find("flow-close").is_some());
}
#[test]
fn flow_source_and_chart_remain_inside_minimum_window() {
    let mut runtime = runtime();
    runtime.set_viewport((1050.0, 680.0), 1.0);
    runtime.frame(1.0 / 60.0);
    for key in ["flow-close", "flow-canvas", "flow-source-pane", "flow-fit"] {
        let element = runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap();
        assert!(
            element.rect[0] >= 0.0 && element.rect[0] + element.rect[2] <= 1051.0,
            "{key}: {:?}",
            element.rect
        );
        assert!(
            element.visible[2] > 20.0 && element.visible[3] > 15.0,
            "{key} is clipped"
        );
    }
}

fn assert_source_highlight(runtime: &mut Runtime<State>, selected: usize, unselected: usize) {
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }
    let alpha = |index| {
        runtime
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(format!("flow-source-line-{index}").as_str()))
            .unwrap()
            .fill
            .a
    };
    assert!(
        alpha(selected) > 0.1,
        "Selected source row did not receive visible highlighting"
    );
    assert!(
        alpha(unselected) < 0.01,
        "An unrelated source row was highlighted"
    );
}

#[test]
fn selecting_another_step_moves_the_visible_source_highlight() {
    let mut runtime = runtime();
    let chart = &runtime
        .state
        .flow_workspace
        .current
        .as_ref()
        .unwrap()
        .ready
        .as_ref()
        .unwrap()
        .chart;
    let call = chart
        .steps
        .iter()
        .position(|step| !step.calls.is_empty())
        .unwrap();
    let zero = chart
        .steps
        .iter()
        .position(|step| step.label == "0")
        .unwrap();
    click(&mut runtime, &format!("flow-step-{call}"));
    assert_source_highlight(&mut runtime, 1, 2);
    click(&mut runtime, &format!("flow-step-{zero}"));
    assert_source_highlight(&mut runtime, 2, 1);
}

fn session(runtime: &Runtime<State>) -> &crate::flow_workspace::Session {
    runtime.state.flow_workspace.current.as_ref().unwrap()
}

#[test]
fn inspector_action_opens_flow_and_switching_during_load_keeps_latest_function() {
    let mut state = State::new();
    flow_scenes::install(&mut state);
    state.close_flow();
    let mut runtime = Runtime::new(state, view::view, (1420.0, 900.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    runtime.frame(0.0);
    click(&mut runtime, "open-flowchart");
    runtime.update(|state, _| state.open_flow("flow/save".into()));
    ready(&mut runtime);
    let current = runtime.state.flow_workspace.current.as_ref().unwrap();
    assert_eq!(current.symbol, "flow/save");
    assert_eq!(current.ready.as_ref().unwrap().chart.symbol, "flow/save");
}

fn shift_spans(value: &mut serde_json::Value, delta: usize) {
    match value {
        serde_json::Value::Object(fields) => {
            if let Some(span) = fields.get_mut("span").and_then(|v| v.as_object_mut()) {
                for key in ["start", "end"] {
                    let offset = span[key].as_u64().unwrap() as usize;
                    span.insert(key.into(), serde_json::json!(offset + delta));
                }
            }
            for child in fields.values_mut() {
                shift_spans(child, delta);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                shift_spans(child, delta);
            }
        }
        _ => {}
    }
}

#[test]
fn previous_function_restores_the_actual_source_scroll_offset() {
    let mut state = State::new();
    flow_scenes::install(&mut state);
    state.flow_workspace.invalidate();
    let prefix = "// earlier source\n".repeat(200);
    let mut data = flow_scenes::data();
    data.architecture.as_mut().unwrap().files[0]
        .content
        .insert_str(0, &prefix);
    let mut inventory = serde_json::to_value(data.flow_inventory.take().unwrap()).unwrap();
    shift_spans(&mut inventory, prefix.len());
    data.flow_inventory = Some(serde_json::from_value(inventory).unwrap());
    state.graph = Some(std::sync::Arc::new(
        codegraph::graph::GraphIndex::new(std::sync::Arc::new(data)).unwrap(),
    ));
    state.open_flow("flow/load".into());
    let mut runtime = Runtime::new(state, view::view, (1420.0, 900.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    ready(&mut runtime);
    scroll_source(&mut runtime, 120.0);
    let before = source_scroll(&runtime);
    assert!(
        (before[1] - 120.0).abs() < 1.0,
        "Actual source scroll did not move: {before:?}"
    );
    runtime.update(|state, _| state.open_flow("flow/save".into()));
    ready(&mut runtime);
    let saved_callee = source_scroll(&runtime);
    assert!(saved_callee[1] > 1000.0);
    runtime.update(|state, _| state.open_flow("flow/load".into()));
    ready(&mut runtime);
    assert!(
        source_scroll(&runtime)[1] > 1000.0,
        "Cached charts should reveal their method"
    );
    scroll_source(&mut runtime, 240.0);
    click(&mut runtime, "flow-back");
    runtime.frame(1.0 / 60.0);
    assert_eq!(source_scroll(&runtime), saved_callee);
    click(&mut runtime, "flow-back");
    runtime.frame(1.0 / 60.0);
    assert_eq!(source_scroll(&runtime), before);
}

fn source_scroll(runtime: &Runtime<State>) -> [f32; 2] {
    session(runtime).scroll.get()
}
fn scroll_source(runtime: &mut Runtime<State>, y: f32) {
    let key = session(runtime).scroll_key.clone();
    runtime.update(move |_, tx| tx.scroll_to_instant(key, 0.0, y));
    runtime.frame(1.0 / 60.0);
    runtime.frame(1.0 / 60.0);
}
