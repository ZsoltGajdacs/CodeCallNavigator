//! Pure controller policies consume actual core expansions of explicit immutable IR.
//! No CLI, compiler, filesystem, layout subprocess or background publisher is used.
use super::{model::*, *};
use crate::engine_client::{Descriptor, QueryContext};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

const ROOT: &str = "child(); child();";
const CHILD: &str = "child body; return;";

fn function(symbol: &str, content: &str, body: Value) -> flow::FunctionFlowIR {
    serde_json::from_value(json!({
        "symbol":symbol,"file":format!("{symbol}.cs"),"build":"Test.csproj",
        "project":"Test","language":"csharp","execution":"synchronous",
        "span":{"start":0,"end":content.len()},"body":body
    }))
    .unwrap()
}

fn call(start: usize) -> Value {
    json!({"kind":"evaluate","value":{"kind":"value","text":"child()",
        "span":{"start":start,"end":start+7},"calls":[{
        "label":"child()","span":{"start":start,"end":start+7},
        "invocation":{"timing":"synchronous","dispatch":"direct","isolated_ordered":true},
        "targets":[{"symbol":"child","certainty":"direct","provenance":"explicit input"}]}]}})
}

fn root() -> flow::FunctionFlowIR {
    function(
        "root",
        ROOT,
        json!([call(0), call(9), {"kind":"action","text":"unlocated step"}]),
    )
}

fn prepared() -> (
    Prepared,
    Vec<codecallnavigator::query::FlowExpansionSelection>,
) {
    let root = root();
    let child = function(
        "child",
        CHILD,
        json!([
            {"kind":"action","text":"child body","span":{"start":0,"end":10}},
            {"kind":"return","span":{"start":12,"end":18}}
        ]),
    );
    let site = flow::call_sites(&root).remove(0);
    let choice = flow::CallChoice {
        site: site.address,
        target: site.site.targets[0].clone(),
    };
    let request = flow::ExpansionRequest::new("root").select_root(choice.clone());
    let result = flow::expand(
        &[
            flow::FunctionSource {
                function: &root,
                content: ROOT,
            },
            flow::FunctionSource {
                function: &child,
                content: CHILD,
            },
        ],
        &request,
        &AtomicBool::new(false),
    )
    .unwrap();
    result.validate().unwrap();
    let root_source = result.occurrences[0].source.unwrap();
    let mut sources: Vec<_> = (0..result.sources.len())
        .map(|index| {
            let snapshot = result.sources.get(flow::SourceId(index)).unwrap();
            Some(crate::source::from_snapshot(
                snapshot.content.clone(),
                Path::new(&snapshot.owner.file),
            ))
        })
        .collect();
    let ready = crate::flow_workspace::Ready {
        chart: result.chart.clone(),
        source: sources[root_source.0].take().unwrap(),
    };
    let keys = (0..result.sources.len())
        .map(|id| format!("explicit-source-{id}"))
        .collect();
    let published = Arc::new(Published {
        badges: super::badges::collect(&result),
        result,
        sources,
        keys,
        root_source,
        query_time: Duration::ZERO,
        layout_time: Duration::ZERO,
    });
    let selections = vec![codecallnavigator::query::FlowExpansionSelection {
        caller: flow::OccurrencePath::default(),
        choice,
    }];
    (Prepared { ready, published }, selections)
}

fn session(state: &State) -> &Session {
    state.flow_workspace.current.as_ref().unwrap()
}

