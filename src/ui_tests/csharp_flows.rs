//! Headless Rune controls over a real .NET/compiler/CLI C# artifact, never fixture queries.
use super::{
    cli_backend::{analyze_existing, desktop, settle},
    click,
    project_explorer::capture,
};
use crate::{
    app::State, engine_client, flow_workspace::Session, graph_interaction::SelectionAction,
};
use rune::Runtime;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

const SOURCE: &[u8] = include_bytes!("../../fixtures/flow/csharp-desktop/Example.cs");
const PROJECT: &[u8] = include_bytes!("../../fixtures/flow/csharp-desktop/DesktopFlow.csproj");

fn open_snapshot(project: &Path) -> engine_client::Snapshot {
    let runtime = crate::runtime::Runtime::discover().unwrap();
    engine_client::QueryContext::for_project(&runtime, project.into())
        .snapshot(&AtomicBool::new(false))
        .unwrap()
}

fn session(runtime: &Runtime<State>) -> &Session {
    runtime.state.flow_workspace.current.as_ref().unwrap()
}

fn frames(runtime: &mut Runtime<State>) {
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }
}

fn symbol(runtime: &Runtime<State>, suffix: &str) -> String {
    let matches: Vec<_> = runtime
        .state
        .graph
        .as_ref()
        .unwrap()
        .data
        .nodes
        .iter()
        .filter(|node| node.signature.ends_with(suffix))
        .collect();
    assert_eq!(matches.len(), 1, "Unambiguous fixture signature {suffix}");
    matches[0].id.clone()
}

fn select(runtime: &mut Runtime<State>, identity: &str) {
    let member = runtime
        .state
        .graph
        .as_ref()
        .unwrap()
        .node_index(identity)
        .unwrap();
    runtime.update(|state, _| state.select_method(member, SelectionAction::Inspect));
    settle(runtime, |state| state.view_is_current());
    frames(runtime);
}

fn open(runtime: &mut Runtime<State>, identity: &str) {
    if runtime.state.flow_workspace.current.is_some() {
        click(runtime, "flow-close");
    }
    select(runtime, identity);
    click(runtime, "open-flowchart");
    settle(runtime, |state| {
        state
            .flow_workspace
            .current
            .as_ref()
            .is_some_and(|session| session.ready.is_some())
    });
    frames(runtime);
    assert_eq!(session(runtime).symbol, identity);
}

fn call_step(runtime: &Runtime<State>, label: &str) -> usize {
    session(runtime)
        .ready
        .as_ref()
        .unwrap()
        .chart
        .steps
        .iter()
        .position(|step| step.calls.iter().any(|call| call.label == label))
        .unwrap()
}

fn visible_text(runtime: &Runtime<State>, fragment: &str) -> bool {
    runtime.elements().iter().any(|element| {
        element
            .text
            .as_deref()
            .is_some_and(|text| text.contains(fragment))
    })
}

fn exact_snapshot(runtime: &Runtime<State>) {
    assert!(!runtime.state.uses_fixture_queries());
    let ready = session(runtime).ready.as_ref().unwrap();
    assert_eq!(ready.source.text.as_bytes(), SOURCE);
    assert!(ready.chart.steps.iter().any(|step| step.kind == "decision"));
}

fn source_highlight(runtime: &Runtime<State>, step: usize) {
    let ready = session(runtime).ready.as_ref().unwrap();
    let span = &ready.chart.steps[step].span;
    let line = ready
        .source
        .lines
        .partition_point(|line| line.range.end < span.start);
    let key = format!("flow-source-line-{line}");
    let row = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some(key.as_str()))
        .unwrap();
    assert!(
        row.fill.a > 0.1,
        "Exact source row has a visible selected fill"
    );
}

fn assert_minimum_window(runtime: &mut Runtime<State>) {
    runtime.set_viewport((1050.0, 680.0), 1.0);
    for dark in [true, false] {
        runtime.set_system_dark(dark);
        frames(runtime);
        minimum_controls(runtime);
        capture_minimum(runtime, dark);
    }
}

