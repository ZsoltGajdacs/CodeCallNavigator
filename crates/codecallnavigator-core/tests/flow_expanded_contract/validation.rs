//! Pure result-boundary checks; malformed saved data cannot silently change owners.
use super::*;
use serde_json::{Value, json};

fn rejected(change: impl FnOnce(&mut Value)) {
    let mut wire = serde_json::to_value(root_only()).unwrap();
    change(&mut wire);
    let result: ExpandedFlow = serde_json::from_value(wire).unwrap();
    assert!(result.validate().is_err());
}

#[test]
fn expanded_results_reject_future_versions_invalid_graph_references_and_missing_origins() {
    rejected(|wire| wire["version"] = json!(2));
    rejected(|wire| wire["root_symbol"] = json!("other"));
    rejected(|wire| wire["chart"]["entry"] = json!(99));
    rejected(|wire| wire["chart"]["steps"][1]["id"] = json!(0));
    rejected(|wire| wire["chart"]["links"][0]["to"] = json!(99));
    rejected(|wire| {
        wire["chart"]["regions"] = json!([{"entry":99,"steps":[],"label":"region","deferred":true}])
    });
    rejected(|wire| {
        wire["step_origins"].as_array_mut().unwrap().pop();
    });
    rejected(|wire| wire["step_origins"][1]["occurrence"] = json!(99));
}

#[test]
fn expanded_results_reject_source_hash_owner_index_and_utf8_range_corruption() {
    rejected(|wire| wire["sources"][0]["digest"] = json!("incorrect"));
    rejected(|wire| wire["occurrences"][0]["owner"]["project"] = json!("wrong"));
    rejected(|wire| wire["sources"][0]["owner"]["file"] = json!(""));
    rejected(|wire| wire["occurrences"][0]["source"] = json!(9));
    rejected(|wire| wire["step_origins"][1]["source"] = json!(9));
    rejected(|wire| {
        let duplicate = wire["sources"][0].clone();
        wire["sources"].as_array_mut().unwrap().push(duplicate);
    });
    rejected(|wire| wire["chart"]["steps"][1]["span"] = json!({"start":2,"end":1}));
    rejected(|wire| wire["chart"]["steps"][1]["calls"][0]["span"] = json!({"start":0,"end":9}));
    let mut result = root_only();
    let content = "é()";
    let mut sources = SourceTable::default();
    sources.intern(owner(), content).unwrap();
    result.sources = sources;
    result.chart.steps[1].span = flow::SourceSpan { start: 1, end: 2 };
    assert!(
        result.validate().is_err(),
        "A range must not split a UTF-8 code point"
    );
    result.chart.steps[1].span = flow::SourceSpan::default();
    result.validate().unwrap();
}

#[test]
fn expanded_boundary_references_enforce_owner_and_role() {
    rejected(|wire| wire["occurrences"][0]["boundaries"]["call"] = json!(1));
    rejected(|wire| wire["occurrences"][0]["boundaries"]["entry"] = json!(99));
    rejected(|wire| wire["step_origins"][0]["role"] = json!("throw"));
    rejected(|wire| wire["occurrences"][0]["boundaries"]["returns"] = json!([1]));
    rejected(|wire| wire["occurrences"][0]["boundaries"]["throws"] = json!([1]));
    rejected(|wire| wire["occurrences"][0]["boundaries"]["marker"] = json!(1));
}

fn stopped() -> ExpandedFlow {
    let mut result = root_only();
    result.step_origins[1].role = OriginRole::Marker;
    let mut child = result.occurrences[0].clone();
    child.id = OccurrenceId(1);
    child.symbol = "target".into();
    child.parent = Some(OccurrenceId(0));
    child.source = None;
    child.owner = None;
    child.span = flow::SourceSpan::default();
    child.path = flow::OccurrencePath(vec![flow::CallChoice {
        site: result.step_origins[1].calls[0].clone(),
        target: result.chart.steps[1].calls[0].targets[0].clone(),
    }]);
    child.outcome = OccurrenceOutcome::Stopped {
        reason: flow::ExpansionStop::Unavailable,
        message: "Saved body unavailable".into(),
    };
    child.boundaries = BoundaryRefs {
        marker: Some(1),
        ..Default::default()
    };
    result.occurrences.push(child);
    result
}

