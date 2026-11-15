//! Target mapping must not promote compiler dispatch or discard external-site facts.
use super::*;
use crate::flow::{self, CallSite, ExecutionTiming, InvocationDispatch};
use serde_json::json;

fn raw(dispatch: &str, targets: &[&str]) -> FunctionFlowIR {
    serde_json::from_value(json!({
        "symbol":"raw-caller","file":"/repo/F.java","language":"java",
        "execution":"synchronous","span":{"start":0,"end":3},
        "body":[{"kind":"evaluate","value":{"kind":"value","text":"f()",
            "span":{"start":0,"end":3},"calls":[{"span":{"start":0,"end":3},"label":"f()",
                "invocation":{"timing":"synchronous","dispatch":dispatch,"isolated_ordered":true},
                "targets":targets.iter().map(|symbol|json!({"symbol":symbol,"certainty":"dispatch","provenance":"javac"})).collect::<Vec<_>>()
            }]} }]
    })).unwrap()
}

fn sites(function: &FunctionFlowIR) -> Vec<CallSite> {
    let mut sites = Vec::new();
    visit_calls_mut(&mut function.clone(), &mut |site| sites.push(site.clone()));
    sites
}

fn owner() -> Node {
    Node {
        id: "bound-caller".into(),
        build: "/repo/pom.xml".into(),
        project: "app".into(),
        ..Default::default()
    }
}

#[test]
fn mapping_then_pruning_to_one_target_preserves_virtual_and_candidate_facts() {
    for dispatch in ["virtual", "candidate"] {
        let original = raw(dispatch, &["native", "external"]);
        let unchanged = original.clone();
        let bound = bind_function(
            &original,
            &owner(),
            &HashMap::from([("native".into(), "bound-target")]),
        );
        let original_site = &sites(&original)[0];
        let site = &sites(&bound)[0];
        assert_eq!(bound.execution, ExecutionTiming::Synchronous);
        assert_eq!(site.invocation, original_site.invocation);
        assert_ne!(site.invocation.dispatch, InvocationDispatch::Direct);
        assert_eq!(site.targets.len(), 1);
        assert_eq!(site.targets[0].symbol, "bound-target");
        assert_eq!(original, unchanged);
        flow::build(&bound).unwrap();
    }
}

#[test]
fn external_call_keeps_direct_facts_with_no_invented_project_target() {
    let original = raw("direct", &["external"]);
    let bound = bind_function(&original, &owner(), &HashMap::new());
    let site = &sites(&bound)[0];
    assert!(site.targets.is_empty());
    assert_eq!(site.invocation, sites(&original)[0].invocation);
    assert!(site.diagnostic.as_ref().unwrap().contains("external"));
    flow::build(&bound).unwrap();
}
