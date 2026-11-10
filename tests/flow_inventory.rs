use codegraph::flow::{self, FlowInventory, FunctionFlowIR, Statement, StatementKind};
use serde_json::json;
use std::collections::HashSet;
fn inventory(symbol: &str) -> FlowInventory {
    FlowInventory {
        version: flow::INVENTORY_VERSION,
        functions: vec![FunctionFlowIR {
            symbol: symbol.into(),
            ..Default::default()
        }],
    }
}

#[test]
fn supported_inventories_merge_across_unsupported_languages_and_reject_conflicts() {
    let mut merged = None;
    flow::merge_inventory(&mut merged, None).unwrap();
    flow::merge_inventory(&mut merged, Some(inventory("java"))).unwrap();
    flow::merge_inventory(&mut merged, None).unwrap();
    flow::merge_inventory(&mut merged, Some(inventory("typescript"))).unwrap();
    flow::merge_inventory(&mut merged, Some(inventory("java"))).unwrap();
    assert_eq!(merged.as_ref().unwrap().functions.len(), 2);
    let mut changed = inventory("java");
    changed.functions[0].file = "different".into();
    assert!(flow::merge_inventory(&mut merged, Some(changed)).is_err());
}

#[test]
fn pruning_excluded_symbols_visits_nested_callback_and_short_circuit_targets() {
    let function:FunctionFlowIR=serde_json::from_value(json!({"symbol":"keep","body":[{"kind":"return","value":{"kind":"and","left":{"kind":"value","text":"condition"},"right":{"kind":"value","text":"register","regions":[{"kind":"region","label":"callback","deferred":true,"body":[{"kind":"action","text":"call","calls":[{"label":"call","targets":[{"symbol":"keep","certainty":"direct","provenance":"compiler"},{"symbol":"excluded"}]}]}]}]}}}]})).unwrap();
    let mut inventory = inventory("excluded");
    inventory.functions.push(function);
    inventory.retain_symbols(&HashSet::from(["keep"]));
    assert_eq!(inventory.functions.len(), 1);
    let mut targets = Vec::new();
    flow::visit_calls_mut(&mut inventory.functions[0], &mut |call| {
        targets.extend(call.targets.clone());
        assert!(call.diagnostic.is_some());
    });
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].symbol, "keep");
    assert_eq!(targets[0].certainty, "direct");
}

#[test]
fn oversized_function_has_a_visible_limit_error_instead_of_a_truncated_control_flow() {
    let body = (0..flow::MAX_STEPS)
        .map(|_| Statement {
            span: Default::default(),
            kind: StatementKind::Action {
                text: "step".into(),
                calls: Vec::new(),
            },
        })
        .collect();
    let function = FunctionFlowIR {
        body,
        ..Default::default()
    };
    let error = flow::build(&function).unwrap_err();
    assert!(error.to_string().contains("1500 steps"));
}
