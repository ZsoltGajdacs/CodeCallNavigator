use codecallnavigator::flow::{self, FlowChart, FunctionFlowIR};
use serde_json::{Value, json};
use std::collections::HashSet;

fn function(body: Vec<Value>) -> FunctionFlowIR {
    serde_json::from_value(json!({"symbol":"fixture.method","file":"fixture.ts","span":{"start":0,"end":200},"body":body})).unwrap()
}
fn action(text: &str) -> Value {
    json!({"kind":"action","text":text})
}
fn value(text: &str) -> Value {
    json!({"kind":"value","text":text})
}
fn ret(text: &str) -> Value {
    json!({"kind":"return","value":value(text)})
}
fn throw(text: &str) -> Value {
    json!({"kind":"throw","value":value(text)})
}
fn call(text: &str) -> Value {
    json!({"kind":"action","text":text,"calls":[{"label":text}]})
}
fn paths(chart: &FlowChart) -> Vec<Vec<String>> {
    fn visit(
        chart: &FlowChart,
        node: usize,
        mut trace: Vec<String>,
        mut seen: HashSet<usize>,
        paths: &mut Vec<Vec<String>>,
    ) {
        if !seen.insert(node) {
            return;
        }
        trace.push(chart.steps[node].label.clone());
        let next: Vec<_> = chart
            .links
            .iter()
            .filter(|link| link.from == node)
            .collect();
        if next.is_empty() {
            paths.push(trace);
            return;
        }
        for link in next {
            visit(chart, link.to, trace.clone(), seen.clone(), paths);
        }
    }
    let mut paths = Vec::new();
    visit(chart, chart.entry, Vec::new(), HashSet::new(), &mut paths);
    paths
}
fn contains(path: &[String], label: &str) -> bool {
    path.iter().any(|step| step == label)
}

#[test]
fn finally_clones_preserve_return_and_normal_completions() {
    let ir = function(vec![
        json!({"kind":"try","body":[{"kind":"if","condition":value("flag"),"then_branch":[ret("result")]}],"finally":[action("cleanup")]}),
        action("after"),
    ]);
    let chart = flow::build(&ir).unwrap();
    let paths = paths(&chart);
    assert!(paths.iter().any(|path| contains(path, "after")));
    for path in paths {
        assert!(contains(&path, "cleanup"));
        if contains(&path, "return") {
            assert!(!contains(&path, "after"));
            assert_eq!(path.last().unwrap(), "Return to caller");
        }
    }
}

#[test]
fn finally_return_overrides_pending_throw_and_switch_yield() {
    let ir = function(vec![
        json!({"kind":"try","body":[throw("error")],"finally":[ret("replacement")]}),
        action("unreachable"),
    ]);
    let chart = flow::build(&ir).unwrap();
    for path in paths(&chart) {
        assert_eq!(path.last().unwrap(), "Return to caller");
        assert!(!contains(&path, "unreachable"));
    }
    let switch = json!({"kind":"switch","span":{"start":0,"end":20},"subject":value("choice"),"cases":[{"is_default":true,"body":[{"kind":"try","body":[{"kind":"yield","value":value("picked")}],"finally":[action("yield cleanup")]}]}]});
    let chart = flow::build(&function(vec![json!({"kind":"return","value":switch})])).unwrap();
    assert!(!chart.partial);
    for path in paths(&chart) {
        assert!(contains(&path, "yield cleanup"));
        assert_eq!(path.last().unwrap(), "Return to caller");
    }
}

#[test]
fn resource_acquisition_failure_closes_only_previously_acquired_resources() {
    let ir = function(vec![
        json!({"kind":"try","resources":[call("acquire first"),throw("second acquisition failed")],"body":[action("body")],"cleanup":[action("close second"),action("close first")],"finally":[action("finally")]} ),
    ]);
    let chart = flow::build(&ir).unwrap();
    let paths = paths(&chart);
    assert!(
        paths
            .iter()
            .any(|path| contains(path, "second acquisition failed"))
    );
    for path in paths
        .iter()
        .filter(|path| contains(path, "second acquisition failed"))
    {
        assert!(contains(path, "close first"));
        assert!(contains(path, "finally"));
        assert!(!contains(path, "close second"));
        assert!(!contains(path, "body"));
    }
}

#[test]
fn protected_call_exception_passes_through_resource_close_and_finally() {
    let ir = function(vec![
        json!({"kind":"try","resources":[action("acquire")],"body":[call("body call")],"cleanup":[action("close")],"finally":[action("finally")]} ),
    ]);
    let chart = flow::build(&ir).unwrap();
    let exceptions: Vec<_> = paths(&chart)
        .into_iter()
        .filter(|path| path.last().is_some_and(|step| step == "Exception escapes"))
        .collect();
    assert!(!exceptions.is_empty());
    for path in exceptions {
        assert!(contains(&path, "close"));
        assert!(contains(&path, "finally"));
    }
}

#[test]
fn short_circuit_sequences_and_deferred_callbacks_do_not_invent_eager_calls() {
    let expression = json!({"kind":"sequence","items":[{"kind":"and","left":value("flag"),"right":value("inner call")},value("outer call")]});
    let callback = json!({"kind":"region","label":"deferred callback","deferred":true,"body":[action("callback body")]});
    let ir = function(vec![
        json!({"kind":"evaluate","value":expression}),
        json!({"kind":"return","value":{"kind":"value","text":"callback registration","regions":[callback]}}),
    ]);
    let chart = flow::build(&ir).unwrap();
    let paths = paths(&chart);
    assert!(paths.iter().any(|path| !contains(path, "inner call")));
    assert!(
        paths
            .iter()
            .all(|path| contains(path, "outer call") && !contains(path, "callback body"))
    );
    assert_eq!(chart.regions.len(), 1);
}

