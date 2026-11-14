//! Existing-IR encoding for explicitly opaque C# completion outcomes.
use codecallnavigator_core::flow::{self, FlowChart, FunctionFlowIR};
use serde_json::{Value, json};

fn paths(chart: &FlowChart) -> Vec<Vec<String>> {
    fn walk(chart: &FlowChart, at: usize, mut path: Vec<String>, out: &mut Vec<Vec<String>>) {
        path.push(chart.steps[at].label.clone());
        let links: Vec<_> = chart.links.iter().filter(|link| link.from == at).collect();
        if links.is_empty() {
            out.push(path);
            return;
        }
        for link in links {
            walk(chart, link.to, path.clone(), out);
        }
    }
    let mut result = Vec::new();
    walk(chart, chart.entry, Vec::new(), &mut result);
    result
}
fn function(body: Value) -> FunctionFlowIR {
    serde_json::from_value(json!({"span":{"start":10,"end":100},"body":body})).unwrap()
}
#[test]
fn synthetic_exception_outcome_unwinds_outer_cleanup_without_fake_calls() {
    let opaque = json!({"kind":"unsupported","text":"filtered region","reason":"Filter dispatch precedes unwind and remains opaque"});
    let value = |text: &str| json!({"kind":"value","text":text});
    let ir = function(
        json!([{"kind":"try","body":[opaque,{"kind":"if","condition":value("synthetic C#: opaque completion outcome is exceptional"),"then_branch":[{"kind":"throw","value":value("opaque exception propagates")}]}],"finally":[{"kind":"action","text":"outer cleanup"}]},{"kind":"action","text":"after"}]),
    );
    flow::validate_source(&ir, &"x".repeat(100)).unwrap();
    let chart = flow::build(&ir).unwrap();
    assert!(chart.partial);
    assert!(chart.steps.iter().all(|step| step.calls.is_empty()));
    let paths = paths(&chart);
    assert!(
        paths
            .iter()
            .any(|path| path.last().unwrap() == "Exception escapes")
    );
    assert!(
        paths
            .iter()
            .any(|path| path.last().unwrap() == "Function exit")
    );
    for path in paths {
        assert!(path.iter().any(|step| step == "outer cleanup"));
        if path.last().unwrap() == "Exception escapes" {
            assert!(!path.iter().any(|step| step == "after"));
        }
    }
}
#[test]
fn legacy_unsupported_nodes_do_not_acquire_new_completion_semantics() {
    let ir = function(
        json!([{"kind":"unsupported","text":"legacy opaque","reason":"Unknown"},{"kind":"action","text":"after"}]),
    );
    let chart = flow::build(&ir).unwrap();
    for path in paths(&chart) {
        assert_eq!(path.last().unwrap(), "Function exit");
        assert!(path.iter().any(|step| step == "after"));
    }
}

#[test]
fn synthetic_exception_outcome_can_enter_an_outer_catch_after_cleanup() {
    let value = |text: &str| json!({"kind":"value","text":text});
    let ir = function(
        json!([{"kind":"try","body":[{"kind":"try","body":[{"kind":"unsupported","text":"filtered region","reason":"Opaque filter dispatch"},{"kind":"if","condition":value("synthetic C#: opaque completion outcome is exceptional"),"then_branch":[{"kind":"throw","value":value("opaque exception propagates")}]}],"finally":[{"kind":"action","text":"inner cleanup"}]}],"catches":[{"parameter":"Exception candidate","catch_all":false,"body":[{"kind":"action","text":"outer handler"}]}],"finally":[{"kind":"action","text":"outer cleanup"}]}]),
    );
    let chart = flow::build(&ir).unwrap();
    let paths = paths(&chart);
    assert!(
        paths
            .iter()
            .any(|path| path.iter().any(|step| step == "outer handler"))
    );
    assert!(
        paths
            .iter()
            .any(|path| path.last().unwrap() == "Exception escapes")
    );
    for path in paths {
        let inner = path
            .iter()
            .position(|step| step == "inner cleanup")
            .unwrap();
        let outer = path
            .iter()
            .position(|step| step == "outer cleanup")
            .unwrap();
        assert!(inner < outer);
        if let Some(handler) = path.iter().position(|step| step == "outer handler") {
            assert!(inner < handler && handler < outer);
        }
    }
}
