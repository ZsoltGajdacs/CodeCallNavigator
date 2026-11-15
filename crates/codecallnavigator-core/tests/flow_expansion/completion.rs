//! Observable paths distinguish child completion from the caller's own continuation.
use super::*;
#[test]
fn selected_call_in_return_and_condition_resumes_the_original_caller_operation() {
    let child = function("child", json!([action("child body"),{"kind":"return"}]));
    let root = function(
        "root",
        json!([{"kind":"return","value":call("child")},action("unreachable after return")]),
    );
    let result = expanded(&root, &child);
    result.validate().unwrap();
    assert!(
        !result
            .chart
            .steps
            .iter()
            .any(|step| step.label == "unreachable after return")
    );
    let returned = result.occurrences[1].boundaries.returns[0];
    let caller_return = result
        .chart
        .steps
        .iter()
        .find(|step| {
            step.kind == "return"
                && result.step_origins[step.id].occurrence == flow::OccurrenceId(0)
        })
        .unwrap()
        .id;
    assert!(reaches(&result, returned, caller_return, None));
    let root = function(
        "root",
        json!([{"kind":"if","condition":call("child"),"then_branch":[action("yes branch")],"else_branch":[action("no branch")]}]),
    );
    let result = expanded(&root, &child);
    result.validate().unwrap();
    let resumed = result
        .chart
        .steps
        .iter()
        .find(|step| step.kind == "decision" && step.label == "call()" && step.calls.is_empty())
        .unwrap()
        .id;
    assert!(result.step_origins[resumed].calls.is_empty());
    assert!(reaches(
        &result,
        result.occurrences[1].boundaries.returns[0],
        resumed,
        None
    ));
    assert_eq!(
        result
            .chart
            .steps
            .iter()
            .map(|step| step.calls.len())
            .sum::<usize>(),
        1
    );
    for label in ["yes branch", "no branch"] {
        let branch = step_id(&result, label);
        assert!(reaches(&result, resumed, branch, None));
        assert!(!reaches(
            &result,
            result.chart.entry,
            branch,
            result.occurrences[1].boundaries.entry
        ));
    }
}

#[test]
fn child_throw_and_leaf_failure_unwind_child_then_caller_cleanup_and_catch() {
    let root = function(
        "root",
        json!([{"kind":"try","body":[{"kind":"evaluate","value":call("child")}],"catches":[{"parameter":"failure","catch_all":true,"body":[action("caller catch")]}],"finally":[action("caller cleanup")]},action("caller after")]),
    );
    for body in [
        json!([{"kind":"throw","value":{"kind":"value","text":"failure"}}]),
        json!([{"kind":"action","text":"leaf call","calls":[{"label":"leaf()"}]}]),
    ] {
        let child = function(
            "child",
            json!([{"kind":"try","body":body,"finally":[action("child cleanup")]}]),
        );
        let result = expanded(&root, &child);
        result.validate().unwrap();
        let thrown = result.occurrences[1].boundaries.throws[0];
        let handler = result
            .chart
            .steps
            .iter()
            .find(|step| step.label == "caller catch")
            .unwrap()
            .id;
        assert!(reaches(&result, thrown, handler, None));
        let paths = labels_from(&result, result.occurrences[1].boundaries.entry.unwrap());
        for path in &paths {
            let child_cleanup = path
                .iter()
                .position(|label| label == "child cleanup")
                .unwrap();
            let caller_cleanup = path
                .iter()
                .position(|label| label == "caller cleanup")
                .unwrap();
            assert!(child_cleanup < caller_cleanup);
            assert!(path.iter().any(|label| label == "caller after"));
        }
        assert!(
            paths
                .iter()
                .any(|path| path.iter().any(|label| label == "caller catch"))
        );
        // A pre-body call failure is separate: it reaches caller cleanup/catch
        // while legitimately bypassing the child's entry and cleanup.
        assert!(reaches(
            &result,
            result.occurrences[1].boundaries.call.unwrap(),
            handler,
            result.occurrences[1].boundaries.entry
        ));
    }
}

#[test]
fn selected_cleanup_copy_uses_original_resource_finally_or_unpaired_cleanup_identity() {
    use flow::{CleanupCopy, CleanupSite, IrField, IrPath, PendingRoute};
    let child = function("child", json!([action("cleanup child")]));
    let scope = IrPath::default().field(IrField::Body).index(0);
    let variants = [
        (
            json!([{"kind":"try","body":[{"kind":"if","condition":{"kind":"value","text":"flag"},"then_branch":[{"kind":"return"}]}],"finally":[{"kind":"evaluate","value":call("child")}]}]),
            CleanupSite::Finally {
                block: scope.clone().field(IrField::Finally),
            },
            PendingRoute::Return,
        ),
        (
            json!([{"kind":"try","resources":[action("acquire")],"body":[action("body")],"cleanup":[{"kind":"evaluate","value":call("child")}]}]),
            CleanupSite::Resource {
                resource: scope.clone().field(IrField::Resources).index(0),
            },
            PendingRoute::Normal,
        ),
        (
            json!([{"kind":"try","body":[action("body")],"cleanup":[{"kind":"evaluate","value":call("child")}]}]),
            CleanupSite::Cleanup {
                block: scope.field(IrField::Cleanup),
            },
            PendingRoute::Normal,
        ),
    ];
    for (body, site, route) in variants {
        let root = function("root", body);
        let original = flow::call_sites(&root).remove(0);
        let mut address = original.address;
        address.cleanup = vec![CleanupCopy { site, route }];
        let choice = CallChoice {
            site: address,
            target: original.site.targets[0].clone(),
        };
        let request = ExpansionRequest::new("root").select_root(choice.clone());
        let result = run(&root, &child, &request);
        result.validate().unwrap();
        assert_eq!(result.occurrences[1].path.0, vec![choice]);
        assert_eq!(
            result
                .chart
                .steps
                .iter()
                .filter(|step| step.label == "cleanup child")
                .count(),
            1
        );
    }
}

