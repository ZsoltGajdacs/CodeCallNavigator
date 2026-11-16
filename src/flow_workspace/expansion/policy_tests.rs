//! Pure UI selection edits consume actual core occurrence/site identities, not reconstructed spans.
use super::{model::*, *};
use crate::{flow_workspace::Ready, source};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
const TEXT: &str = "call();call();";
fn function(symbol: &str, body: Value) -> flow::FunctionFlowIR {
    serde_json::from_value(json!({"symbol":symbol,"file":format!("{symbol}.cs"),"build":"App.csproj","project":"App","language":"csharp","execution":"synchronous","span":{"start":0,"end":TEXT.len()},"body":body})).unwrap()
}
fn call(start: usize, candidate: bool) -> Value {
    let targets = if candidate {
        json!([{"symbol":"child","certainty":"possible","provenance":"compiler A"},{"symbol":"other","certainty":"possible","provenance":"compiler B"}])
    } else {
        json!([{"symbol":"child","certainty":"direct","provenance":"compiler direct"}])
    };
    json!({"kind":"evaluate","value":{"kind":"value","text":"call()","span":{"start":start,"end":start+6},"calls":[{"label":"call()","span":{"start":start,"end":start+6},"invocation":{"timing":"synchronous","dispatch":if candidate{"candidate"}else{"direct"},"isolated_ordered":true},"targets":targets}]}})
}
fn selection(
    function: &flow::FunctionFlowIR,
    index: usize,
    caller: flow::OccurrencePath,
) -> FlowExpansionSelection {
    let site = flow::call_sites(function).remove(index);
    FlowExpansionSelection {
        caller,
        choice: flow::CallChoice {
            site: site.address,
            target: site.site.targets[0].clone(),
        },
    }
}
fn fixture() -> Session {
    let root = function("root", json!([call(0, true), call(7, true)]));
    let child = function("child", json!([call(0, false)]));
    let first = selection(&root, 0, Default::default());
    let second = selection(&root, 1, Default::default());
    let nested = selection(&child, 0, flow::OccurrencePath(vec![first.choice.clone()]));
    assembled(&root, &child, vec![first, nested, second])
}
fn assembled(
    root: &flow::FunctionFlowIR,
    child: &flow::FunctionFlowIR,
    selections: Vec<FlowExpansionSelection>,
) -> Session {
    assembled_from(
        root,
        child,
        selections,
        FixtureSources {
            root: TEXT,
            child: TEXT,
        },
    )
}
struct FixtureSources<'a> {
    root: &'a str,
    child: &'a str,
}
fn assembled_from(
    root: &flow::FunctionFlowIR,
    child: &flow::FunctionFlowIR,
    selections: Vec<FlowExpansionSelection>,
    content: FixtureSources<'_>,
) -> Session {
    let mut request = flow::ExpansionRequest::new("root");
    for selection in &selections {
        request = request.select(selection.caller.clone(), selection.choice.clone());
    }
    let result = flow::expand(
        &[
            flow::FunctionSource {
                function: root,
                content: content.root,
            },
            flow::FunctionSource {
                function: child,
                content: content.child,
            },
        ],
        &request,
        &AtomicBool::new(false),
    )
    .unwrap();
    result.validate().unwrap();
    let root_source = result.occurrences[0].source.unwrap();
    let sources = (0..result.sources.len())
        .map(|index| {
            let snapshot = result.sources.get(flow::SourceId(index)).unwrap();
            Some(source::from_snapshot(
                snapshot.content.clone(),
                Path::new(&snapshot.owner.file),
            ))
        })
        .collect();
    let ready = Arc::new(Ready {
        chart: result.chart.clone(),
        source: source::from_snapshot(content.root.into(), Path::new("root.cs")),
    });
    let mut session = Session::new("root".into(), 0, 0);
    session.ready = Some(Arc::new(Ready {
        chart: flow::build(root).unwrap(),
        source: source::from_snapshot(content.root.into(), Path::new("root.cs")),
    }));
    let original = Box::new(session.clone());
    session.ready = Some(ready);
    session.scroll = Rc::new(Cell::new(session.scroll.get()));
    session.expansion = Some(Box::new(Mode {
        original,
        published: Some(Arc::new(Published {
            badges: super::badges::collect(&result),
            result,
            sources,
            keys: vec!["root-owner".into(), "child-owner".into()],
            root_source,
            query_time: Duration::ZERO,
            layout_time: Duration::ZERO,
        })),
        selections,
        pending: None,
        scrolls: Default::default(),
        mounted: None,
        active: Some(root_source),
    }));
    session
}
#[test]
fn collapsing_a_recursive_marker_removes_only_its_exact_nested_selection() {
    let mut session = fixture();
    let result = session.expanded_flow().unwrap();
    let stopped = result
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
    let expected = stopped.path.clone();
    session.selected = stopped.boundaries.marker;
    assert_eq!(session.selected_occurrence(), Some(&expected));
    assert_eq!(
        session.marker_badge(session.selected.unwrap()),
        Some("Cycle")
    );
    let edited = session.without_expansion_path(&expected);
    assert_eq!(edited.len(), 2);
    assert!(edited.iter().all(|selection| selection.caller.0.is_empty()));
    assert!(
        edited
            .iter()
            .all(|selection| selection.choice.target.symbol == "child")
    );
}
#[test]
fn replacing_an_explicit_candidate_retains_siblings_and_removes_old_descendants() {
    let mut session = fixture();
    let mode = session.expansion.as_ref().unwrap();
    let first = mode.selections[0].clone();
    let sibling = mode.selections[2].clone();
    session.selected = session
        .expanded_flow()
        .unwrap()
        .step_origins
        .iter()
        .position(|origin| {
            origin.occurrence == flow::OccurrenceId(0) && origin.calls.contains(&first.choice.site)
        });
    let (caller, choice) = session.expansion_choice(0, 1).unwrap();
    let edited = session.replacing_expansion_choice(&FlowExpansionSelection { caller, choice });
    assert_eq!(edited.len(), 2);
    assert!(
        edited
            .iter()
            .any(|selection| selection.caller == sibling.caller
                && selection.choice == sibling.choice)
    );
    let replacement = edited
        .iter()
        .find(|selection| selection.choice.site == first.choice.site)
        .unwrap();
    assert_eq!(
        (
            replacement.choice.target.symbol.as_str(),
            replacement.choice.target.certainty.as_str(),
            replacement.choice.target.provenance.as_str()
        ),
        ("other", "possible", "compiler B")
    );
    assert!(edited.iter().all(|selection| selection.caller.0.is_empty()));
}
#[test]
fn a_shared_stopped_marker_requires_a_specific_call_choice_instead_of_guessing() {
    let mut raw = call(0, false);
    let mut site = raw["value"]["calls"][0].clone();
    site["invocation"] = json!({});
    raw["value"]["calls"] = json!([site.clone(), site]);
    let root = function("root", json!([raw]));
    let child = function("child", json!([]));
    let selections = vec![
        selection(&root, 0, Default::default()),
        selection(&root, 1, Default::default()),
    ];
    let mut session = assembled(&root, &child, selections);
    let result = session.expanded_flow().unwrap();
    assert_eq!(result.occurrences.len(), 3);
    assert_eq!(
        result.occurrences[1].boundaries.marker,
        result.occurrences[2].boundaries.marker
    );
    session.selected = result.occurrences[1].boundaries.marker;
    assert!(session.selected_occurrence().is_none());
    assert_eq!(
        session.marker_badge(session.selected.unwrap()),
        Some("Stopped choices (2)")
    );
    assert_eq!(
        [
            session.flow_choice_selected(0, 0),
            session.flow_choice_selected(1, 0)
        ],
        [true, true]
    );
    let (mut path, choice) = session.expansion_choice(0, 0).unwrap();
    path.0.push(choice);
    let edited = session.without_expansion_path(&path);
    assert_eq!(edited.len(), 1);
    assert_eq!(edited[0].choice.site.call_slot, 1);
}