fn state() -> State {
    let mut state = State::new();
    let runtime = crate::runtime::Runtime {
        engine: "/not-invoked/engine".into(),
        resources: "/not-read/resources".into(),
        dot: "/not-invoked/dot".into(),
    };
    state.query_context = Some(QueryContext::new(
        &runtime,
        Descriptor {
            project: "/not-read/project".into(),
            roots: vec![],
            generation: Some("saved-generation".into()),
        },
    ));
    let (prepared, selections) = prepared();
    let ready = Arc::new(crate::flow_workspace::Ready {
        chart: flow::build(&root()).unwrap(),
        source: crate::source::from_snapshot(ROOT.into(), Path::new("root.cs")),
    });
    let mut original = Session::new("root".into(), state.epoch, 0);
    original.ready = Some(ready.clone());
    original.camera.pan = [17.0, 29.0];
    original.camera.zoom = 0.75;
    original.scroll.set([0.0, 91.0]);
    let mut current = original.clone();
    current.ready = Some(Arc::new(prepared.ready));
    current.scroll = Rc::new(Cell::new([0.0, 120.0]));
    current.selected = Some(0);
    current.message = "Current valid status".into();
    current.expansion = Some(Box::new(Mode {
        original: Box::new(original),
        active: Some(prepared.published.root_source),
        published: Some(prepared.published),
        selections,
        pending: None,
        scrolls: Default::default(),
        mounted: None,
    }));
    state
        .flow_workspace
        .cache
        .insert((state.epoch, "ordinary-cache".into()), ready);
    state.flow_workspace.current = Some(current);
    state.flow_workspace.viewport = [650.0, 420.0];
    state
}

fn output(state: &State) -> Output {
    let (prepared, selections) = prepared();
    Output {
        serial: state.flow_workspace.serial,
        epoch: state.epoch,
        symbol: "root".into(),
        generation: "saved-generation".into(),
        selections,
        result: Ok(prepared),
    }
}

#[derive(Debug, PartialEq)]
struct Snapshot {
    ready: usize,
    original_ready: usize,
    published: usize,
    cache: Vec<((u64, String), usize)>,
    selections: Value,
    selected: Option<usize>,
    camera: Value,
    scroll: [f32; 2],
    message: String,
    presentation: u64,
}

fn snapshot(state: &State) -> Snapshot {
    let workspace = &state.flow_workspace;
    let session = session(state);
    let mode = session.expansion.as_ref().unwrap();
    let mut cache: Vec<_> = workspace
        .cache
        .iter()
        .map(|(key, ready)| (key.clone(), Arc::as_ptr(ready) as usize))
        .collect();
    cache.sort_by(|a, b| a.0.cmp(&b.0));
    Snapshot {
        ready: Arc::as_ptr(session.ready.as_ref().unwrap()) as usize,
        original_ready: Arc::as_ptr(mode.original.ready.as_ref().unwrap()) as usize,
        published: Arc::as_ptr(mode.published.as_ref().unwrap()) as usize,
        cache,
        selections: serde_json::to_value(&mode.selections).unwrap(),
        selected: session.selected,
        camera: serde_json::to_value(&session.camera).unwrap(),
        scroll: session.scroll.get(),
        message: session.message.clone(),
        presentation: workspace.presentation,
    }
}

#[derive(Clone, Copy, Debug)]
enum Obsolete {
    Serial,
    Epoch,
    Generation,
    Root,
}

fn obsolete(output: &mut Output, reason: Obsolete) {
    match reason {
        Obsolete::Serial => output.serial += 1,
        Obsolete::Epoch => output.epoch += 1,
        Obsolete::Generation => output.generation = "older-generation".into(),
        Obsolete::Root => output.symbol = "another-root".into(),
    }
}

fn rejects_obsolete(error: bool) {
    for reason in [
        Obsolete::Serial,
        Obsolete::Epoch,
        Obsolete::Generation,
        Obsolete::Root,
    ] {
        let mut state = state();
        let before = snapshot(&state);
        let mut output = output(&state);
        obsolete(&mut output, reason);
        if error {
            output.result = Err("Obsolete error must remain hidden".into());
        }
        assert!(!publication::publish(&mut state, output), "{reason:?}");
        assert_eq!(snapshot(&state), before, "{reason:?}");
    }
}

#[test]
fn stale_ready_publications_preserve_valid_session_choices_and_cache() {
    rejects_obsolete(false);
}

#[test]
fn stale_errors_cannot_replace_current_status_or_choices() {
    rejects_obsolete(true);
}

#[test]
fn current_error_and_cancel_retain_the_last_published_chart() {
    let mut state = state();
    let mut before = snapshot(&state);
    let mut failed = output(&state);
    failed.result = Err("Useful current error".into());
    assert!(publication::publish(&mut state, failed));
    before.message = "Useful current error".into();
    assert_eq!(snapshot(&state), before);
    let late = output(&state);
    state.cancel_flow_expansion();
    let cancelled = snapshot(&state);
    assert!(cancelled.message.contains("cancelled"));
    assert!(!publication::publish(&mut state, late));
    assert_eq!(snapshot(&state), cancelled);
}

