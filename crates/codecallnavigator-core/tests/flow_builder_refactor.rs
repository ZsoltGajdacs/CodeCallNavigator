//! Full pre-refactor serialization complements the existing completion/path tests.
//! Explicit IR inputs and immutable expected output keep this a pure core unit test.
use codecallnavigator_core::flow::{self, FunctionFlowIR};

#[test]
fn ordinary_flow_serialization_preserves_completion_call_and_pruning_order() {
    let inputs: Vec<FunctionFlowIR> =
        serde_json::from_str(include_str!("fixtures/builder-refactor/inputs.json")).unwrap();
    let charts: Vec<_> = inputs
        .iter()
        .map(|function| flow::build(function).unwrap())
        .collect();
    assert_eq!(
        serde_json::to_string(&charts).unwrap(),
        include_str!("fixtures/builder-refactor/charts.json")
    );
}