fn navigation_state(previous: Session) -> State {
    use codecallnavigator::{
        graph::GraphIndex,
        model::{GraphData, Node},
    };
    let mut state = State::new();
    let graph = GraphData {
        nodes: ["root", "child", "other", "next"]
            .into_iter()
            .map(|symbol| Node {
                id: symbol.into(),
                label: symbol.into(),
                language: "csharp".into(),
                file: format!("{symbol}.cs"),
                build: "App.csproj".into(),
                project: "App".into(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    state.graph = Some(Arc::new(GraphIndex::new(Arc::new(graph)).unwrap()));
    let next = function("next", json!([]));
    let cached = Arc::new(Ready {
        chart: flow::build(&next).unwrap(),
        source: source::from_snapshot(TEXT.into(), Path::new("next.cs")),
    });
    state
        .flow_workspace
        .cache
        .insert((state.epoch, "next".into()), cached);
    state.flow_workspace.current = Some(previous);
    // Both endpoints already have real core-built Ready values; this exercises
    // history policy without starting a query, compiler, filesystem or layout job.
    state
}
fn navigate_back(previous: Session) -> Session {
    let mut state = navigation_state(previous);
    state.open_flow("next".into());
    state.flow_back();
    state.flow_workspace.current.take().unwrap()
}
#[test]
fn leaving_an_unpublished_expansion_restores_ordinary_history_instead_of_a_ghost_mode() {
    let mut session = fixture();
    let mode = session.expansion.as_mut().unwrap();
    let original = mode.original.ready.as_ref().unwrap().clone();
    let offset = mode.original.scroll.get();
    mode.pending = Some(vec![]);
    mode.published = None;
    mode.selections.clear();
    mode.active = None;
    session.ready = Some(original.clone());
    session.message = "Preparing expanded flow…".into();
    let restored = navigate_back(session);
    assert!(
        !restored.expansion_mode(),
        "Back must not restore a mode whose addressed request was cancelled before publication"
    );
    assert!(Arc::ptr_eq(restored.ready.as_ref().unwrap(), &original));
    assert_eq!(restored.scroll.get(), offset);
}
#[test]
fn leaving_a_pending_edit_keeps_the_last_published_view_and_clears_only_pending_status() {
    let mut session = fixture();
    let ready = session.ready.as_ref().unwrap().clone();
    let choices = serde_json::to_value(&session.expansion.as_ref().unwrap().selections).unwrap();
    session.camera.pan = [31.0, 41.0];
    session.scroll.set([0.0, 83.0]);
    session.expansion.as_mut().unwrap().pending = Some(vec![]);
    session.message = "Preparing expanded flow…".into();
    let restored = navigate_back(session);
    assert!(Arc::ptr_eq(restored.ready.as_ref().unwrap(), &ready));
    assert_eq!(
        serde_json::to_value(&restored.expansion.as_ref().unwrap().selections).unwrap(),
        choices
    );
    assert_eq!(
        (
            restored.camera.pan,
            restored.scroll.get(),
            restored.message.as_str()
        ),
        ([31.0, 41.0], [0.0, 83.0], ""),
        "Back preserves the published viewport and clears cancelled pending status"
    );
}

#[derive(Clone, Copy)]
enum PresentedCase {
    Expand,
    CollapseCall,
    CollapseOccurrence,
}
fn presented_state(case: PresentedCase) -> State {
    let mut session = fixture();
    let result = session.expanded_flow().unwrap();
    session.selected = Some(match case {
        PresentedCase::CollapseOccurrence => result
            .occurrences
            .iter()
            .find_map(|occurrence| occurrence.boundaries.marker)
            .unwrap(),
        _ => result
            .step_origins
            .iter()
            .position(|origin| {
                origin.occurrence == flow::OccurrenceId(0) && !origin.calls.is_empty()
            })
            .unwrap(),
    });
    navigation_state(session)
}
fn presented_action(state: &State, case: PresentedCase) -> super::actions::PresentedAction {
    match case {
        PresentedCase::Expand => state.present_flow_call(0, 1).unwrap(),
        PresentedCase::CollapseCall => state.present_flow_call(0, 0).unwrap(),
        PresentedCase::CollapseOccurrence => state.present_flow_occurrence().unwrap(),
    }
}
#[derive(Clone, Copy)]
enum ChangedPresentation {
    Step,
    Ready,
    Result,
    Serial,
    Epoch,
    Pending,
}
fn change_presentation(state: &mut State, change: ChangedPresentation) {
    match change {
        ChangedPresentation::Step => {
            state.flow_workspace.current.as_mut().unwrap().selected = Some(0)
        }
        ChangedPresentation::Ready => {
            let session = state.flow_workspace.current.as_mut().unwrap();
            session.ready = Some(Arc::new(Ready {
                chart: session.ready.as_ref().unwrap().chart.clone(),
                source: source::from_snapshot(TEXT.into(), Path::new("root.cs")),
            }));
        }
        ChangedPresentation::Result => {
            state
                .flow_workspace
                .current
                .as_mut()
                .unwrap()
                .expansion
                .as_mut()
                .unwrap()
                .published = fixture().expansion.unwrap().published
        }
        ChangedPresentation::Serial => state.flow_workspace.serial += 1,
        ChangedPresentation::Epoch => state.epoch += 1,
        ChangedPresentation::Pending => {
            state
                .flow_workspace
                .current
                .as_mut()
                .unwrap()
                .expansion
                .as_mut()
                .unwrap()
                .pending = Some(vec![])
        }
    }
}
#[test]
fn presented_expansion_actions_require_the_exact_ready_step_result_and_request_binding() {
    for case in [
        PresentedCase::Expand,
        PresentedCase::CollapseCall,
        PresentedCase::CollapseOccurrence,
    ] {
        let state = presented_state(case);
        let action = presented_action(&state, case);
        assert!(
            action.edits(&state).is_some(),
            "Current presented actions must remain actionable"
        );
        for changed in [
            ChangedPresentation::Step,
            ChangedPresentation::Ready,
            ChangedPresentation::Result,
            ChangedPresentation::Serial,
            ChangedPresentation::Epoch,
            ChangedPresentation::Pending,
        ] {
            let mut state = presented_state(case);
            let action = presented_action(&state, case);
            change_presentation(&mut state, changed);
            assert!(
                action.edits(&state).is_none(),
                "Obsolete controls cannot select a different call or subtree"
            );
        }
    }
}

#[test]
fn cancelling_an_unpublished_mode_restores_the_exact_ordinary_session() {
    let mut session = fixture();
    let original = session.expansion.as_ref().unwrap().original.clone();
    let camera = serde_json::to_value(&original.camera).unwrap();
    let offset = original.scroll.get();
    let mode = session.expansion.as_mut().unwrap();
    mode.published = None;
    mode.pending = None;
    mode.selections.clear();
    session.ready = original.ready.clone();
    session.message = "Preparing expanded flow…".into();
    let mut state = navigation_state(session);
    state.flow_workspace.fit_pending = true;
    state.cancel_flow_expansion();
    let restored = state.flow_workspace.current.as_ref().unwrap();
    assert_eq!(
        (
            restored.expansion_mode(),
            Arc::ptr_eq(
                restored.ready.as_ref().unwrap(),
                original.ready.as_ref().unwrap()
            )
        ),
        (false, true),
        "Initial cancellation restores the exact ordinary ready view"
    );
    assert_eq!(
        (
            restored.selected,
            &restored.scroll_key,
            restored.scroll.get(),
            serde_json::to_value(&restored.camera).unwrap()
        ),
        (original.selected, &original.scroll_key, offset, camera)
    );
    assert_eq!(
        (
            state.flow_workspace.fit_pending,
            state.flow_workspace.mount_restore.is_some()
        ),
        (false, true)
    );
}

#[test]
fn resized_expansion_toolbar_keeps_drawn_and_target_controls_inside_viewport() {
    let mut state = navigation_state(fixture());
    let mut data = state.graph.as_ref().unwrap().data.to_graph_data();
    data.nodes[0].signature = "ExpansionDesktop.Root.Run(System.Int32)".into();
    state.graph = Some(Arc::new(
        codecallnavigator::graph::GraphIndex::new(Arc::new(data)).unwrap(),
    ));
    let mut runtime = rune::Runtime::new(
        state,
        crate::view::view,
        (1420.0, 900.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..6 {
        runtime.frame(1.0 / 60.0);
    }
    runtime.set_viewport((1050.0, 680.0), 1.0);
    runtime.set_titlebar_inset(rune::tooling::Chrome::DEFAULT.titlebar_inset());
    for _ in 0..6 {
        runtime.frame(1.0 / 60.0);
    }
    let elements = runtime.elements();
    let geometry: Vec<_> = elements
        .iter()
        .filter(|element| {
            matches!(
                element.key.as_deref(),
                Some(
                    "kit-root"
                        | "flow-workspace"
                        | "flow-toolbar"
                        | "flow-source-pane"
                        | "flow-fit"
                )
            )
        })
        .map(|element| (&element.key, element.rect, element.target))
        .collect();
    let fit = elements
        .iter()
        .find(|element| element.key.as_deref() == Some("flow-fit"))
        .unwrap();
    assert!(
        fit.target[0] + fit.target[2] <= 1050.5,
        "Target outside {:?}: {geometry:?}",
        runtime.viewport
    );
    assert!(
        fit.rect[0] + fit.rect[2] <= 1050.5,
        "Drawn outside {:?}: {geometry:?}",
        runtime.viewport
    );
}

fn measured_source_runtime(mut session: Session) -> rune::Runtime<State> {
    session.scroll_key = session.expansion_key().unwrap().into();
    let mut runtime = rune::Runtime::new(
        navigation_state(session),
        crate::view::view,
        (1050.0, 680.0),
        rune::Env::new(true, 1.0),
    );
    for _ in 0..6 {
        runtime.frame(1.0 / 60.0);
    }
    runtime
}

#[test]
fn same_source_selection_centers_an_unrealized_row_after_publication() {
    let content = format!("{}call();call();", "\n".repeat(60));
    let mut root = function("root", json!([call(60, false), call(67, false)]));
    root.span.end = content.len();
    let child = function("child", json!([call(0, false)]));
    let first = selection(&root, 0, Default::default());
    let session = assembled_from(
        &root,
        &child,
        vec![first],
        FixtureSources {
            root: &content,
            child: TEXT,
        },
    );
    let selected = session
        .ready
        .as_ref()
        .unwrap()
        .chart
        .steps
        .iter()
        .find(|step| step.span.start == 67 && !step.calls.is_empty())
        .unwrap()
        .id;
    let mut runtime = measured_source_runtime(session);
    let key = runtime
        .state
        .flow_workspace
        .current
        .as_ref()
        .unwrap()
        .scroll_key
        .clone();
    runtime.update(move |_, tx| tx.scroll_to_instant(key, 0.0, 120.0));
    for _ in 0..6 {
        runtime.frame(1.0 / 60.0);
    }
    runtime.update(move |state, tx| state.select_flow_step(selected, tx));
    for _ in 0..6 {
        runtime.frame(1.0 / 60.0);
    }
    let session = runtime.state.flow_workspace.current.as_ref().unwrap();
    let row = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("flow-source-line-60"))
        .unwrap();
    assert!(
        row.fill.a > 0.1 && row.visible[2] > 0.0 && row.visible[3] > 0.0,
        "span={:?} scroll={:?} mount={:?} row={:?} visible={:?} fill={:?}",
        session.selected_source_span(),
        session.scroll.get(),
        session
            .expansion
            .as_ref()
            .unwrap()
            .mounted
            .as_ref()
            .map(|mount| (&mount.key, mount.offset, mount.center)),
        row.rect,
        row.visible,
        row.fill
    );
}

#[test]
fn switching_source_centers_the_new_owned_row_without_replaying_saved_offset() {
    let content = format!("{}call();call();", "\n".repeat(60));
    let mut root = function("root", json!([call(60, false), call(67, false)]));
    root.span.end = content.len();
    let mut child = function("child", json!([call(60, false)]));
    child.span.end = content.len();
    let first = selection(&root, 0, Default::default());
    let session = assembled_from(
        &root,
        &child,
        vec![first],
        FixtureSources {
            root: &content,
            child: &content,
        },
    );
    let result = session.expanded_flow().unwrap();
    let selected = result
        .step_origins
        .iter()
        .zip(&result.chart.steps)
        .find(|(origin, step)| origin.occurrence != flow::OccurrenceId(0) && !step.calls.is_empty())
        .unwrap()
        .1
        .id;
    let mut runtime = measured_source_runtime(session);
    runtime.update(move |state, tx| state.select_flow_step(selected, tx));
    for _ in 0..6 {
        runtime.frame(1.0 / 60.0);
    }
    let session = runtime.state.flow_workspace.current.as_ref().unwrap();
    assert!(
        session.expansion.as_ref().unwrap().mounted.is_none(),
        "New source mount must be consumed"
    );
    let row = runtime
        .elements()
        .into_iter()
        .find(|element| !element.exiting && element.key.as_deref() == Some("flow-source-line-60"))
        .expect("The selected row must be realized after the new source mount");
    assert!(
        row.fill.a > 0.1 && row.visible[2] > 0.0 && row.visible[3] > 0.0,
        "scroll={:?} row={:?} visible={:?} fill={:?}",
        session.scroll.get(),
        row.rect,
        row.visible,
        row.fill
    );
}
