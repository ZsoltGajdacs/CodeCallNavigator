//! Compiler facts authorize selection; identity, certainty and source facts survive unchanged.
use super::*;
fn unsupported_root(base: &Value, field: &str) -> FunctionFlowIR {
    let mut wire = serde_json::to_value(function("root", base.clone())).unwrap();
    match field {
        "timing" => wire["body"][0]["value"]["calls"][0]["invocation"] = json!({}),
        "dispatch" => {
            wire["body"][0]["value"]["calls"][0]["invocation"]["dispatch"] = json!("unknown")
        }
        "isolated_ordered" => {
            wire["body"][0]["value"]["calls"][0]["invocation"]["isolated_ordered"] = json!(false)
        }
        "reference" => {
            wire["body"][0]["value"]["calls"][0]["targets"][0]["certainty"] = json!("reference")
        }
        "http" | "navigation" => {
            wire["body"][0]["value"]["calls"][0]["targets"][0]["kind"] = json!(field)
        }
        "deferred_activation" => {
            wire["body"][0]["value"]["calls"][0]["invocation"] = json!({"timing":"deferred"})
        }
        _ => (),
    }
    serde_json::from_value(wire).unwrap()
}
fn assert_stopped(result: &ExpandedFlow, excluded: &str) {
    result.validate().unwrap();
    assert!(matches!(
        result.occurrences[1].outcome,
        flow::OccurrenceOutcome::Stopped { .. }
    ));
    assert!(!result.chart.steps.iter().any(|step| step.label == excluded));
    assert!(result.occurrences[1].boundaries.marker.is_some());
}
fn assert_ineligible_choice(root: &FunctionFlowIR, body: &FunctionFlowIR) {
    let original = flow::call_sites(root).remove(0);
    let expected = CallChoice {
        site: original.address,
        target: original.site.targets[0].clone(),
    };
    let result = expanded(root, body);
    assert_stopped(&result, "must not execute");
    assert!(matches!(
        result.occurrences[1].outcome,
        flow::OccurrenceOutcome::Stopped {
            reason: flow::ExpansionStop::Ineligible,
            ..
        }
    ));
    assert_eq!(result.sources.len(), 1);
    assert_eq!(result.occurrences[1].path.0, vec![expected.clone()]);
    let marker = result.occurrences[1].boundaries.marker.unwrap();
    assert!(
        result.chart.steps[marker]
            .calls
            .iter()
            .any(|call| call.targets.contains(&expected.target))
    );
}
#[test]
fn unknown_reference_missing_and_deferred_sites_remain_visible_stops_without_child_execution() {
    let child = function("child", json!([action("must not execute")]));
    let base = json!([{"kind":"evaluate","value":call("child")}]);
    for field in [
        "timing",
        "dispatch",
        "isolated_ordered",
        "reference",
        "body",
        "http",
        "navigation",
        "deferred_activation",
        "deferred_body",
    ] {
        let root = unsupported_root(&base, field);
        let mut body = child.clone();
        if field == "body" {
            body.execution = flow::ExecutionTiming::Unknown;
        } else if field == "deferred_body" {
            body.execution = flow::ExecutionTiming::Deferred;
        }
        assert_ineligible_choice(&root, &body);
    }
    let root = function(
        "root",
        json!([{"kind":"region","label":"callback","deferred":true,"body":base}]),
    );
    let result = expanded(&root, &child);
    assert_stopped(&result, "must not execute");
    let root = function("root", base);
    let result = flow::expand(
        &[FunctionSource {
            function: &root,
            content: SOURCE,
        }],
        &selection(&root),
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
}
#[test]
fn explicit_virtual_and_candidate_choices_preserve_full_uncertain_targets() {
    let child = function("child", json!([action("selected implementation")]));
    for dispatch in ["virtual", "candidate"] {
        let mut value = call("child");
        value["calls"][0]["invocation"]["dispatch"] = json!(dispatch);
        value["calls"][0]["targets"][0]["certainty"] = json!("possible");
        let root = function("root", json!([{"kind":"evaluate","value":value}]));
        let result = expanded(&root, &child);
        result.validate().unwrap();
        assert_eq!(result.occurrences[1].path.0[0].target.certainty, "possible");
        assert_eq!(
            result.chart.steps[result.occurrences[1].boundaries.call.unwrap()].calls[0].targets[0],
            result.occurrences[1].path.0[0].target
        );
    }
}
#[test]
fn stale_target_cleanup_context_and_cancellation_are_explicit_errors() {
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
    let site = flow::call_sites(&root).remove(0);
    let mut target = site.site.targets[0].clone();
    target.provenance = "stale proof".into();
    let request = ExpansionRequest::new("root").select_root(CallChoice {
        site: site.address.clone(),
        target,
    });
    assert!(flow::expand(&inputs, &request, &AtomicBool::new(false)).is_err());
    let mut address = site.address;
    address.cleanup.push(flow::CleanupCopy {
        site: flow::CleanupSite::Finally {
            block: flow::IrPath::default()
                .field(flow::IrField::Body)
                .index(0)
                .field(flow::IrField::Finally),
        },
        route: flow::PendingRoute::Throw,
    });
    let request = ExpansionRequest::new("root").select_root(CallChoice {
        site: address,
        target: site.site.targets[0].clone(),
    });
    assert!(flow::expand(&inputs, &request, &AtomicBool::new(false)).is_err());
    assert!(flow::expand(&inputs, &selection(&root), &AtomicBool::new(true)).is_err());
}
#[test]
fn coincident_spans_and_multiple_calls_do_not_imply_eligibility_or_duplicate_origins() {
    let child = function("child", json!([action("child body")]));
    let mut value = call("child");
    value["calls"][0]["invocation"]["isolated_ordered"] = json!(false);
    let second = value["calls"][0].clone();
    value["calls"].as_array_mut().unwrap().push(second);
    let root = function("root", json!([{"kind":"evaluate","value":value}]));
    let result = expanded(&root, &child);
    result.validate().unwrap();
    assert!(matches!(
        result.occurrences[1].outcome,
        flow::OccurrenceOutcome::Stopped { .. }
    ));
    let origins = result
        .step_origins
        .iter()
        .find(|origin| origin.calls.len() == 2)
        .unwrap();
    assert_ne!(origins.calls[0], origins.calls[1]);
    assert!(
        !result
            .chart
            .steps
            .iter()
            .any(|step| step.label == "child body")
    );
}

#[test]
fn valid_unproven_action_and_implicit_cleanup_selections_have_visible_ineligible_markers() {
    use flow::{CleanupCopy, CleanupSite, IrField, IrPath, PendingRoute};
    let unproven = json!({"kind":"action","text":"implicit or legacy call","calls":[{"label":"child()","targets":[{"symbol":"child","certainty":"reference","provenance":"implicit unit input"}]}]});
    let scope = IrPath::default().field(IrField::Body).index(0);
    let child = function("child", json!([action("must not execute")]));
    for (body, cleanup) in [
        (json!([unproven]), vec![]),
        (
            json!([{"kind":"try","resources":[action("acquire")],"body":[action("body")],"cleanup":[unproven]}]),
            vec![CleanupCopy {
                site: CleanupSite::Resource {
                    resource: scope.field(IrField::Resources).index(0),
                },
                route: PendingRoute::Normal,
            }],
        ),
    ] {
        let root = function("root", body);
        let original = flow::call_sites(&root).remove(0);
        let mut address = original.address;
        address.cleanup = cleanup;
        let request = ExpansionRequest::new("root").select_root(CallChoice {
            site: address,
            target: original.site.targets[0].clone(),
        });
        let result = run(&root, &child, &request);
        assert_stopped(&result, "must not execute");
        assert!(matches!(
            result.occurrences[1].outcome,
            flow::OccurrenceOutcome::Stopped {
                reason: flow::ExpansionStop::Ineligible,
                ..
            }
        ));
        assert_eq!(result.sources.len(), 1);
    }
}

#[test]
fn stopped_conditions_keep_the_original_single_decision_and_literal_routing() {
    let child = function("child", json!([action("must not execute")]));
    for text in ["flag", "false"] {
        let mut value = call("child");
        value["text"] = json!(text);
        value["calls"][0]["invocation"] = json!({});
        let root = function(
            "root",
            json!([{"kind":"if","condition":value,"then_branch":[action("yes branch")],"else_branch":[action("no branch")]}]),
        );
        let result = expanded(&root, &child);
        result.validate().unwrap();
        assert_eq!(
            serde_json::to_vec(&result.chart).unwrap(),
            serde_json::to_vec(&flow::build(&root).unwrap()).unwrap()
        );
        let marker = result.occurrences[1].boundaries.marker.unwrap();
        assert_eq!(
            (
                result.chart.steps[marker].kind.as_str(),
                result.chart.steps[marker].label.as_str(),
                result.step_origins[marker].role,
                result.chart.steps[marker].calls.len(),
                result.occurrences[1].boundaries.call.is_none()
            ),
            ("decision", text, flow::OriginRole::Marker, 1, true)
        );
    }
}
