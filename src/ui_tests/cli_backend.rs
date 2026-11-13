//! A real compiler graph drives the production desktop CLI backend and Rune controls.
use super::{click, project_explorer::capture};
use crate::{
    app::State, engine_client, jobs, projection::Grouping, search, source_popup::Contents, view,
};
use rune::{Env, Key, KeyEvent, Modifier, Mods, Runtime};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

pub(super) fn settle(runtime: &mut Runtime<State>, ready: impl Fn(&State) -> bool) {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(30) {
        runtime.frame(1.0 / 60.0);
        if ready(&runtime.state) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!(
        "CLI-backed UI did not settle: status={} warning={}",
        runtime.state.status, runtime.state.warning
    );
}

fn shift(runtime: &mut Runtime<State>) {
    for pressed in [true, false] {
        runtime.key(KeyEvent {
            key: Key::Modifier {
                modifier: Modifier::ShiftLeft,
                pressed,
            },
            mods: Mods {
                shift: pressed,
                ..Default::default()
            },
        });
        runtime.frame(0.025);
    }
}

pub(super) fn analyze(project: &Path) -> engine_client::Snapshot {
    fs::write(project.join("tsconfig.json"),r#"{"compilerOptions":{"strict":true,"target":"ES2022","types":[]},"include":["example.ts"]}"#).unwrap();
    fs::write(project.join("example.ts"),"export function leaf(): number { return 1; }\nexport function start(flag: boolean): number { if(flag) return leaf(); return 0; }\n").unwrap();
    let modules = std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES")
        .expect("Pinned TypeScript dependencies");
    std::os::unix::fs::symlink(modules, project.join("node_modules")).unwrap();
    analyze_existing(project)
}

pub(super) fn analyze_existing(project: &Path) -> engine_client::Snapshot {
    let runtime = crate::runtime::Runtime::discover().unwrap();
    let targets = engine_client::discover(&runtime, project).unwrap();
    engine_client::analyze_snapshot(
        &runtime,
        engine_client::Analysis {
            project: project.into(),
            targets,
        },
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap()
}

pub(super) fn desktop(snapshot: engine_client::Snapshot, pending: &Path) -> Runtime<State> {
    let mut state = State::new();
    state.runtime = Some(Arc::new(crate::runtime::Runtime::discover().unwrap()));
    state.generation = snapshot.context.descriptor.generation.clone();
    state.graph = Some(snapshot.graph);
    state.query_context = Some(snapshot.context);
    state.nav.grouping = Grouping::Method;
    state.root = rune::TextEdit::new(pending.to_string_lossy());
    state.root_edited();
    state.needs_layout = true;
    let mut runtime = Runtime::new(state, view::view, (1420.0, 900.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    runtime.on_key = Some(view::key);
    runtime.on_key_activity = Some(search::key_activity);
    runtime.set_modifier_keys(true);
    settle(&mut runtime, |state| state.view_is_current());
    runtime
}

pub(super) fn search_start(runtime: &mut Runtime<State>) -> String {
    shift(runtime);
    shift(runtime);
    assert!(runtime.state.search.open);
    runtime.update(|state, _| {
        state.search.input = rune::TextEdit::new("start");
        state.request_search();
    });
    settle(runtime, |state| !state.search.hits().is_empty());
    click(runtime, "search-result-0");
    settle(runtime, |state| {
        state.view_is_current() && !state.neighbors.1.is_empty()
    });
    let selected = runtime.state.selected().unwrap();
    let node = &runtime.state.graph.as_ref().unwrap().data.nodes[selected as usize];
    assert_eq!(node.label, "start");
    node.id.clone()
}

fn layer_focus_and_path_preserve_visibility(runtime: &Runtime<State>, symbol: &str) {
    use crate::projection::{FocusTarget, Scope};
    let graph = runtime.state.graph.as_ref().unwrap();
    let context = runtime.state.query_context.as_ref().unwrap().clone();
    let cancel = AtomicBool::new(false);
    let architecture: crate::graph_queries::ArchitectureResult = context
        .request(
            codecallnavigator::query::Query::Architecture {
                selection: Some(codecallnavigator::architecture::Selection::None),
                previous_signature: None,
                configuration_root: None,
            },
            &cancel,
        )
        .unwrap();
    let queries = crate::graph_queries::GraphQueries::cli(graph.clone(), context).with_layers(
        Some(codecallnavigator::architecture::Selection::None),
        Some(architecture.signature),
    );
    let mut request = runtime.state.request().unwrap();
    request.grouping = Grouping::Layer;
    request.scope = Scope::Focus(graph.node_index(symbol).unwrap());
    request.focus_target = Some(FocusTarget::Method(symbol.into()));
    request.layer_tab = Some("unrelated-tab".into());
    request.architecture = Some(Arc::new(
        codecallnavigator::architecture::LayerAssignments::default(),
    ));
    let selection = queries.render(&request, &cancel).unwrap();
    let view = crate::projection::project_prepared(graph, &request, selection);
    assert!(!view.focus_valid);
    assert!(
        view.nodes.is_empty(),
        "Focused layer view keeps active-tab clipping"
    );
    let leaf = graph
        .data
        .nodes
        .iter()
        .position(|node| node.label == "leaf")
        .unwrap() as u32;
    request.scope = Scope::Path(graph.node_index(symbol).unwrap(), leaf);
    request.focus_target = None;
    let selection = queries.render(&request, &cancel).unwrap();
    let view = crate::projection::project_prepared(graph, &request, selection);
    assert_eq!(
        view.nodes.len(),
        2,
        "Shortest paths bypass layer-tab clipping"
    );
    assert!(view.layer_bands.is_empty());
}

fn preview_loaded_project_source(runtime: &mut Runtime<State>) {
    let selected = runtime.state.selected().unwrap();
    let node = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|node| node.member == Some(selected))
        .unwrap()
        .clone();
    runtime.update(|state, tx| state.hover_source(node, true, tx));
    settle(runtime, |state| {
        matches!(
            state
                .source_popup
                .active
                .as_ref()
                .map(|preview| &preview.contents),
            Some(Contents::Source(_))
        )
    });
    let Contents::Source(source) = &runtime.state.source_popup.active.as_ref().unwrap().contents
    else {
        unreachable!()
    };
    assert!(source.text.contains("function start"));
    runtime.update(|state, _| state.close_source());
}

fn open_flow(runtime: &mut Runtime<State>, symbol: &str) {
    click(runtime, "open-flowchart");
    settle(runtime, |state| {
        state
            .flow_workspace
            .current
            .as_ref()
            .is_some_and(|session| session.ready.is_some())
    });
    let ready = runtime
        .state
        .flow_workspace
        .current
        .as_ref()
        .unwrap()
        .ready
        .as_ref()
        .unwrap();
    assert_eq!(ready.chart.symbol, symbol);
    assert!(ready.chart.steps.iter().any(|step| step.kind == "decision"));
    assert!(ready.source.text.contains("function start"));
}

fn optional_captures(runtime: &mut Runtime<State>) {
    let Some(output) = std::env::var_os("CODECALLNAVIGATOR_CLI_UI_CAPTURES") else {
        return;
    };
    let output = PathBuf::from(output);
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }
    capture(runtime, (1420.0, 900.0), &output.join("dark-flow.png"));
    runtime.set_system_dark(false);
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
    }
    capture(runtime, (1420.0, 900.0), &output.join("light-flow.png"));
}

#[test]
#[ignore = "requires built CLI, Node24/TypeScript6; optional captures require Metal"]
fn actual_cli_keeps_graph_context_across_edited_root_and_drives_search_source_flow() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().canonicalize().unwrap();
    let other = tempfile::tempdir().unwrap();
    fs::write(other.path().join("other.txt"), "another selected folder").unwrap();
    let snapshot = analyze(&project);
    let generation = snapshot.context.descriptor.generation.clone();
    let mut runtime = desktop(snapshot, other.path());
    let symbol = search_start(&mut runtime);
    assert!(!runtime.state.uses_fixture_queries());
    assert_eq!(
        runtime
            .state
            .query_context
            .as_ref()
            .unwrap()
            .descriptor
            .project,
        project
    );
    assert_eq!(runtime.state.generation, generation);
    layer_focus_and_path_preserve_visibility(&runtime, &symbol);
    settle(&mut runtime, |state| {
        state
            .explorer
            .rows
            .iter()
            .any(|row| row.label == "other.txt")
    });
    preview_loaded_project_source(&mut runtime);
    open_flow(&mut runtime, &symbol);
    optional_captures(&mut runtime);
    click(&mut runtime, "flow-close");
    assert!(runtime.state.flow_workspace.current.is_none());
    assert_eq!(runtime.state.nav.selected.as_deref(), Some(symbol.as_str()));
}