#[test]
fn unprotected_child_leaf_failures_escape_but_finally_throw_overrides_child_return() {
    let root = function(
        "root",
        json!([{"kind":"try","body":[{"kind":"evaluate","value":call("child")}],"catches":[{"catch_all":true,"body":[action("caller handler")]}]}]),
    );
    let child = function(
        "child",
        json!([{"kind":"action","text":"unprotected leaf","calls":[{"label":"leaf()"}]}]),
    );
    let result = expanded(&root, &child);
    let leaf = result
        .chart
        .steps
        .iter()
        .find(|step| step.label == "unprotected leaf")
        .unwrap()
        .id;
    let handler = result
        .chart
        .steps
        .iter()
        .find(|step| step.label == "caller handler")
        .unwrap()
        .id;
    assert!(
        result.occurrences[1]
            .boundaries
            .throws
            .iter()
            .any(|boundary| reaches(&result, leaf, *boundary, None)
                && reaches(&result, *boundary, handler, None))
    );
    let root = function(
        "root",
        json!([{"kind":"evaluate","value":call("child")},action("must not continue")]),
    );
    let child = function(
        "child",
        json!([{"kind":"try","body":[{"kind":"return"}],"finally":[{"kind":"throw","value":{"kind":"value","text":"override"}}]}]),
    );
    let result = expanded(&root, &child);
    result.validate().unwrap();
    assert!(result.occurrences[1].boundaries.returns.is_empty());
    assert!(!result.occurrences[1].boundaries.throws.is_empty());
    assert!(
        !result
            .chart
            .steps
            .iter()
            .any(|step| step.label == "must not continue")
    );
}

#[test]
fn escaping_child_break_continue_or_yield_is_a_visible_stop_not_a_caller_transfer() {
    let root = function(
        "root",
        json!([{"kind":"evaluate","value":call("child")},action("caller continuation")]),
    );
    for kind in ["break", "continue", "yield"] {
        let child = function(
            "child",
            json!([action("must not graft child"),{"kind":kind}]),
        );
        let result = expanded(&root, &child);
        result.validate().unwrap();
        assert_eq!(
            result.sources.len(),
            1,
            "Rejected staging must not retain or charge unused callee sources"
        );
        assert!(matches!(
            result.occurrences[1].outcome,
            flow::OccurrenceOutcome::Stopped {
                reason: flow::ExpansionStop::Ineligible,
                ..
            }
        ));
        assert!(
            !result
                .chart
                .steps
                .iter()
                .any(|step| step.label == "must not graft child")
        );
        assert!(
            result
                .chart
                .steps
                .iter()
                .any(|step| step.label == "caller continuation")
        );
    }
}

#[test]
fn callee_escape_summary_excludes_only_proven_reference_only_steps() {
    let root = function("root", json!([{"kind":"evaluate","value":call("child")} ]));
    let reference = json!({"label":"method group","targets":[{"symbol":"leaf","certainty":"reference","provenance":"explicit unit input"}]});
    let unknown = json!({"label":"unknown leaf","targets":[]});
    let mixed = json!({"label":"mixed leaf","targets":[{"symbol":"leaf","certainty":"reference"},{"symbol":"leaf","certainty":"possible"}]});
    for (calls, may_throw) in [
        (json!([reference]), false),
        (json!([unknown]), true),
        (json!([mixed]), true),
        (json!([reference, unknown]), true),
    ] {
        let child = function(
            "child",
            json!([{"kind":"evaluate","value":{"kind":"value","text":"leaf metadata","calls":calls}}]),
        );
        let result = expanded(&root, &child);
        result.validate().unwrap();
        assert_eq!(
            !result.occurrences[1].boundaries.throws.is_empty(),
            may_throw
        );
        let step = step_id(&result, "leaf metadata");
        assert_eq!(
            result.step_origins[step].calls.len(),
            calls.as_array().unwrap().len()
        );
        assert_eq!(
            serde_json::to_value(&result.chart.steps[step].calls).unwrap(),
            serde_json::to_value(
                flow::call_sites(&child)
                    .into_iter()
                    .rev()
                    .map(|site| site.site.clone())
                    .collect::<Vec<_>>()
            )
            .unwrap()
        );
        let call = result.occurrences[1].boundaries.call.unwrap();
        assert!(
            result
                .chart
                .links
                .iter()
                .any(|link| link.from == call && link.kind == "exception"),
            "Selected invocation pre-body failure remains independent of leaf-reference facts"
        );
    }
}
