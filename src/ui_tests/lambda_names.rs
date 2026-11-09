//! Readable Java symbols through the production canvas, search and inspector.
use crate::{app::State, jobs, projection::Grouping, scenes, view};
use codegraph::graph::GraphIndex;
use rune::{Env, Runtime};
use std::sync::Arc;

const NAME: &str = "COST_TYPE_VALIDATION";
const SIGNATURE: &str =
    "fixture.ValidationFunctions:lambda$static$11(java.lang.String) -> java.lang.String";

fn runtime() -> Runtime<State> {
    let mut state = State::new();
    scenes::install(&mut state);
    let mut graph = (*state.graph.as_ref().unwrap().data).clone();
    let lambda = &mut graph.nodes[2];
    lambda.label = NAME.into();
    lambda.signature = SIGNATURE.into();
    lambda.language = "java".into();
    state.graph = Some(Arc::new(GraphIndex::new(Arc::new(graph)).unwrap()));
    state.nav.set_grouping(Grouping::Method);
    state.request_layout();
    let mut runtime = Runtime::new(state, view::view, (1420.0, 900.0), Env::new(true, 1.0));
    runtime.poll = Some(jobs::poll);
    settle(&mut runtime);
    runtime
}

fn settle(runtime: &mut Runtime<State>) {
    for _ in 0..30 {
        runtime.frame(1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn has_option(runtime: &Runtime<State>, label: &str) -> bool {
    runtime
        .elements()
        .iter()
        .any(|element| element.role == "option" && element.label.as_deref() == Some(label))
}

#[test]
fn friendly_lambda_name_is_drawn_and_searchable_by_either_name() {
    let mut runtime = runtime();
    assert!(runtime.elements().iter().any(|element| {
        element
            .key
            .as_deref()
            .is_some_and(|key| key.starts_with("graph-node-"))
            && element
                .label
                .as_deref()
                .is_some_and(|label| label.starts_with(NAME))
    }));
    for query in ["cost_type_validation", "lambda$static$11"] {
        runtime.update(|state, _| {
            state.open_search();
            state.query = rune::TextEdit::new(query);
            state.request_search();
        });
        settle(&mut runtime);
        assert_eq!(runtime.state.hits, vec![2]);
        assert!(
            has_option(&runtime, NAME),
            "Missing friendly search result for {query}"
        );
        assert!(!has_option(&runtime, SIGNATURE));
    }
}

#[test]
fn neighbors_and_relationships_use_friendly_names_and_inspector_keeps_signature() {
    let mut runtime = runtime();
    runtime.update(|state, _| state.select(1, false));
    settle(&mut runtime);
    assert!(
        has_option(&runtime, NAME),
        "Callee list must show the source name"
    );
    assert!(
        runtime.elements().iter().any(|element| {
            element.role == "option"
                && element.label.as_deref().is_some_and(|label| {
                    label.ends_with(NAME) && label.contains("Virtual / interface call")
                })
        }),
        "Relationship list must show the source name"
    );
    runtime.update(|state, _| state.select(2, false));
    settle(&mut runtime);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some(NAME))
    );
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| element.text.as_deref() == Some(SIGNATURE))
    );
}

#[test]
fn virtual_call_details_explain_the_declared_target_without_reclassifying_it() {
    let mut runtime = runtime();
    runtime.update(|state, _| {
        state.select(1, false);
        let edge = state
            .graph
            .as_ref()
            .unwrap()
            .data
            .edges
            .iter()
            .position(|edge| edge.certainty == "dispatch")
            .unwrap();
        state.selected_edge = Some(edge as u32);
    });
    settle(&mut runtime);
    assert!(
        runtime.elements().iter().any(|element| {
            element.text.as_deref() == Some("Certainty: Virtual / interface call")
        })
    );
    let edge = runtime.state.selected_edge.unwrap();
    assert_eq!(
        runtime.state.graph.as_ref().unwrap().data.edges[edge as usize].certainty,
        "dispatch"
    );
}
