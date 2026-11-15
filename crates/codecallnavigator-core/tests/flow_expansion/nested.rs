//! Explicit occurrence paths distinguish nested activation from repeated siblings.
use super::*;
fn choice(function: &FunctionFlowIR, index: usize) -> CallChoice {
    let sites = flow::call_sites(function);
    CallChoice {
        site: sites[index].address.clone(),
        target: sites[index].site.targets[0].clone(),
    }
}
#[test]
fn explicitly_selected_nested_calls_execute_in_their_own_source_occurrences() {
    let root = function(
        "root",
        json!([{"kind":"evaluate","value":call("child")},action("root after")]),
    );
    let child = function(
        "child",
        json!([{"kind":"evaluate","value":call("leaf")},action("child after")]),
    );
    let leaf = function("leaf", json!([action("nested leaf body")]));
    let outer = choice(&root, 0);
    let inner = choice(&child, 0);
    let request = ExpansionRequest::new("root")
        .select_root(outer.clone())
        .select(flow::OccurrencePath(vec![outer.clone()]), inner.clone());
    let inputs = [
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
    ];
    let result = flow::expand(&inputs, &request, &AtomicBool::new(false)).unwrap();
    result.validate().unwrap();
    assert_eq!(
        (
            result.occurrences.len(),
            result.sources.len(),
            &result.occurrences[2].path
        ),
        (3, 3, &flow::OccurrencePath(vec![outer, inner]))
    );
    let body = step_id(&result, "nested leaf body");
    assert_eq!(result.step_origins[body].occurrence, flow::OccurrenceId(2));
    assert!(reaches(&result, result.chart.entry, body, None));
    assert!(reaches(
        &result,
        result.occurrences[2].boundaries.returns[0],
        step_id(&result, "child after"),
        None
    ));
}