#[test]
fn catch_all_handles_explicit_throw_without_an_impossible_escape() {
    let ir = function(vec![
        json!({"kind":"try","body":[throw("failure")],"catches":[{"parameter":"error","catch_all":true,"body":[action("caught")]}]}),
        action("after"),
    ]);
    let chart = flow::build(&ir).unwrap();
    assert!(!chart.partial);
    for path in paths(&chart) {
        assert!(contains(&path, "caught"));
        assert!(contains(&path, "after"));
        assert_eq!(path.last().unwrap(), "Function exit");
    }
}

#[test]
fn graphviz_preserves_flow_link_identity_and_opaque_boundaries() {
    let ir = function(vec![
        json!({"kind":"unsupported","text":"opaque construct","reason":"Unsupported syntax"}),
        ret("done"),
    ]);
    let chart = flow::build(&ir).unwrap();
    assert!(chart.partial);
    assert!(chart.links.iter().any(|link| link.kind == "unknown"));
    let chart = flow::layout(chart, std::path::Path::new("/opt/homebrew/bin/dot")).unwrap();
    assert!(chart.size.iter().all(|value| *value > 0.0));
    assert!(chart.links.iter().all(|link| link.points.len() >= 4));
}

#[test]
fn nested_try_call_exceptions_cannot_skip_inner_finally() {
    let ir = function(vec![
        json!({"kind":"try","body":[{"kind":"try","body":[call("nested call")],"finally":[action("inner finally")]}],"finally":[action("outer finally")]} ),
    ]);
    let chart = flow::build(&ir).unwrap();
    for path in paths(&chart) {
        assert!(contains(&path, "inner finally"));
        assert!(contains(&path, "outer finally"));
    }
}

#[test]
fn literal_boolean_ternary_conditions_do_not_invent_opposite_branches() {
    let condition = json!({"kind":"conditional","condition":value("true"),"when_true":value("false"),"when_false":value("true")});
    let ir = function(vec![
        json!({"kind":"if","condition":condition,"then_branch":[action("impossible")],"else_branch":[action("actual")]} ),
    ]);
    let chart = flow::build(&ir).unwrap();
    assert!(
        paths(&chart)
            .iter()
            .all(|path| contains(path, "actual") && !contains(path, "impossible"))
    );
}

#[test]
fn labeled_loop_break_and_continue_cross_finally_before_their_real_destinations() {
    let ir = function(vec![
        json!({"kind":"loop","label":"outer","condition":value("true"),"body":[{"kind":"try","body":[{"kind":"if","condition":value("stop"),"then_branch":[{"kind":"break","label":"outer"}],"else_branch":[{"kind":"continue","label":"outer"}]}],"finally":[action("loop cleanup")]}],"update":[action("update")]}),
        action("after loop"),
    ]);
    let chart = flow::build(&ir).unwrap();
    let breaks: Vec<_> = paths(&chart)
        .into_iter()
        .filter(|path| contains(path, "break outer"))
        .collect();
    assert!(!breaks.is_empty());
    for path in breaks {
        assert!(contains(&path, "loop cleanup"));
        assert!(contains(&path, "after loop"));
        assert!(!contains(&path, "update"));
    }
    let continued = chart
        .steps
        .iter()
        .find(|step| step.label == "continue outer")
        .unwrap()
        .id;
    let cleanup = chart
        .links
        .iter()
        .find(|link| link.from == continued)
        .unwrap()
        .to;
    assert_eq!(chart.steps[cleanup].label, "loop cleanup");
    let next = chart
        .links
        .iter()
        .find(|link| link.from == cleanup)
        .unwrap()
        .to;
    assert_eq!(chart.steps[next].kind, "loop_update");
}

#[test]
fn effectful_switch_tests_stop_at_match_and_default_fallthrough_skips_retesting() {
    let case_test = |text: &str| json!({"kind":"value","text":text,"calls":[{"label":text}]});
    let ir = function(vec![
        json!({"kind":"switch","subject":value("input"),"cases":[
            {"is_default":true,"falls_through":true,"body":[action("default body")]},
            {"labels":["A"],"test":case_test("test A"),"body":[action("A body")]},
            {"labels":["B"],"test":case_test("test B"),"body":[action("B body")]}
        ]}),
        action("after switch"),
    ]);
    let chart = flow::build(&ir).unwrap();
    let paths = paths(&chart);
    let matched_a = paths
        .iter()
        .find(|path| contains(path, "A body") && !contains(path, "default body"))
        .unwrap();
    assert!(contains(matched_a, "test A"));
    assert!(!contains(matched_a, "test B"));
    let fallback = paths
        .iter()
        .find(|path| contains(path, "default body"))
        .unwrap();
    assert!(contains(fallback, "A body"));
    assert!(!contains(fallback, "B body"));
    let index = |text: &str| fallback.iter().position(|step| step == text).unwrap();
    assert!(index("test A") < index("test B"));
    assert!(index("test B") < index("default body"));
    assert_eq!(fallback.iter().filter(|step| *step == "test A").count(), 1);
    assert_eq!(fallback.iter().filter(|step| *step == "test B").count(), 1);
}
