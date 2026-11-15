//! Public limits constrain complete assemblies, including work after a child graft.
use super::*;
use flow::{ExpansionLimit, ExpansionLimits, ExpansionStop, OccurrenceOutcome};
#[test]
fn zero_depth_and_request_allowances_return_typed_stops_without_changing_collapsed_charts() {
    let root = function("root", json!([{"kind":"evaluate","value":call("child")} ]));
    let child = function("child", json!([action("must not expand")]));
    for (limits, expected) in [
        (
            ExpansionLimits {
                depth: 0,
                ..Default::default()
            },
            ExpansionLimit::Depth,
        ),
        (
            ExpansionLimits {
                requests: 0,
                ..Default::default()
            },
            ExpansionLimit::Requests,
        ),
    ] {
        let request = selection(&root).with_limits(limits);
        let result = run(&root, &child, &request);
        assert!(
            matches!(result.occurrences[1].outcome,OccurrenceOutcome::Stopped{reason:ExpansionStop::Limit{limit},..} if limit==expected)
        );
        assert_eq!(
            serde_json::to_vec(&result.chart).unwrap(),
            serde_json::to_vec(&flow::build(&root).unwrap()).unwrap()
        );
        assert_eq!(result.sources.len(), 1);
    }
}

fn stopped_limit(result: &ExpandedFlow, index: usize, limit: ExpansionLimit) {
    assert!(
        matches!(result.occurrences[index].outcome,OccurrenceOutcome::Stopped{reason:ExpansionStop::Limit{limit:actual},..} if actual==limit)
    );
}
fn run_inputs(inputs: &[FunctionSource<'_>], request: &ExpansionRequest) -> ExpandedFlow {
    flow::expand(inputs, request, &AtomicBool::new(false)).unwrap()
}
#[test]
fn exact_graph_limits_succeed_and_steps_or_links_independently_demote_the_child() {
    let root = function("root", json!([{"kind":"evaluate","value":call("child")} ]));
    let child = function("child", json!([action("child body")]));
    let full = expanded(&root, &child);
    let limits = ExpansionLimits {
        steps: full.chart.steps.len(),
        links: full.chart.links.len(),
        source_bytes: full.sources.byte_count().unwrap() as u64,
        ..Default::default()
    };
    let exact = run(&root, &child, &selection(&root).with_limits(limits));
    assert_eq!(
        serde_json::to_vec(&exact).unwrap(),
        serde_json::to_vec(&full).unwrap()
    );
    for (limits, limit) in [
        (
            ExpansionLimits {
                steps: limits.steps - 1,
                ..limits
            },
            ExpansionLimit::Steps,
        ),
        (
            ExpansionLimits {
                links: limits.links - 1,
                ..limits
            },
            ExpansionLimit::Links,
        ),
    ] {
        let result = run(&root, &child, &selection(&root).with_limits(limits));
        stopped_limit(&result, 1, limit);
        assert_eq!(
            serde_json::to_vec(&result.chart).unwrap(),
            serde_json::to_vec(&flow::build(&root).unwrap()).unwrap()
        );
        assert_eq!(result.sources.len(), 1);
    }
}
#[test]
fn caller_suffix_and_duplicated_cleanup_overflow_demote_an_earlier_complete_graft() {
    let suffix: Vec<_> = (0..12)
        .map(|index| action(&format!("caller suffix {index}")))
        .collect();
    let cleanup: Vec<_> = (0..8)
        .map(|index| action(&format!("caller cleanup {index}")))
        .collect();
    let mut body = vec![json!({"kind":"evaluate","value":call("child")})];
    body.extend(suffix);
    let variants = [
        json!(body),
        json!([{"kind":"try","body":[{"kind":"evaluate","value":call("child")}],"finally":cleanup}]),
    ];
    let child = function(
        "child",
        json!([
            action("child1"),
            action("child2"),
            action("child3"),
            action("child4")
        ]),
    );
    for body in variants {
        let root = function("root", body);
        let collapsed = flow::build(&root).unwrap();
        let limits = ExpansionLimits {
            steps: collapsed.steps.len() + 2,
            ..Default::default()
        };
        let result = run(&root, &child, &selection(&root).with_limits(limits));
        stopped_limit(&result, 1, ExpansionLimit::Steps);
        assert_eq!(
            serde_json::to_vec(&result.chart).unwrap(),
            serde_json::to_vec(&collapsed).unwrap()
        );
        assert_eq!(result.sources.len(), 1);
    }
}
fn selected_at(function: &FunctionFlowIR, body_index: usize) -> CallChoice {
    let path = flow::IrPath::default()
        .field(flow::IrField::Body)
        .index(body_index)
        .field(flow::IrField::Value);
    let site = flow::call_sites(function)
        .into_iter()
        .find(|site| site.address.path == path)
        .unwrap();
    CallChoice {
        site: site.address,
        target: site.site.targets[0].clone(),
    }
}
#[test]
fn demoting_an_always_throwing_child_can_restore_a_larger_suffix_and_force_another_retry() {
    let mut body = vec![
        json!({"kind":"evaluate","value":call("a")}),
        json!({"kind":"evaluate","value":call("b")}),
    ];
    body.extend((0..12).map(|index| action(&format!("restored suffix {index}"))));
    let root = function("root", json!(body));
    let a = function(
        "a",
        json!([action("a1"), action("a2"), action("a3"), action("a4")]),
    );
    let mut throwing: Vec<_> = (0..8).map(|index| action(&format!("b{index}"))).collect();
    throwing.push(json!({"kind":"throw","value":{"kind":"value","text":"failure"}}));
    let b = function("b", json!(throwing));
    let first = selected_at(&root, 0);
    let second = selected_at(&root, 1);
    let request = ExpansionRequest::new("root")
        .select_root(first.clone())
        .select_root(second.clone());
    let inputs = [
        FunctionSource {
            function: &root,
            content: SOURCE,
        },
        FunctionSource {
            function: &a,
            content: SOURCE,
        },
        FunctionSource {
            function: &b,
            content: SOURCE,
        },
    ];
    let full = run_inputs(&inputs, &request);
    assert!(
        !full
            .chart
            .steps
            .iter()
            .any(|step| step.label.starts_with("restored suffix"))
    );
    let collapsed = flow::build(&root).unwrap();
    let limits = ExpansionLimits {
        steps: collapsed.steps.len(),
        ..Default::default()
    };
    let result = run_inputs(&inputs, &request.with_limits(limits));
    stopped_limit(&result, 1, ExpansionLimit::Steps);
    stopped_limit(&result, 2, ExpansionLimit::Steps);
    assert_eq!(
        serde_json::to_vec(&result.chart).unwrap(),
        serde_json::to_vec(&collapsed).unwrap()
    );
    let reversed = ExpansionRequest::new("root")
        .select_root(second)
        .select_root(first)
        .with_limits(limits);
    let permuted = run_inputs(&[inputs[2], inputs[0], inputs[1]], &reversed);
    assert_eq!(
        serde_json::to_vec(&result).unwrap(),
        serde_json::to_vec(&permuted).unwrap()
    );
    assert_eq!(result.sources.len(), 1);
}
#[test]
fn source_allowance_deduplicates_owners_and_stops_oversized_children_before_copying() {
    let root = function("root", json!([{"kind":"evaluate","value":call("child")} ]));
    let mut shared = function("child", json!([action("shared owner")]));
    shared.file = root.file.clone();
    let limits = ExpansionLimits {
        source_bytes: SOURCE.len() as u64,
        ..Default::default()
    };
    let result = run(&root, &shared, &selection(&root).with_limits(limits));
    assert_eq!(result.sources.len(), 1);
    assert_eq!(result.occurrences[1].source, result.occurrences[0].source);
    let child = function("child", json!([{"kind":"evaluate","value":call("leaf")} ]));
    let leaf = function("leaf", json!([action("must not copy")]));
    // Crossing the old hard child cap proves admission occurs before text
    // materialization, rather than rejecting this borrowed source in Prepared.
    let large = "x".repeat(flow::MAX_SOURCE_BYTES as usize + 1);
    let outer = selected_at(&root, 0);
    let inner = selected_at(&child, 0);
    let request = ExpansionRequest::new("root")
        .select_root(outer.clone())
        .select(flow::OccurrencePath(vec![outer]), inner)
        .with_limits(limits);
    let inputs = [
        FunctionSource {
            function: &root,
            content: SOURCE,
        },
        FunctionSource {
            function: &child,
            content: &large,
        },
        FunctionSource {
            function: &leaf,
            content: SOURCE,
        },
    ];
    let limited = run_inputs(&inputs, &request);
    stopped_limit(&limited, 1, ExpansionLimit::SourceBytes);
    assert_eq!(
        (limited.sources.len(), limited.blocked_selections.len()),
        (1, 1)
    );
    let oversized_root = [FunctionSource {
        function: &root,
        content: &large,
    }];
    assert!(
        flow::expand(
            &oversized_root,
            &ExpansionRequest::new("root"),
            &AtomicBool::new(false)
        )
        .is_err()
    );
}
#[test]
fn evaluation_requests_spent_inside_rejected_nested_staging_are_not_refunded() {
    let root = function(
        "root",
        json!([{"kind":"evaluate","value":call("child")},{"kind":"evaluate","value":call("other")} ]),
    );
    let child = function(
        "child",
        json!([{"kind":"evaluate","value":call("leaf")},{"kind":"break"}]),
    );
    let leaf = function("leaf", json!([action("discarded leaf")]));
    let other = function("other", json!([action("must not evaluate")]));
    let outer = selected_at(&root, 0);
    let request = ExpansionRequest::new("root")
        .select_root(outer.clone())
        .select(flow::OccurrencePath(vec![outer]), selected_at(&child, 0))
        .select_root(selected_at(&root, 1))
        .with_limits(ExpansionLimits {
            requests: 2,
            ..Default::default()
        });
    let result = run_inputs(
        &[
            FunctionSource {
                function: &root,
                content: SOURCE,
            },
            FunctionSource {
                function: &child,
                content: SOURCE,
            },
            FunctionSource {
                function: &leaf,
                content: SOURCE,
            },
            FunctionSource {
                function: &other,
                content: SOURCE,
            },
        ],
        &request,
    );
    assert!(matches!(
        result.occurrences[1].outcome,
        OccurrenceOutcome::Stopped {
            reason: ExpansionStop::Ineligible,
            ..
        }
    ));
    stopped_limit(&result, 2, ExpansionLimit::Requests);
    assert_eq!(
        (result.sources.len(), result.blocked_selections.len()),
        (1, 1)
    );
}
#[test]
fn invalid_limits_and_mandatory_root_exhaustion_are_errors_and_cancel_is_not_a_limit_stop() {
    let root = function("root", json!([{"kind":"evaluate","value":call("child")} ]));
    let child = function("child", json!([action("child")]));
    let inputs = [
        FunctionSource {
            function: &root,
            content: SOURCE,
        },
        FunctionSource {
            function: &child,
            content: SOURCE,
        },
    ];
    let invalid = invalid_limit_inputs();
    for limits in invalid {
        assert!(
            flow::expand(
                &inputs,
                &selection(&root).with_limits(limits),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    let collapsed = flow::build(&root).unwrap();
    for limits in [
        ExpansionLimits {
            steps: collapsed.steps.len() - 1,
            ..Default::default()
        },
        ExpansionLimits {
            links: collapsed.links.len() - 1,
            ..Default::default()
        },
        ExpansionLimits {
            source_bytes: SOURCE.len() as u64 - 1,
            ..Default::default()
        },
    ] {
        assert!(
            flow::expand(
                &inputs,
                &selection(&root).with_limits(limits),
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
    let error = flow::expand(&inputs, &selection(&root), &AtomicBool::new(true)).unwrap_err();
    assert!(error.to_string().contains("cancelled"));
}

fn invalid_limit_inputs() -> [ExpansionLimits; 8] {
    [
        ExpansionLimits {
            depth: 33,
            ..Default::default()
        },
        ExpansionLimits {
            requests: 129,
            ..Default::default()
        },
        ExpansionLimits {
            steps: 0,
            ..Default::default()
        },
        ExpansionLimits {
            steps: 1501,
            ..Default::default()
        },
        ExpansionLimits {
            links: 0,
            ..Default::default()
        },
        ExpansionLimits {
            links: 6001,
            ..Default::default()
        },
        ExpansionLimits {
            source_bytes: 0,
            ..Default::default()
        },
        ExpansionLimits {
            source_bytes: flow::MAX_SOURCE_BYTES + 1,
            ..Default::default()
        },
    ]
}