fn minimum_controls(runtime: &Runtime<State>) {
    for key in [
        "flow-close",
        "flow-back",
        "flow-canvas",
        "flow-source-pane",
        "flow-fit",
    ] {
        let element = runtime
            .elements()
            .into_iter()
            .find(|element| element.key.as_deref() == Some(key))
            .unwrap();
        assert!(
            element.rect[0] >= 0.0 && element.rect[0] + element.rect[2] <= 1051.0,
            "{key} horizontal bounds: {:?}",
            element.rect
        );
        assert!(
            element.rect[1] >= 0.0 && element.rect[1] + element.rect[3] <= 681.0,
            "{key} vertical bounds: {:?}",
            element.rect
        );
        assert!(
            element.visible[2] > 20.0 && element.visible[3] > 15.0,
            "{key} clipped"
        );
    }
}

fn capture_minimum(runtime: &mut Runtime<State>, dark: bool) {
    if let Some(output) = std::env::var_os("CODECALLNAVIGATOR_CSHARP_UI_CAPTURES") {
        let name = if dark {
            "dark-minimum.png"
        } else {
            "light-minimum.png"
        };
        capture(runtime, (1050.0, 680.0), &PathBuf::from(output).join(name));
    }
}

#[derive(Debug, PartialEq)]
struct Bookmark {
    symbol: String,
    selected: Option<usize>,
    scroll: [f32; 2],
    camera: serde_json::Value,
}

impl Bookmark {
    fn current(runtime: &Runtime<State>) -> Self {
        let current = session(runtime);
        Self {
            symbol: current.symbol.clone(),
            selected: current.selected,
            scroll: current.scroll.get(),
            camera: serde_json::to_value(&current.camera).unwrap(),
        }
    }
}

fn navigate_target_and_back(runtime: &mut Runtime<State>, leaf: &str, step: usize) {
    click(runtime, &format!("flow-step-{step}"));
    frames(runtime);
    source_highlight(runtime, step);
    let key = session(runtime).scroll_key.clone();
    runtime.update(move |state, tx| {
        state.flow_workspace.current.as_mut().unwrap().camera.pan = [17.0, 23.0];
        tx.scroll_to_instant(key, 0.0, 120.0);
    });
    frames(runtime);
    let scroll = session(runtime).scroll.get();
    assert!((scroll[1] - 120.0).abs() < 1.0);
    let saved = Bookmark::current(runtime);
    click(runtime, "flow-target-0-0");
    settle(runtime, |state| {
        state
            .flow_workspace
            .current
            .as_ref()
            .is_some_and(|current| current.symbol == leaf && current.ready.is_some())
    });
    frames(runtime);
    click(runtime, "flow-back");
    frames(runtime);
    assert_eq!(Bookmark::current(runtime), saved);
}

#[test]
#[ignore = "requires built CLI, .NET11 SDK and Graphviz; optional captures require Metal"]
fn real_csharp_desktop_flows_reuse_one_analysis() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().canonicalize().unwrap();
    fs::write(project.join("DesktopFlow.csproj"), PROJECT).unwrap();
    let source = project.join("Example.cs");
    fs::write(&source, SOURCE).unwrap();

    exact_source_cleanup_targets_and_history(desktop(analyze_existing(&project), &project));
    cached_source_partial_flow_and_missing_body(
        desktop(open_snapshot(&project), &project),
        &source,
    );
    // Restore live inputs for dispatch. Replacing the published inventory is the final scenario.
    fs::write(&source, SOURCE).unwrap();
    possible_dispatch_and_old_inventory(&project);
}

fn exact_source_cleanup_targets_and_history(mut runtime: Runtime<State>) {
    let run = symbol(&runtime, "Example::Run(System.Boolean,DesktopFlow.IWorker)");
    let leaf = symbol(&runtime, "Example::Leaf(System.Int32)");
    open(&mut runtime, &run);
    exact_snapshot(&runtime);
    let step = call_step(&runtime, "Leaf(7)");
    let ready = session(&runtime).ready.as_ref().unwrap();
    let call = &ready.chart.steps[step].calls[0];
    assert_eq!(
        &ready.source.text[call.span.start..call.span.end],
        "Leaf(7)"
    );
    let targets: Vec<_> = call
        .targets
        .iter()
        .map(|target| (target.symbol.as_str(), target.certainty.as_str()))
        .collect();
    assert_eq!(targets, vec![(leaf.as_str(), "direct")]);
    let cleanup = ready
        .chart
        .steps
        .iter()
        .position(|step| {
            step.calls
                .iter()
                .any(|call| call.label.starts_with("synthetic C#: Dispose captured"))
        })
        .unwrap();
    click(&mut runtime, &format!("flow-step-{cleanup}"));
    frames(&mut runtime);
    assert!(visible_text(&runtime, "roslyn:implicit:Dispose"));
    assert!(
        runtime.find("flow-target-0-0").is_none(),
        "Implicit cleanup must not invent a native graph target"
    );
    let graph_navigation = serde_json::to_value(&runtime.state.nav).unwrap();
    navigate_target_and_back(&mut runtime, &leaf, step);
    assert_minimum_window(&mut runtime);
    click(&mut runtime, "flow-close");
    assert_eq!(
        serde_json::to_value(&runtime.state.nav).unwrap(),
        graph_navigation
    );
}

