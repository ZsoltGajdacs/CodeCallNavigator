//! Compiler facts round-trip without manufacturing execution eligibility from graph hints.
use codecallnavigator_core::flow::{self, CallSite, FunctionFlowIR};
use serde_json::{Value, json};
fn raw() -> Value {
    json!({"symbol":"caller","file":"/repo/F.cs","build":"/repo/App.csproj","project":"App","language":"csharp","span":{"start":0,"end":3},"body":[{"span":{"start":0,"end":3},"kind":"evaluate","value":{"kind":"value","text":"f()","span":{"start":0,"end":3},"calls":[{"span":{"start":0,"end":3},"label":"f()","targets":[{"symbol":"target","certainty":"direct","provenance":"compiler"}]}]}}]})
}
fn function(value: Value) -> FunctionFlowIR {
    serde_json::from_value(value).unwrap()
}
fn call(value: &mut Value) -> &mut Value {
    &mut value["body"][0]["value"]["calls"][0]
}
fn proven() -> Value {
    let mut value = raw();
    value["execution"] = "synchronous".into();
    call(&mut value)["invocation"] =
        json!({"timing":"synchronous","dispatch":"direct","isolated_ordered":true});
    value
}
#[test]
fn known_compiler_facts_survive_ir_and_chart_roundtrips() {
    let f = function(proven());
    let saved = serde_json::to_value(&f).unwrap();
    assert_eq!(saved["execution"], "synchronous");
    assert_eq!(
        saved["body"][0]["value"]["calls"][0]["invocation"],
        json!({"timing":"synchronous","dispatch":"direct","isolated_ordered":true})
    );
    let chart = serde_json::to_value(flow::build(&f).unwrap()).unwrap();
    assert!(chart["steps"].as_array().unwrap().iter().any(|step| {
        step["calls"].as_array().is_some_and(|calls| {
            calls
                .iter()
                .any(|site| site["invocation"]["isolated_ordered"] == true)
        })
    }));
}
#[test]
fn unsupported_fact_enum_states_are_rejected_instead_of_ignored() {
    let mut value = raw();
    value["execution"] = "async_guess".into();
    assert!(serde_json::from_value::<FunctionFlowIR>(value).is_err());
    for field in ["timing", "dispatch"] {
        let mut value = raw();
        call(&mut value)["invocation"] = json!({(field):"guessed"});
        assert!(serde_json::from_value::<FunctionFlowIR>(value).is_err());
    }
}
#[test]
fn legacy_serialization_baseline() {
    let f = function(raw());
    let ir = serde_json::to_string(&f).unwrap();
    let chart = serde_json::to_string(&flow::build(&f).unwrap()).unwrap();
    assert_eq!(ir, include_str!("fixtures/invocation-facts/ir.json"));
    assert_eq!(chart, include_str!("fixtures/invocation-facts/chart.json"));
    assert_eq!(
        serde_json::to_string(&function(serde_json::from_str(&ir).unwrap())).unwrap(),
        ir
    );
}
#[test]
fn isolated_proof_requires_an_owned_single_value_call_and_meaningful_equal_spans() {
    let mut variants = Vec::new();
    let mut value = proven();
    call(&mut value)["invocation"]["timing"] = "unknown".into();
    variants.push(value);
    let mut value = proven();
    let site = call(&mut value).clone();
    value["body"][0] = json!({"kind":"action","text":"f()","calls":[site]});
    variants.push(value);
    let mut value = proven();
    let site = call(&mut value).clone();
    value["body"][0]["value"]["calls"]
        .as_array_mut()
        .unwrap()
        .push(site);
    variants.push(value);
    let mut value = proven();
    value["body"][0]["value"]["regions"] =
        json!([{"kind":"region","label":"deferred","deferred":true,"body":[]}]);
    variants.push(value);
    let mut value = proven();
    call(&mut value)["span"] = json!({"start":0,"end":0});
    variants.push(value);
    let mut value = proven();
    call(&mut value)["span"] = json!({"start":1,"end":3});
    variants.push(value);
    for value in variants {
        assert!(
            flow::build(&function(value)).is_err(),
            "Invalid isolation proof accepted"
        );
    }
}
#[test]
fn direct_dispatch_rejects_multiple_native_targets_but_other_relations_do_not_count() {
    let mut value = proven();
    call(&mut value)["targets"]
        .as_array_mut()
        .unwrap()
        .push(json!({"symbol":"other","certainty":"possible","provenance":"compiler"}));
    assert!(flow::build(&function(value)).is_err());
    let mut value = proven();
    call(&mut value)["targets"].as_array_mut().unwrap().push(
        json!({"kind":"http","symbol":"route","certainty":"possible","provenance":"framework"}),
    );
    flow::build(&function(value)).unwrap();
}
#[test]
fn incomplete_and_unproven_legacy_calls_remain_valid() {
    let mut value = raw();
    call(&mut value)["invocation"] = json!({"timing":"synchronous"});
    flow::build(&function(value)).unwrap();
    let mut value = raw();
    call(&mut value)["span"] = json!({"start":0,"end":0});
    let site: CallSite = serde_json::from_value(call(&mut value).clone()).unwrap();
    assert_eq!(serde_json::to_value(site).unwrap().get("invocation"), None);
    flow::build(&function(value)).unwrap();
}
fn graph(value: Value) -> codecallnavigator_core::model::GraphData {
    use codecallnavigator_core::{
        architecture::{ArchitectureFacts, FileFacts},
        flow::FlowInventory,
        model::{GraphData, Node},
    };
    let nodes = [
        ("caller", "Caller"),
        ("target", "Target"),
        ("other", "Other"),
        ("route", "Route"),
    ]
    .into_iter()
    .map(|(id, label)| Node {
        id: id.into(),
        label: label.into(),
        signature: label.into(),
        kind: "method".into(),
        language: "csharp".into(),
        file: "/repo/F.cs".into(),
        build: "/repo/App.csproj".into(),
        project: "App".into(),
        line: 1,
        ..Default::default()
    })
    .collect();
    GraphData {
        nodes,
        flow_inventory: Some(FlowInventory {
            version: 1,
            functions: vec![function(value)],
        }),
        architecture: Some(ArchitectureFacts {
            version: 1,
            files: vec![FileFacts {
                file: "/repo/F.cs".into(),
                build: "/repo/App.csproj".into(),
                project: "App".into(),
                language: "csharp".into(),
                content: "f()".into(),
                ..Default::default()
            }],
        }),
        ..Default::default()
    }
}
#[test]
fn legacy_artifact_serialization_baseline() {
    let artifact = codecallnavigator_core::artifact::canonical(
        &graph(raw()),
        &[("repo".into(), "/repo".into())],
    )
    .unwrap();
    assert_eq!(
        artifact.bytes().unwrap(),
        include_bytes!("fixtures/invocation-facts/artifact.json")
    );
}
#[test]
fn explicit_unknown_defaults_preserve_exact_legacy_bytes_and_do_not_infer_from_hints() {
    let mut value = raw();
    value["execution"] = "unknown".into();
    call(&mut value)["invocation"] =
        json!({"timing":"unknown","dispatch":"unknown","isolated_ordered":false});
    assert_eq!(
        serde_json::to_string(&function(value)).unwrap(),
        include_str!("fixtures/invocation-facts/ir.json")
    );
}
#[test]
fn partial_facts_and_callee_execution_are_independent() {
    for execution in ["unknown", "synchronous", "deferred"] {
        let mut value = proven();
        value["execution"] = execution.into();
        flow::build(&function(value)).unwrap();
    }
    for invocation in [
        json!({"timing":"synchronous"}),
        json!({"dispatch":"virtual"}),
        json!({"dispatch":"candidate"}),
        json!({"timing":"deferred"}),
    ] {
        let mut value = raw();
        call(&mut value)["invocation"] = invocation.clone();
        let saved = serde_json::to_value(function(value)).unwrap();
        assert_eq!(
            saved["body"][0]["value"]["calls"][0]["invocation"],
            invocation
        );
    }
}
#[test]
fn direct_allows_zero_duplicate_targets_and_ignores_http_navigation() {
    for targets in [
        json!([]),
        json!([{"symbol":"target"},{"symbol":"target","certainty":"possible"}]),
        json!([{"symbol":"target"},{"kind":"http","symbol":"route"},{"kind":"navigation","symbol":"other"}]),
    ] {
        let mut value = proven();
        call(&mut value)["targets"] = targets;
        flow::build(&function(value)).unwrap();
    }
}
#[test]
fn unproven_action_regions_multicalls_and_zero_spans_stay_legacy_valid() {
    let mut value = raw();
    let mut site = call(&mut value).clone();
    site["span"] = json!({"start":0,"end":0});
    value["body"][0] = json!({"kind":"action","text":"calls","calls":[site.clone(),site]});
    flow::build(&function(value)).unwrap();
    let mut value = raw();
    value["body"][0]["value"]["regions"] =
        json!([{"kind":"region","label":"deferred","deferred":true,"body":[]}]);
    flow::build(&function(value)).unwrap();
}
#[test]
fn invalid_claims_are_rejected_at_source_checked_and_artifact_boundaries() {
    let mut value = proven();
    call(&mut value)["span"] = json!({"start":0,"end":0});
    let f = function(value.clone());
    assert!(flow::validate_source(&f, "f()").is_err());
    assert!(flow::checked_chart(&f, "f()").is_err());
    assert!(
        codecallnavigator_core::artifact::canonical(
            &graph(value),
            &[("repo".into(), "/repo".into())]
        )
        .is_err()
    );
}
#[test]
fn candidate_and_virtual_dispatch_survive_target_filtering_without_promotion() {
    use std::collections::HashSet;
    for dispatch in ["virtual", "candidate"] {
        let mut value = proven();
        call(&mut value)["invocation"]["dispatch"] = dispatch.into();
        call(&mut value)["targets"]
            .as_array_mut()
            .unwrap()
            .push(json!({"symbol":"other"}));
        let mut inventory = flow::FlowInventory {
            version: 1,
            functions: vec![function(value)],
        };
        inventory.retain_symbols(&HashSet::from(["caller", "target"]));
        let saved = serde_json::to_value(&inventory.functions[0]).unwrap();
        let site = &saved["body"][0]["value"]["calls"][0];
        assert_eq!(site["targets"].as_array().unwrap().len(), 1);
        assert_eq!(site["invocation"]["dispatch"], dispatch);
        assert_eq!(saved["execution"], "synchronous");
        flow::build(&inventory.functions[0]).unwrap();
    }
}
#[test]
fn compiler_facts_survive_canonical_artifact_and_borrowed_comparison_view() {
    use codecallnavigator_core::artifact::{
        self, Artifact,
        flow::{Selection, View},
    };
    let a = artifact::canonical(&graph(proven()), &[("repo".into(), "/repo".into())]).unwrap();
    let bytes = a.bytes().unwrap();
    let a = Artifact::from_bytes(&bytes).unwrap();
    assert_eq!(a.bytes().unwrap(), bytes);
    let symbol = a.graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["label"] == "Caller")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let view = View::new(&a).unwrap();
    let Selection::Ready(side) = view.select(symbol) else {
        panic!("Saved facts unavailable")
    };
    assert_eq!(
        serde_json::to_value(side.function).unwrap()["execution"],
        "synchronous"
    );
    assert!(
        serde_json::to_value(side.chart).unwrap()["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["calls"].as_array().is_some_and(|sites| sites
                .iter()
                .any(|site| site["invocation"]["dispatch"] == "direct")))
    );
}

