//! Explicit immutable IR/source inputs prove expansion semantics without compiler processes.
use codecallnavigator_core::flow::{
    self, CallChoice, ExpandedFlow, ExpansionRequest, FunctionFlowIR, FunctionSource,
};
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;
const SOURCE: &str = "call() then() child() return throw cleanup leaf()";
fn function(symbol: &str, body: Value) -> FunctionFlowIR {
    serde_json::from_value(json!({"symbol":symbol,"file":format!("{symbol}.cs"),"build":"App.csproj","project":"App","language":"csharp","execution":"synchronous","span":{"start":0,"end":SOURCE.len()},"body":body})).unwrap()
}
fn call(target: &str) -> Value {
    json!({"kind":"value","text":"call()","span":{"start":0,"end":6},"calls":[{"label":"call()","span":{"start":0,"end":6},"invocation":{"timing":"synchronous","dispatch":"direct","isolated_ordered":true},"targets":[{"symbol":target,"certainty":"direct","provenance":"explicit unit input"}]}]})
}
fn action(text: &str) -> Value {
    json!({"kind":"action","text":text})
}
fn selection(root: &FunctionFlowIR) -> ExpansionRequest {
    let site = flow::call_sites(root).remove(0);
    ExpansionRequest::new(&root.symbol).select_root(CallChoice {
        site: site.address,
        target: site.site.targets[0].clone(),
    })
}
fn expanded(root: &FunctionFlowIR, child: &FunctionFlowIR) -> ExpandedFlow {
    run(root, child, &selection(root))
}
#[test]
fn a_selected_synchronous_child_returns_to_the_caller_continuation_with_exact_sources() {
    let root = function(
        "root",
        json!([{"kind":"evaluate","value":call("child")},action("caller after")]),
    );
    let child = function("child", json!([action("child body"),{"kind":"return"}]));
    let result = expanded(&root, &child);
    result.validate().unwrap();
    assert_eq!(result.occurrences.len(), 2);
    assert_eq!(result.sources.len(), 2);
    assert!(
        result
            .chart
            .steps
            .iter()
            .any(|step| step.label == "child body")
    );
    assert!(
        result
            .chart
            .steps
            .iter()
            .any(|step| step.label == "caller after")
    );
    assert_eq!(
        result
            .sources
            .get(result.occurrences[1].source.unwrap())
            .unwrap()
            .owner
            .file,
        "child.cs"
    );
    assert_eq!(
        result.occurrences[1].outcome,
        flow::OccurrenceOutcome::Expanded
    );
    assert_normal_child_path(&result);
    assert_eq!(
        result.occurrences[1].path.0[0],
        CallChoice {
            site: flow::call_sites(&root)[0].address.clone(),
            target: flow::call_sites(&root)[0].site.targets[0].clone()
        }
    );
}

fn step_id(result: &ExpandedFlow, label: &str) -> usize {
    result
        .chart
        .steps
        .iter()
        .find(|step| step.label == label)
        .unwrap()
        .id
}
fn assert_normal_child_path(result: &ExpandedFlow) {
    let child_entry = result.occurrences[1].boundaries.entry.unwrap();
    let child_body = step_id(result, "child body");
    let after = step_id(result, "caller after");
    assert!(reaches(result, result.chart.entry, child_body, None));
    assert!(!reaches(
        result,
        result.chart.entry,
        after,
        Some(child_entry)
    ));
    assert!(
        result.occurrences[1]
            .boundaries
            .returns
            .iter()
            .any(|step| reaches(result, *step, after, None))
    );
    assert_eq!(
        result.step_origins[child_body].occurrence,
        flow::OccurrenceId(1)
    );
    assert_eq!(result.step_origins[after].occurrence, flow::OccurrenceId(0));
}

fn reaches(result: &ExpandedFlow, from: usize, to: usize, excluded: Option<usize>) -> bool {
    let mut pending = vec![from];
    let mut seen = std::collections::HashSet::new();
    while let Some(at) = pending.pop() {
        if Some(at) == excluded || !seen.insert(at) {
            continue;
        }
        if at == to {
            return true;
        }
        pending.extend(
            result
                .chart
                .links
                .iter()
                .filter(|link| link.from == at)
                .map(|link| link.to),
        );
    }
    false
}

#[path = "flow_expansion/completion.rs"]
mod completion;
#[path = "flow_expansion/root_only.rs"]
mod root_only;
#[path = "flow_expansion/selection.rs"]
mod selected;
fn run(root: &FunctionFlowIR, child: &FunctionFlowIR, request: &ExpansionRequest) -> ExpandedFlow {
    flow::expand(
        &[
            FunctionSource {
                function: root,
                content: SOURCE,
            },
            FunctionSource {
                function: child,
                content: SOURCE,
            },
        ],
        request,
        &AtomicBool::new(false),
    )
    .unwrap()
}
fn labels_from(result: &ExpandedFlow, start: usize) -> Vec<Vec<String>> {
    let mut pending = vec![(start, vec![], std::collections::HashSet::new())];
    let mut paths = Vec::new();
    while let Some((at, mut path, mut seen)) = pending.pop() {
        if !seen.insert(at) {
            continue;
        }
        path.push(result.chart.steps[at].label.clone());
        let next: Vec<_> = result
            .chart
            .links
            .iter()
            .filter(|link| link.from == at)
            .map(|link| link.to)
            .collect();
        if next.is_empty() {
            paths.push(path);
        } else {
            for to in next {
                pending.push((to, path.clone(), seen.clone()));
            }
        }
    }
    paths
}
#[path = "flow_expansion/budgets.rs"]
mod budgets;
#[path = "flow_expansion/nested.rs"]
mod nested;