fn cached_source_partial_flow_and_missing_body(mut runtime: Runtime<State>, source: &Path) {
    let run = symbol(&runtime, "Example::Run(System.Boolean,DesktopFlow.IWorker)");
    fs::write(source, "// a different live file\n").unwrap();
    open(&mut runtime, &run);
    exact_snapshot(&runtime);
    click(&mut runtime, "flow-close");
    fs::remove_file(source).unwrap();
    let suspended = symbol(&runtime, "Example::Suspended()");
    open(&mut runtime, &suspended);
    let ready = session(&runtime).ready.as_ref().unwrap();
    assert_eq!(ready.source.text.as_bytes(), SOURCE);
    assert!(
        ready.chart.partial
            && ready
                .chart
                .steps
                .iter()
                .any(|step| step.kind == "unsupported")
    );
    assert!(
        ready.chart.steps.iter().all(|step| step.calls.is_empty()),
        "Async body must not appear eager"
    );
    assert!(visible_text(&runtime, "Partial flow"));
    click(&mut runtime, "flow-close");
    let absent = symbol(&runtime, "MissingBody::Absent()");
    select(&mut runtime, &absent);
    // The C# adapter's flag means missing source file, not an abstract-body inference.
    // An authored declaration therefore asks the CLI for authoritative flow availability.
    click(&mut runtime, "open-flowchart");
    settle(&mut runtime, |state| {
        state
            .flow_workspace
            .current
            .as_ref()
            .is_some_and(|current| {
                current.ready.is_none() && current.message.contains("flow_unavailable")
            })
    });
    assert!(visible_text(&runtime, "flow_unavailable"));
    assert!(runtime.find("flow-close").is_some());
    click(&mut runtime, "flow-close");
    let pending = symbol(&runtime, "Example::Leaf(System.String)");
    runtime.update(|state, _| {
        state.open_flow(pending.clone());
        state.epoch += 1;
    });
    frames(&mut runtime);
    assert!(
        runtime.state.flow_workspace.current.is_none(),
        "Obsolete in-flight flow must not reopen after epoch change"
    );
}

fn possible_dispatch_and_old_inventory(project: &Path) {
    let mut runtime = desktop(open_snapshot(project), project);
    let run = symbol(&runtime, "Example::Run(System.Boolean,DesktopFlow.IWorker)");
    open(&mut runtime, &run);
    let step = call_step(&runtime, "worker.Work()");
    assert!(
        session(&runtime).ready.as_ref().unwrap().chart.steps[step]
            .calls
            .iter()
            .flat_map(|call| &call.targets)
            .any(|target| target.certainty == "possible")
    );
    click(&mut runtime, &format!("flow-step-{step}"));
    frames(&mut runtime);
    assert!(visible_text(&runtime, "possible"));
    click(&mut runtime, "flow-close");
    drop(runtime);
    let mut artifact = codecallnavigator::artifact::load(project).unwrap();
    artifact
        .graph
        .as_object_mut()
        .unwrap()
        .remove("flow_inventory");
    codecallnavigator::artifact::publish(project, &artifact, &AtomicBool::new(false)).unwrap();
    let mut old = desktop(open_snapshot(project), project);
    select(&mut old, &run);
    click(&mut old, "open-flowchart");
    settle(&mut old, |state| {
        state
            .flow_workspace
            .current
            .as_ref()
            .is_some_and(|session| session.message.contains("Refresh"))
    });
    assert!(visible_text(&old, "Refresh"));
    assert!(session(&old).ready.is_none());
}