#[test]
fn zero_value_proofs_reject_but_incomplete_action_hints_are_preserved() {
    let mut value = proven();
    value["body"][0]["value"]["span"] = json!({"start":0,"end":0});
    call(&mut value)["span"] = json!({"start":0,"end":0});
    assert!(flow::build(&function(value)).is_err());
    let mut value = raw();
    let mut site = call(&mut value).clone();
    site["invocation"] = json!({"timing":"synchronous","dispatch":"direct"});
    value["body"][0] = json!({"kind":"action","text":"f()","calls":[site]});
    flow::build(&function(value)).unwrap();
}
#[test]
fn malformed_fact_enums_and_claims_cannot_reenter_via_saved_artifacts() {
    use codecallnavigator_core::artifact::{
        self, Artifact,
        flow::{Reason, Selection, View},
    };
    let mut a = artifact::canonical(&graph(proven()), &[("repo".into(), "/repo".into())]).unwrap();
    a.graph["flow_inventory"]["functions"][0]["execution"] = "guessed".into();
    assert!(Artifact::from_bytes(&serde_json::to_vec(&a).unwrap()).is_err());
    let mut a = artifact::canonical(&graph(proven()), &[("repo".into(), "/repo".into())]).unwrap();
    a.graph["flow_inventory"]["functions"][0]["body"][0]["value"]["calls"][0]["invocation"]["timing"] =
        "unknown".into();
    assert!(a.validate().is_err());
    let id = a.graph["flow_inventory"]["functions"][0]["symbol"]
        .as_str()
        .unwrap();
    assert!(
        matches!(View::new(&a).unwrap().select(id),Selection::Unavailable(reason) if reason.reason==Reason::InvalidFlow)
    );
}