#[test]
fn stopped_markers_roundtrip_but_cannot_hide_explanations_or_claim_an_assembled_body() {
    let result = stopped();
    result.validate().unwrap();
    let bytes = serde_json::to_vec(&result).unwrap();
    let saved: ExpandedFlow = serde_json::from_slice(&bytes).unwrap();
    saved.validate().unwrap();
    assert_eq!(serde_json::to_vec(&saved).unwrap(), bytes);
    let mut wire = serde_json::to_value(&result).unwrap();
    wire["occurrences"][1]["outcome"]["message"] = json!(" ");
    assert!(
        serde_json::from_value::<ExpandedFlow>(wire)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut invalid = stopped();
    invalid.chart.steps[1].label.clear();
    assert!(invalid.validate().is_err());
    let mut invalid = stopped();
    invalid.occurrences[1].boundaries.entry = Some(0);
    assert!(invalid.validate().is_err());
    let mut invalid = stopped();
    invalid.occurrences[1].path.0[0].target.symbol = "different".into();
    assert!(invalid.validate().is_err());
    let mut invalid = stopped();
    invalid.occurrences.push(invalid.occurrences[1].clone());
    invalid.occurrences[2].id = OccurrenceId(2);
    assert!(
        invalid.validate().is_err(),
        "A complete occurrence path cannot alias"
    );
}

#[test]
fn child_choices_require_an_exact_parent_site_target_and_optional_call_boundary() {
    let result = stopped();
    result.validate().unwrap();
    for field in ["certainty", "provenance", "kind"] {
        let mut invalid = stopped();
        let target = &mut invalid.occurrences[1].path.0[0].target;
        match field {
            "certainty" => target.certainty = "possible".into(),
            "provenance" => target.provenance = "another proof".into(),
            "kind" => target.kind = "navigation".into(),
            _ => unreachable!(),
        }
        assert!(
            invalid.validate().is_err(),
            "Full target field {field} must match the parent call"
        );
    }
    let mut invalid = stopped();
    invalid.occurrences[1].path.0[0].site.call_slot = 9;
    assert!(
        invalid.validate().is_err(),
        "A self-consistent child path still needs a parent call"
    );
    let mut invalid = stopped();
    let mut other_step = invalid.chart.steps[1].clone();
    other_step.id = invalid.chart.steps.len();
    let mut other_origin = invalid.step_origins[1].clone();
    other_origin.role = OriginRole::Call;
    other_origin.calls[0].path = flow::IrPath::default()
        .field(flow::IrField::Body)
        .index(1)
        .field(flow::IrField::Value);
    invalid.occurrences[1].boundaries.call = Some(other_step.id);
    invalid.chart.steps.push(other_step);
    invalid.step_origins.push(other_origin);
    assert!(
        invalid.validate().is_err(),
        "An explicit call boundary cannot redirect to another site in the same parent"
    );
}

#[test]
fn an_expanded_child_cannot_use_a_target_from_a_different_parent_call() {
    let mut result = stopped();
    result.step_origins[1].role = OriginRole::Call;
    let root_source = result.occurrences[0].source;
    let child = &mut result.occurrences[1];
    child.outcome = OccurrenceOutcome::Expanded;
    child.source = root_source;
    child.owner = Some(owner());
    child.span = result.chart.span;
    child.boundaries = BoundaryRefs {
        call: Some(1),
        entry: Some(3),
        ..Default::default()
    };
    let mut entry = result.chart.steps[0].clone();
    entry.id = 3;
    result.chart.steps.push(entry);
    result.step_origins.push(StepOrigin {
        occurrence: OccurrenceId(1),
        source: result.occurrences[0].source,
        role: OriginRole::Original,
        calls: vec![],
    });
    result.validate().unwrap();
    result.occurrences[1].path.0[0].target.provenance = "unrelated compiler evidence".into();
    assert!(result.validate().is_err());
}

#[test]
fn blocked_request_metadata_roundtrips_and_rejects_invalid_ancestor_or_branch_identity() {
    let mut result = stopped();
    let mut choice = result.occurrences[1].path.0[0].clone();
    choice.target.symbol = "deeper".into();
    result.blocked_selections.push(flow::BlockedSelection {
        caller: result.occurrences[1].path.clone(),
        choice,
        ancestor: OccurrenceId(1),
    });
    result.validate().unwrap();
    let bytes = serde_json::to_vec(&result).unwrap();
    let saved: ExpandedFlow = serde_json::from_slice(&bytes).unwrap();
    saved.validate().unwrap();
    assert_eq!(serde_json::to_vec(&saved).unwrap(), bytes);
    let mut invalid = result.clone();
    invalid.blocked_selections[0].ancestor = OccurrenceId(0);
    assert!(invalid.validate().is_err());
    let mut invalid = result.clone();
    invalid.blocked_selections[0].caller = OccurrencePath::default();
    assert!(invalid.validate().is_err());
    let mut invalid = result.clone();
    invalid
        .blocked_selections
        .push(invalid.blocked_selections[0].clone());
    assert!(invalid.validate().is_err());
    let mut invalid = result.clone();
    invalid.blocked_selections[0].choice.target.kind = "future_kind".into();
    assert!(invalid.validate().is_err());
    let mut invalid = result;
    invalid.blocked_selections[0].choice.target.symbol.clear();
    assert!(invalid.validate().is_err());
}
fn ghost_stopped_body() -> ExpandedFlow {
    let mut result = stopped();
    let mut step = result.chart.steps[1].clone();
    step.id = 3;
    step.span = flow::SourceSpan::default();
    step.label = "ghost callee call".into();
    step.calls[0].span = flow::SourceSpan::default();
    step.calls[0].targets[0].symbol = "deeper".into();
    let address = result.step_origins[1].calls[0].clone();
    result.chart.steps.push(step);
    result.step_origins.push(StepOrigin {
        occurrence: OccurrenceId(1),
        source: None,
        role: OriginRole::Marker,
        calls: vec![address],
    });
    result
}
#[test]
fn stopped_occurrences_cannot_own_ghost_calls_or_render_descendant_bodies() {
    let result = ghost_stopped_body();
    assert!(
        result.validate().is_err(),
        "A stopped occurrence cannot own a call-bearing body step"
    );
    let mut result = ghost_stopped_body();
    let mut descendant = result.occurrences[1].clone();
    descendant.id = OccurrenceId(2);
    descendant.parent = Some(OccurrenceId(1));
    descendant.symbol = "deeper".into();
    descendant.path.0.push(flow::CallChoice {
        site: result.step_origins[3].calls[0].clone(),
        target: result.chart.steps[3].calls[0].targets[0].clone(),
    });
    descendant.boundaries = BoundaryRefs {
        marker: Some(3),
        ..Default::default()
    };
    result.occurrences.push(descendant);
    assert!(
        result
            .validate()
            .unwrap_err()
            .to_string()
            .contains("stopped parent")
    );
}