fn run_many(functions: &[&FunctionFlowIR], request: &ExpansionRequest) -> ExpandedFlow {
    let inputs: Vec<_> = functions
        .iter()
        .map(|function| FunctionSource {
            function,
            content: SOURCE,
        })
        .collect();
    flow::expand(&inputs, request, &AtomicBool::new(false)).unwrap()
}
#[test]
fn direct_and_mutual_recursion_stop_on_the_active_path_without_rebinding() {
    let root = function("root", json!([{"kind":"evaluate","value":call("root")} ]));
    let result = run_many(&[&root], &selection(&root));
    assert!(matches!(
        result.occurrences[1].outcome,
        flow::OccurrenceOutcome::Stopped {
            reason: flow::ExpansionStop::Cycle,
            ..
        }
    ));
    assert_eq!(result.sources.len(), 1);
    let root = function("root", json!([{"kind":"evaluate","value":call("child")} ]));
    let child = function("child", json!([{"kind":"evaluate","value":call("root")} ]));
    let outer = choice(&root, 0);
    let request = ExpansionRequest::new("root")
        .select_root(outer.clone())
        .select(flow::OccurrencePath(vec![outer]), choice(&child, 0));
    let result = run_many(&[&root, &child], &request);
    assert_eq!(result.occurrences.len(), 3);
    assert!(matches!(
        result.occurrences[2].outcome,
        flow::OccurrenceOutcome::Stopped {
            reason: flow::ExpansionStop::Cycle,
            ..
        }
    ));
    assert_eq!(result.occurrences[2].parent, Some(flow::OccurrenceId(1)));
}
#[test]
fn repeated_siblings_and_diamond_callees_expand_independently_and_deterministically() {
    let mut left_call = call("left");
    let mut right_call = call("right");
    for value in [&mut left_call, &mut right_call] {
        value["calls"][0]["invocation"]["dispatch"] = json!("candidate");
        value["calls"][0]["targets"][0]["certainty"] = json!("possible");
    }
    let root = function(
        "root",
        json!([{"kind":"evaluate","value":left_call},{"kind":"evaluate","value":right_call}]),
    );
    let left = function("left", json!([{"kind":"evaluate","value":call("common")} ]));
    let right = function(
        "right",
        json!([{"kind":"evaluate","value":call("common")} ]),
    );
    let common = function("common", json!([action("shared body")]));
    // Parts lists calls in validation order; explicitly choose the original body indexes.
    let l = choice(&root, 1);
    let r = choice(&root, 0);
    let requests = [
        (flow::OccurrencePath::default(), l.clone()),
        (flow::OccurrencePath(vec![l.clone()]), choice(&left, 0)),
        (flow::OccurrencePath::default(), r.clone()),
        (flow::OccurrencePath(vec![r.clone()]), choice(&right, 0)),
    ];
    let request = requests.iter().fold(
        ExpansionRequest::new("root"),
        |request, (caller, choice)| request.select(caller.clone(), choice.clone()),
    );
    let reversed = requests.iter().rev().fold(
        ExpansionRequest::new("root"),
        |request, (caller, choice)| request.select(caller.clone(), choice.clone()),
    );
    let result = run_many(&[&root, &left, &right, &common], &request);
    let other = run_many(&[&common, &right, &root, &left], &reversed);
    assert_eq!(
        serde_json::to_vec(&result).unwrap(),
        serde_json::to_vec(&other).unwrap()
    );
    assert_eq!(
        (
            result
                .occurrences
                .iter()
                .filter(|occurrence| occurrence.symbol == "common")
                .count(),
            result.sources.len()
        ),
        (2, 4)
    );
    assert!(
        result
            .occurrences
            .iter()
            .skip(1)
            .all(|occurrence| occurrence.path.0[0].target.certainty == "possible")
    );
    let root = function(
        "root",
        json!([{"kind":"evaluate","value":call("common")},{"kind":"evaluate","value":call("common")} ]),
    );
    let request = ExpansionRequest::new("root")
        .select_root(choice(&root, 0))
        .select_root(choice(&root, 1));
    let repeated = run_many(&[&root, &common], &request);
    assert_eq!((repeated.occurrences.len(), repeated.sources.len()), (3, 2));
    assert_ne!(repeated.occurrences[1].path, repeated.occurrences[2].path);
}
#[test]
fn nested_cleanup_selection_keeps_both_occurrence_and_semantic_route_identity() {
    let root = function("root", json!([{"kind":"evaluate","value":call("child")} ]));
    let child = function(
        "child",
        json!([{"kind":"try","body":[{"kind":"return"}],"finally":[{"kind":"evaluate","value":call("leaf")}]}]),
    );
    let leaf = function("leaf", json!([action("nested cleanup body")]));
    let outer = choice(&root, 0);
    let mut inner = choice(&child, 0);
    inner.site.cleanup = vec![flow::CleanupCopy {
        site: flow::CleanupSite::Finally {
            block: flow::IrPath::default()
                .field(flow::IrField::Body)
                .index(0)
                .field(flow::IrField::Finally),
        },
        route: flow::PendingRoute::Return,
    }];
    let request = ExpansionRequest::new("root")
        .select_root(outer.clone())
        .select(flow::OccurrencePath(vec![outer.clone()]), inner.clone());
    let result = run_many(&[&root, &child, &leaf], &request);
    assert_eq!(
        result.occurrences[2].path,
        flow::OccurrencePath(vec![outer, inner])
    );
    assert!(reaches(
        &result,
        result.chart.entry,
        step_id(&result, "nested cleanup body"),
        None
    ));
}
#[test]
fn stopped_ancestors_explain_blocked_descendants_without_materializing_ghost_sources() {
    let root = function("root", json!([{"kind":"evaluate","value":call("child")} ]));
    let mut child = function("child", json!([{"kind":"evaluate","value":call("leaf")} ]));
    child.execution = flow::ExecutionTiming::Unknown;
    let leaf = function("leaf", json!([action("must not render")]));
    let outer = choice(&root, 0);
    let inner = choice(&child, 0);
    let request = ExpansionRequest::new("root")
        .select_root(outer.clone())
        .select(flow::OccurrencePath(vec![outer]), inner.clone());
    let result = run_many(&[&root, &child, &leaf], &request);
    assert_eq!(
        (
            result.occurrences.len(),
            result.sources.len(),
            result.blocked_selections.len()
        ),
        (2, 1, 1)
    );
    assert_eq!(
        (
            &result.blocked_selections[0].choice,
            result.blocked_selections[0].ancestor
        ),
        (&inner, flow::OccurrenceId(1))
    );
    assert!(
        matches!(&result.occurrences[1].outcome,flow::OccurrenceOutcome::Stopped{message,..} if message.contains("1 selected descendants blocked"))
    );
    let missing = run_many(&[&root, &leaf], &request);
    assert_eq!(
        (missing.blocked_selections.len(), missing.sources.len()),
        (1, 1)
    );
}
#[test]
fn rejected_nested_staging_rolls_back_occurrences_statuses_and_borrowed_sources() {
    let root = function("root", json!([{"kind":"evaluate","value":call("child")} ]));
    let child = function(
        "child",
        json!([{"kind":"evaluate","value":call("leaf")},{"kind":"break"}]),
    );
    let leaf = function("leaf", json!([action("must roll back")]));
    let outer = choice(&root, 0);
    let inner = choice(&child, 0);
    let request = ExpansionRequest::new("root")
        .select_root(outer.clone())
        .select(flow::OccurrencePath(vec![outer]), inner);
    let result = run_many(&[&root, &child, &leaf], &request);
    result.validate().unwrap();
    assert_eq!(
        (
            result.occurrences.len(),
            result.sources.len(),
            result.blocked_selections.len()
        ),
        (2, 1, 1)
    );
    assert!(
        !result
            .chart
            .steps
            .iter()
            .any(|step| step.label == "must roll back")
    );
}
#[test]
fn duplicate_orphan_stale_conflicting_input_and_oversized_requests_are_rejected() {
    let root = function("root", json!([{"kind":"evaluate","value":call("child")} ]));
    let child = function("child", json!([{"kind":"evaluate","value":call("leaf")} ]));
    let leaf = function("leaf", json!([action("leaf")]));
    let inputs = [
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
    ];
    assert_rejected_requests(&inputs, &root, &child);
    assert_request_bounds(&inputs, &choice(&root, 0));
    assert_invalid_inputs(&inputs, &root, &child);
}
fn assert_rejected_requests(
    inputs: &[FunctionSource<'_>],
    root: &FunctionFlowIR,
    child: &FunctionFlowIR,
) {
    let outer = choice(root, 0);
    let inner = choice(child, 0);
    let duplicate = ExpansionRequest::new("root")
        .select_root(outer.clone())
        .select_root(outer.clone());
    let orphan = ExpansionRequest::new("root")
        .select(flow::OccurrencePath(vec![outer.clone()]), inner.clone());
    let mut stale = inner;
    stale.target.provenance = "stale".into();
    let stale = ExpansionRequest::new("root")
        .select_root(outer.clone())
        .select(flow::OccurrencePath(vec![outer.clone()]), stale);
    let oversized = (0..129).fold(ExpansionRequest::new("root"), |request, _| {
        request.select_root(outer.clone())
    });
    for request in [duplicate, orphan, stale, oversized] {
        assert!(flow::expand(inputs, &request, &AtomicBool::new(false)).is_err());
    }
    for field in ["kind", "symbol"] {
        let mut invalid = outer.clone();
        if field == "kind" {
            invalid.target.kind = "future_kind".into();
        } else {
            invalid.target.symbol.clear();
        }
        let blocked_request = ExpansionRequest::new("root")
            .select_root(outer.clone())
            .select(flow::OccurrencePath(vec![outer.clone()]), invalid);
        assert!(flow::expand(&inputs[..1], &blocked_request, &AtomicBool::new(false)).is_err());
    }
}
fn assert_request_bounds(inputs: &[FunctionSource<'_>], outer: &CallChoice) {
    let mut large = outer.clone();
    large.target.provenance = "x".repeat(1024 * 1024);
    let error = flow::expand(
        inputs,
        &ExpansionRequest::new("root").select_root(large),
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(error.to_string().contains("1 MiB"));
    let deep = ExpansionRequest::new("root")
        .select(flow::OccurrencePath(vec![outer.clone(); 32]), outer.clone());
    assert!(
        flow::expand(inputs, &deep, &AtomicBool::new(false))
            .unwrap_err()
            .to_string()
            .contains("depth exceeds 32")
    );
}
fn assert_invalid_inputs(
    inputs: &[FunctionSource<'_>],
    root: &FunctionFlowIR,
    child: &FunctionFlowIR,
) {
    for field in ["build", "project", "file", "language", "symbol"] {
        let mut unused =
            serde_json::to_value(function("unused", json!([action("unused")]))).unwrap();
        unused[field] = json!("");
        let unused: FunctionFlowIR = serde_json::from_value(unused).unwrap();
        let invalid = [
            inputs[0],
            inputs[1],
            FunctionSource {
                function: &unused,
                content: SOURCE,
            },
        ];
        assert!(
            flow::expand(&invalid, &selection(root), &AtomicBool::new(false)).is_err(),
            "Invalid provided {field} must be rejected before use"
        );
    }
    let duplicate_inputs = [inputs[0], inputs[1], inputs[1]];
    assert!(flow::expand(&duplicate_inputs, &selection(root), &AtomicBool::new(false)).is_err());
    let mut other = child.clone();
    other.symbol = "different id".into();
    let conflicting = [
        inputs[0],
        inputs[1],
        FunctionSource {
            function: &other,
            content: "different source bytes",
        },
    ];
    assert!(flow::expand(&conflicting, &selection(root), &AtomicBool::new(false)).is_err());
}

#[test]
fn multiple_legacy_slots_share_one_preserved_marker_in_original_call_order() {
    let mut first = call("first");
    first["calls"][0]["invocation"] = json!({});
    let mut second = call("second");
    second["calls"][0]["invocation"] = json!({});
    first["calls"]
        .as_array_mut()
        .unwrap()
        .push(second["calls"][0].clone());
    let root = function("root", json!([{"kind":"evaluate","value":first}]));
    let first_body = function("first", json!([action("must not render first")]));
    let second_body = function("second", json!([action("must not render second")]));
    let request = ExpansionRequest::new("root")
        .select_root(choice(&root, 0))
        .select_root(choice(&root, 1));
    let result = run_many(&[&root, &first_body, &second_body], &request);
    result.validate().unwrap();
    assert_eq!(
        result
            .occurrences
            .iter()
            .map(|occurrence| occurrence.symbol.as_str())
            .collect::<Vec<_>>(),
        vec!["root", "first", "second"]
    );
    let marker = result.occurrences[1].boundaries.marker.unwrap();
    assert_eq!(
        (
            result.occurrences[2].boundaries.marker,
            result
                .occurrences
                .iter()
                .skip(1)
                .all(|occurrence| occurrence.boundaries.call.is_none())
        ),
        (Some(marker), true)
    );
    assert_eq!(
        serde_json::to_vec(&result.chart).unwrap(),
        serde_json::to_vec(&flow::build(&root).unwrap()).unwrap()
    );
    assert_eq!(
        result.step_origins[marker]
            .calls
            .iter()
            .map(|site| site.call_slot)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}
