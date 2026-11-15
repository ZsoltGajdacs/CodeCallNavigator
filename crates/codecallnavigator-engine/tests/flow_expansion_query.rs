//! Public CLI query journeys over saved artifacts need no compiler or live source files.
#[path = "flow_expansion_query/support.rs"]
mod support;
use codecallnavigator_core::flow::{self, ExpandedFlow, OccurrenceOutcome};
use serde_json::json;
use support::*;

#[test]
fn expansion_cli_is_generation_bound_and_keeps_ordinary_flow_unchanged() {
    let fixture = Saved::new();
    let before = std::fs::read(&fixture.path).unwrap();
    let ordinary = fixture.ask(json!({"kind":"flow","symbol":fixture.root}), None);
    let operation = fixture.operation(vec![]);
    assert_eq!(
        fixture.ask(operation.clone(), None)["error"]["code"],
        "invalid_request"
    );
    assert_eq!(
        fixture.ask(operation.clone(), Some("stale"))["error"]["code"],
        "generation_mismatch"
    );
    let mut future = operation.clone();
    future["expansion_version"] = json!(2);
    assert_eq!(
        fixture.ask(future, Some(&fixture.digest))["error"]["code"],
        "unsupported_expansion_version"
    );
    let zero = fixture.ask(operation, Some(&fixture.digest));
    let portable: flow::FunctionFlowIR = serde_json::from_value(
        fixture.saved.graph["flow_inventory"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["symbol"] == fixture.root)
            .unwrap()
            .clone(),
    )
    .unwrap();
    // Expansion retains logical cached ownership; ordinary Flow retains hydrated paths.
    assert_eq!(
        zero["result"]["chart"],
        serde_json::to_value(flow::build(&portable).unwrap()).unwrap()
    );
    let selected = fixture.ask(
        fixture.operation(vec![fixture.selection()]),
        Some(&fixture.digest),
    );
    assert!(selected["error"].is_null(), "{selected}");
    assert_eq!(
        fixture.ask_session(fixture.operation(vec![fixture.selection()])),
        selected
    );
    let result: ExpandedFlow = serde_json::from_value(selected["result"].clone()).unwrap();
    result.validate().unwrap();
    assert_eq!(result.occurrences[1].outcome, OccurrenceOutcome::Expanded);
    assert_eq!(result.sources.len(), 2);
    assert_eq!(selected["generation"], fixture.digest);
    assert_eq!(std::fs::read(&fixture.path).unwrap(), before);
    assert_eq!(
        fixture.ask(json!({"kind":"flow","symbol":fixture.root}), None),
        ordinary
    );
}

#[test]
fn expansion_cli_reports_stale_choices_limits_and_escaped_transport_bounds() {
    let fixture = Saved::new();
    let mut stale = fixture.selection();
    stale["choice"]["target"]["provenance"] = json!("changed");
    assert_eq!(
        fixture.ask(fixture.operation(vec![stale]), Some(&fixture.digest))["error"]["code"],
        "invalid_request"
    );
    let mut invalid_path = fixture.selection();
    invalid_path["choice"]["site"]["call_slot"] = json!(999);
    assert_eq!(
        fixture.ask(fixture.operation(vec![invalid_path]), Some(&fixture.digest))["error"]["code"],
        "invalid_request"
    );
    let mut limited = fixture.operation(vec![fixture.selection()]);
    limited["limits"] = json!({"depth":0});
    let result = fixture.ask(limited, Some(&fixture.digest));
    assert_eq!(
        result["result"]["occurrences"][1]["outcome"]["reason"],
        json!({"kind":"limit","limit":"depth"})
    );
    let large = fixture.with_content("\"".repeat(13 * 1024 * 1024));
    let response = large.ask(large.operation(vec![]), Some(&large.digest));
    assert_eq!(response["error"]["code"], "response_too_large");
    assert!(serde_json::to_vec(&response).unwrap().len() < 1024);
}
