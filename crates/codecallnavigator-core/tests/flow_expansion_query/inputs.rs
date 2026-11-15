use codecallnavigator_core::{
    artifact::{Artifact, SCHEMA},
    flow, query,
};
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;
fn fixture() -> Artifact {
    let node = |id: &str, file: &str| json!({"id":id,"label":id,"signature":id,"language":"csharp","build":"@repo/App.csproj","project":"App","file":file});
    let function = |id: &str, file: &str, body: Value| json!({"symbol":id,"language":"csharp","build":"@repo/App.csproj","project":"App","file":file,"execution":"synchronous","span":{"start":0,"end":6},"body":body});
    let call = json!({"kind":"value","text":"leaf()","span":{"start":0,"end":6},"calls":[{"label":"leaf()","span":{"start":0,"end":6},"invocation":{"timing":"synchronous","dispatch":"direct","isolated_ordered":true},"targets":[{"symbol":"leaf","certainty":"possible","provenance":"saved compiler fact"}]}]});
    let root = function(
        "root",
        "@repo/Root.cs",
        json!([{"kind":"evaluate","value":call}]),
    );
    let leaf = function(
        "leaf",
        "@repo/Leaf.cs",
        json!([{"kind":"action","text":"child body"}]),
    );
    let source = |file: &str| json!({"file":file,"build":"@repo/App.csproj","project":"App","language":"csharp","content":"leaf()"});
    Artifact {
        schema: SCHEMA.into(),
        graph: json!({"nodes":[node("root","@repo/Root.cs"),node("leaf","@repo/Leaf.cs")],"flow_inventory":{"version":1,"functions":[root,leaf]},"architecture":{"version":1,"files":[source("@repo/Root.cs"),source("@repo/Leaf.cs")]}}),
        extensions: Default::default(),
    }
}
fn choice(saved: &Artifact) -> query::FlowExpansionSelection {
    let function: flow::FunctionFlowIR =
        serde_json::from_value(saved.graph["flow_inventory"]["functions"][0].clone()).unwrap();
    let sites = flow::call_sites(&function);
    query::FlowExpansionSelection {
        caller: Default::default(),
        choice: flow::CallChoice {
            site: sites[0].address.clone(),
            target: sites[0].site.targets[0].clone(),
        },
    }
}
fn request(selections: &[query::FlowExpansionSelection]) -> query::FlowExpansionRequest {
    query::FlowExpansionRequest {
        expansion_version: 1,
        symbol: "root".into(),
        selections: selections.to_vec(),
        limits: Default::default(),
    }
}
#[test]
fn cached_expansion_keeps_exact_owned_sources_and_full_target_without_live_files() {
    let mut saved = fixture();
    // Invalid ranges in an unrelated function must not be consumed by expansion.
    let mut unrelated = saved.graph["flow_inventory"]["functions"][1].clone();
    unrelated["symbol"] = json!("unrelated");
    unrelated["span"]["end"] = json!(999);
    saved.graph["flow_inventory"]["functions"]
        .as_array_mut()
        .unwrap()
        .push(unrelated);
    let selected = choice(&saved);
    let result = query::expand_flow(
        &saved,
        &request(std::slice::from_ref(&selected)),
        &AtomicBool::new(false),
    )
    .unwrap();
    result.validate().unwrap();
    assert_eq!(result.occurrences.len(), 2);
    assert_eq!(
        result.occurrences[1].outcome,
        flow::OccurrenceOutcome::Expanded
    );
    assert_eq!(result.occurrences[1].path.0[0], selected.choice);
    assert_eq!(result.sources.len(), 2);
    assert_eq!(
        result
            .sources
            .get(result.occurrences[1].source.unwrap())
            .unwrap()
            .owner
            .file,
        "@repo/Leaf.cs"
    );
}
#[test]
fn missing_callee_stops_but_wrong_owned_source_and_stale_choices_fail_explicitly() {
    let mut saved = fixture();
    let selected = choice(&saved);
    saved.graph["flow_inventory"]["functions"]
        .as_array_mut()
        .unwrap()
        .pop();
    let result = query::expand_flow(
        &saved,
        &request(std::slice::from_ref(&selected)),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(matches!(
        result.occurrences[1].outcome,
        flow::OccurrenceOutcome::Stopped {
            reason: flow::ExpansionStop::Unavailable,
            ..
        }
    ));
    let mut wrong = fixture();
    wrong.graph["architecture"]["files"][0]["project"] = json!("other");
    assert_eq!(
        query::expand_flow(&wrong, &request(&[]), &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "source_unavailable"
    );
    let mut deep = request(std::slice::from_ref(&selected));
    deep.selections[0].caller.0 = vec![selected.choice.clone(); 32];
    assert_eq!(
        query::expand_flow(&wrong, &deep, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "invalid_request"
    );
    let mut oversized = request(std::slice::from_ref(&selected));
    oversized.selections[0].choice.target.provenance = "x".repeat(1024 * 1024 + 1);
    assert_eq!(
        query::expand_flow(&wrong, &oversized, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "invalid_request"
    );
    let mut invalid_limits = request(&[]);
    invalid_limits.limits.steps = 0;
    assert_eq!(
        query::expand_flow(&wrong, &invalid_limits, &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "invalid_request"
    );
    let mut stale = selected;
    stale.choice.target.provenance.push_str(" altered");
    assert_eq!(
        query::expand_flow(&fixture(), &request(&[stale]), &AtomicBool::new(false))
            .unwrap_err()
            .code,
        "invalid_request"
    );
    assert_eq!(
        query::expand_flow(&fixture(), &request(&[]), &AtomicBool::new(true))
            .unwrap_err()
            .code,
        "cancelled"
    );
}
