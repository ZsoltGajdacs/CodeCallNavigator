//! Structural identity is independent of coincident spans and cleanup-route ordering.
use super::*;
use flow::{CallChoice, CleanupCopy, CleanupSite, IrField, IrPath, PendingRoute, SiteAddress};
use serde_json::json;
use std::collections::HashSet;

fn site() -> SiteAddress {
    flow::call_sites(&function())[0].address.clone()
}
fn scope() -> IrPath {
    IrPath::default().field(IrField::Body).index(0)
}

#[test]
fn structural_calls_distinguish_siblings_branches_nested_and_deferred_regions_with_equal_spans() {
    let mut wire = serde_json::to_value(function()).unwrap();
    let statement = wire["body"][0].clone();
    let value = statement["value"].clone();
    wire["body"] = json!([
        {"kind":"if","condition":value,"then_branch":[statement],"else_branch":[statement]},
        {"kind":"region","label":"callback","deferred":true,"body":[statement]},
        {"kind":"evaluate","value":{"kind":"sequence","items":[value,value]}},
        {"kind":"try","resources":[],"body":[],"catches":[{"body":[statement]}],"finally":[statement],"cleanup":[statement]}
    ]);
    let input: FunctionFlowIR = serde_json::from_value(wire).unwrap();
    let calls = flow::call_sites(&input);
    assert_eq!(calls.len(), 9);
    let addresses: HashSet<_> = calls.iter().map(|call| call.address.clone()).collect();
    assert_eq!(addresses.len(), calls.len());
    for call in &calls {
        call.address.validate().unwrap();
        assert_eq!(call.site.span, flow::SourceSpan { start: 0, end: 3 });
        assert!(
            call.address.cleanup.is_empty(),
            "Static listings do not invent execution copies"
        );
    }
    let paths: HashSet<_> = calls
        .iter()
        .map(|call| serde_json::to_string(&call.address.path).unwrap())
        .collect();
    for expected in [
        json!([{"field":"body"},{"index":0},{"field":"then_branch"},{"index":0},{"field":"value"}]),
        json!([{"field":"body"},{"index":0},{"field":"else_branch"},{"index":0},{"field":"value"}]),
        json!([{"field":"body"},{"index":1},{"field":"body"},{"index":0},{"field":"value"}]),
        json!([{"field":"body"},{"index":2},{"field":"value"},{"field":"items"},{"index":1}]),
        json!([{"field":"body"},{"index":3},{"field":"catches"},{"index":0},{"field":"body"},{"index":0},{"field":"value"}]),
    ] {
        assert!(
            paths.contains(&expected.to_string()),
            "Missing structural location {expected}"
        );
    }
}

#[test]
fn cleanup_addresses_keep_original_scope_and_pending_route_when_other_routes_are_inserted() {
    let mut retained = site();
    retained.cleanup = vec![
        CleanupCopy {
            site: CleanupSite::Finally {
                block: scope().field(IrField::Finally),
            },
            route: PendingRoute::Return,
        },
        CleanupCopy {
            site: CleanupSite::Resource {
                resource: scope().field(IrField::Resources).index(4),
            },
            route: PendingRoute::Break {
                label: Some("outer".into()),
            },
        },
        CleanupCopy {
            site: CleanupSite::Cleanup {
                block: scope().field(IrField::Cleanup),
            },
            route: PendingRoute::Normal,
        },
    ];
    retained.validate().unwrap();
    let bytes = serde_json::to_vec(&retained).unwrap();
    let mut inserted = retained.clone();
    inserted.cleanup[1].route = PendingRoute::Throw;
    let choices = HashSet::from([inserted, retained.clone()]);
    assert!(choices.contains(&serde_json::from_slice::<SiteAddress>(&bytes).unwrap()));
    assert_eq!(serde_json::to_vec(&retained).unwrap(), bytes);
    let mut vanished = choices;
    vanished.remove(&retained);
    assert!(
        !vanished.contains(&retained),
        "A vanished route must not rebind to another replay"
    );
    let mut nested = retained.clone();
    nested.cleanup.reverse();
    assert_ne!(
        nested, retained,
        "Full outer-to-inner context is significant"
    );
}

#[test]
fn exact_target_choice_preserves_certainty_provenance_and_repeated_occurrence_paths() {
    let target = function().body.into_iter().next().unwrap();
    let flow::StatementKind::Evaluate {
        value: flow::Expression::Value { calls, .. },
    } = target.kind
    else {
        panic!("fixture value")
    };
    let choice = CallChoice {
        site: site(),
        target: calls[0].targets[0].clone(),
    };
    let mut possible = choice.clone();
    possible.target.certainty = "possible".into();
    let mut other_proof = choice.clone();
    other_proof.target.provenance = "another compiler fact".into();
    assert_eq!(
        HashSet::from([choice.clone(), choice.clone(), possible, other_proof]).len(),
        3
    );
    let one = OccurrencePath(vec![choice.clone()]);
    let repeated = OccurrencePath(vec![choice.clone(), choice]);
    assert_ne!(one, repeated);
    assert_eq!(
        serde_json::from_slice::<OccurrencePath>(&serde_json::to_vec(&repeated).unwrap()).unwrap(),
        repeated
    );
}

#[test]
fn malformed_structural_owner_and_cleanup_scope_paths_are_rejected() {
    for path in [
        IrPath::default(),
        scope().index(0),
        scope().field(IrField::Items),
        scope().field(IrField::Calls).index(0),
    ] {
        let mut address = site();
        address.path = path;
        assert!(address.validate().is_err());
    }
    for cleanup in [
        CleanupCopy {
            site: CleanupSite::Resource {
                resource: scope().field(IrField::Resources),
            },
            route: PendingRoute::Normal,
        },
        CleanupCopy {
            site: CleanupSite::Cleanup {
                block: scope().field(IrField::Cleanup).index(0),
            },
            route: PendingRoute::Normal,
        },
        CleanupCopy {
            site: CleanupSite::Finally {
                block: scope().field(IrField::Body),
            },
            route: PendingRoute::Normal,
        },
        CleanupCopy {
            site: CleanupSite::Finally {
                block: scope().field(IrField::Finally),
            },
            route: PendingRoute::Continue {
                label: Some(String::new()),
            },
        },
    ] {
        let mut address = site();
        address.cleanup.push(cleanup);
        assert!(address.validate().is_err());
    }
}
