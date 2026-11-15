use super::super::{
    cli_backend::{analyze_existing, desktop, settle},
    click,
};
use crate::{app::State, flow_workspace::Session};
use codecallnavigator::flow::{self, OccurrenceOutcome};
use rune::Runtime;
use std::fs;
pub fn open() -> (tempfile::TempDir, Runtime<State>) {
    let project = tempfile::tempdir().unwrap();
    for (name, bytes) in [
        (
            "ExpansionDesktop.csproj",
            include_bytes!("../../../fixtures/flow/expansion-desktop/ExpansionDesktop.csproj")
                .as_slice(),
        ),
        (
            "Root.cs",
            include_bytes!("../../../fixtures/flow/expansion-desktop/Root.cs").as_slice(),
        ),
        (
            "Helpers.cs",
            include_bytes!("../../../fixtures/flow/expansion-desktop/Helpers.cs").as_slice(),
        ),
    ] {
        fs::write(project.path().join(name), bytes).unwrap();
    }
    let snapshot = analyze_existing(project.path());
    let root = snapshot
        .graph
        .data
        .nodes
        .iter()
        .find(|node| node.label == "Run")
        .unwrap()
        .id
        .clone();
    let mut runtime = desktop(snapshot, project.path());
    runtime.update(|state, _| state.open_flow(root));
    settle(&mut runtime, |state| {
        state
            .flow_workspace
            .current
            .as_ref()
            .is_some_and(|session| session.ready.is_some())
    });
    frames(&mut runtime);
    (project, runtime)
}
pub fn session(runtime: &Runtime<State>) -> &Session {
    runtime.state.flow_workspace.current.as_ref().unwrap()
}
pub fn frames(runtime: &mut Runtime<State>) {
    for _ in 0..6 {
        runtime.frame(1.0 / 60.0);
    }
}
pub fn ready(runtime: &mut Runtime<State>) {
    settle(runtime, |state| {
        !state.flow_workspace.expansion_pending()
            && state
                .flow_workspace
                .current
                .as_ref()
                .is_some_and(|session| session.expanded_flow().is_some())
    });
    frames(runtime);
}
pub fn result(runtime: &Runtime<State>) -> &flow::ExpandedFlow {
    session(runtime).expanded_flow().unwrap()
}
pub fn select(runtime: &mut Runtime<State>, step: usize) {
    click(runtime, &format!("flow-step-{step}"));
    frames(runtime);
    assert_eq!(session(runtime).selected, Some(step));
}
pub fn root_call(runtime: &Runtime<State>, ordinal: usize) -> usize {
    result(runtime)
        .step_origins
        .iter()
        .zip(&session(runtime).ready.as_ref().unwrap().chart.steps)
        .filter(|(origin, step)| {
            origin.occurrence == flow::OccurrenceId(0) && !step.calls.is_empty()
        })
        .nth(ordinal)
        .unwrap()
        .1
        .id
}
pub fn child_call(runtime: &Runtime<State>, path: &flow::OccurrencePath) -> usize {
    let result = result(runtime);
    let occurrence = result
        .occurrences
        .iter()
        .find(|occurrence| &occurrence.path == path)
        .unwrap()
        .id;
    result
        .step_origins
        .iter()
        .zip(&session(runtime).ready.as_ref().unwrap().chart.steps)
        .find(|(origin, step)| origin.occurrence == occurrence && !step.calls.is_empty())
        .unwrap()
        .1
        .id
}
pub fn expand(runtime: &mut Runtime<State>, step: usize) {
    select(runtime, step);
    click(runtime, "flow-expand-0-0");
    ready(runtime);
}
pub fn expanded(runtime: &Runtime<State>) -> usize {
    result(runtime)
        .occurrences
        .iter()
        .filter(|occurrence| occurrence.outcome == OccurrenceOutcome::Expanded)
        .count()
}
pub fn scroll(runtime: &mut Runtime<State>, offset: f32) {
    let key = session(runtime).scroll_key.clone();
    runtime.update(move |_, tx| tx.scroll_to_instant(key, 0.0, offset));
    frames(runtime);
}
pub fn rendered_scroll(runtime: &Runtime<State>) -> f32 {
    let session = session(runtime);
    let elements = runtime.elements();
    let viewport = elements
        .iter()
        .find(|element| element.key.as_deref() == Some(&session.scroll_key))
        .unwrap();
    let (index, row) = session
        .displayed_source()
        .unwrap()
        .lines
        .iter()
        .enumerate()
        .find_map(|(i, _)| {
            elements
                .iter()
                .find(|element| {
                    element.key.as_deref() == Some(format!("flow-source-line-{i}").as_str())
                        && element.visible[3] > 19.0
                })
                .map(|row| (i, row))
        })
        .unwrap();
    viewport.rect[1] + index as f32 * 20.0 - row.rect[1]
}
pub fn highlight(runtime: &Runtime<State>, checkpoint: &str) {
    let session = session(runtime);
    let span = session.selected_source_span().unwrap();
    let source = session.displayed_source().unwrap();
    let index = source
        .lines
        .partition_point(|line| line.range.end < span.start);
    let key = format!("flow-source-line-{index}");
    let row = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some(&key))
        .unwrap();
    assert!(
        row.fill.a > 0.1 && row.visible[2] > 0.0 && row.visible[3] > 0.0,
        "{checkpoint}: selected occurrence source row must have visible highlight: selected={:?} file={:?} span={span:?} line={index} scroll={:?} rect={:?} visible={:?} fill={:?} exiting={} source_text={:?} owner={:?} matches={:?}",
        session.selected,
        session.displayed_file(),
        session.scroll.get(),
        row.rect,
        row.visible,
        row.fill,
        row.exiting,
        &source.text[source.lines[index].range.clone()],
        result(runtime).step_origins[session.selected.unwrap()]
            .source
            .map(|id| &result(runtime).sources.get(id).unwrap().owner),
        runtime
            .elements()
            .into_iter()
            .filter(|element| element.key.as_deref() == Some(&key))
            .map(|element| (element.rect, element.visible, element.fill, element.exiting))
            .collect::<Vec<_>>()
    );
}
pub fn minimum(runtime: &mut Runtime<State>) {
    runtime.set_viewport((1050.0, 680.0), 1.0);
    runtime.set_titlebar_inset(rune::tooling::Chrome::DEFAULT.titlebar_inset());
    for dark in [false, true] {
        runtime.set_system_dark(dark);
        frames(runtime);
        for key in [
            "flow-close",
            "flow-back",
            "flow-fit",
            "flow-collapse-all",
            "flow-collapse-call-0-0",
            "flow-canvas",
            "flow-source-pane",
        ] {
            contained(runtime, key);
        }
    }
}
pub fn assert_helper_source(runtime: &Runtime<State>) {
    assert_eq!(
        session(runtime).displayed_source().unwrap().text.as_bytes(),
        include_bytes!("../../../fixtures/flow/expansion-desktop/Helpers.cs")
    );
}

fn contained(runtime: &Runtime<State>, key: &str) {
    let element = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some(key) && !element.exiting)
        .unwrap();
    let [x, y, w, h] = element.rect;
    assert!(
        w > 0.0 && h > 0.0 && x >= -0.5 && y >= -0.5 && x + w <= 1050.5 && y + h <= 680.5,
        "{key} rect={:?} target={:?} viewport={:?} roots={:?}",
        element.rect,
        element.target,
        runtime.viewport,
        runtime
            .elements()
            .into_iter()
            .filter(|element| matches!(
                element.key.as_deref(),
                Some("kit-root" | "flow-workspace" | "flow-toolbar" | "flow-source-pane")
            ))
            .map(|element| (element.key, element.rect, element.target))
            .collect::<Vec<_>>()
    );
    assert!(
        (w - element.visible[2]).abs() < 0.5 && (h - element.visible[3]).abs() < 0.5,
        "Clipped {key}"
    );
}