#[test]
fn successful_publication_keeps_ordinary_cache_and_resets_transient_selection() {
    let mut state = state();
    let before = snapshot(&state);
    let answer = output(&state);
    let choices = serde_json::to_value(&answer.selections).unwrap();
    assert!(publication::publish(&mut state, answer));
    let after = snapshot(&state);
    assert_eq!(after.cache, before.cache);
    assert_eq!(after.selections, choices);
    assert_ne!(after.ready, before.ready);
    assert_ne!(after.published, before.published);
    assert_eq!(after.selected, None);
    assert!(after.message.is_empty());
    assert!(state.flow_workspace.fit_pending);
    let mode = session(&state).expansion.as_ref().unwrap();
    assert_eq!(
        mode.active,
        Some(mode.published.as_ref().unwrap().root_source)
    );
    assert_eq!(
        Arc::as_ptr(mode.original.ready.as_ref().unwrap()) as usize,
        before.original_ready
    );
    let current = state.flow_workspace.current.as_mut().unwrap();
    current.expansion.as_mut().unwrap().mounted = None;
    current.scroll.set([0.0, 120.0]);
    let measured_scroll = current.scroll.clone();
    let answer = output(&state);
    assert!(publication::publish(&mut state, answer));
    let current = session(&state);
    assert!(Rc::ptr_eq(&current.scroll, &measured_scroll));
    assert_eq!(current.scroll.get(), [0.0, 120.0]);
    assert!(current.expansion.as_ref().unwrap().mounted.is_none());
}

#[test]
fn child_source_selection_and_unlocated_step_keep_owner_and_scroll() {
    let mut state = state();
    let flow = session(&state).expanded_flow().unwrap();
    let child = flow
        .chart
        .steps
        .iter()
        .position(|step| step.label == "child body")
        .unwrap();
    let zero = flow
        .chart
        .steps
        .iter()
        .position(|step| step.span == flow::SourceSpan::default())
        .unwrap();
    state.select_flow_step(child, &mut rune::Tx::default());
    assert_eq!(session(&state).displayed_file(), Some("child.cs"));
    assert_eq!(session(&state).displayed_source().unwrap().text, CHILD);
    let key = session(&state).scroll_key.clone();
    state.expansion_source_mounted(&key, &mut rune::Tx::default());
    session(&state).scroll.set([0.0, 83.0]);
    state.select_flow_step(zero, &mut rune::Tx::default());
    assert_eq!(session(&state).selected_source_span(), None);
    assert_eq!(session(&state).displayed_file(), Some("child.cs"));
    assert_eq!(session(&state).scroll_key, key);
    assert_eq!(session(&state).scroll.get(), [0.0, 83.0]);
}

#[test]
fn collapse_all_restores_the_original_ordinary_session() {
    let mut state = state();
    let original = session(&state).expansion.as_ref().unwrap().original.clone();
    let original_scroll = original.scroll.get();
    let cache = snapshot(&state).cache;
    state.collapse_all_flow_expansion();
    let restored = session(&state);
    assert!(!restored.expansion_mode());
    assert!(Arc::ptr_eq(
        restored.ready.as_ref().unwrap(),
        original.ready.as_ref().unwrap()
    ));
    assert_eq!(restored.selected, original.selected);
    assert_eq!(restored.scroll_key, original.scroll_key);
    assert_eq!(restored.scroll.get(), original_scroll);
    assert_eq!(
        serde_json::to_value(&restored.camera).unwrap(),
        serde_json::to_value(&original.camera).unwrap()
    );
    assert_eq!(
        state
            .flow_workspace
            .cache
            .values()
            .map(|ready| Arc::as_ptr(ready) as usize)
            .collect::<Vec<_>>(),
        cache
            .iter()
            .map(|(_, pointer)| *pointer)
            .collect::<Vec<_>>()
    );
}
