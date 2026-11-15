//! The final result contract can wrap a real ordinary chart without claiming callee assembly.
use codecallnavigator_core::flow::{
    self, BoundaryRefs, ExpandedFlow, FlowOccurrence, FunctionFlowIR, OccurrenceId,
    OccurrenceOutcome, OccurrencePath, OriginRole, SourceOwner, SourceTable, StepOrigin,
};

fn function() -> FunctionFlowIR {
    serde_json::from_str(include_str!("fixtures/invocation-facts/ir.json")).unwrap()
}
fn owner() -> SourceOwner {
    let function = function();
    SourceOwner {
        build: function.build,
        project: function.project,
        file: function.file,
        language: function.language,
    }
}
fn root_only() -> ExpandedFlow {
    let function = function();
    let chart = flow::build(&function).unwrap();
    let mut sources = SourceTable::default();
    let source = sources.intern(owner(), "f()").unwrap();
    let occurrence = FlowOccurrence {
        id: OccurrenceId(0),
        symbol: function.symbol.clone(),
        parent: None,
        path: OccurrencePath::default(),
        source: Some(source),
        owner: Some(owner()),
        span: function.span,
        outcome: OccurrenceOutcome::Root,
        boundaries: BoundaryRefs {
            entry: Some(chart.entry),
            ..Default::default()
        },
    };
    let step_origins = chart
        .steps
        .iter()
        .map(|step| StepOrigin {
            occurrence: OccurrenceId(0),
            source: Some(source),
            role: OriginRole::Original,
            calls: if step.calls.is_empty() {
                Vec::new()
            } else {
                flow::call_sites(&function)
                    .iter()
                    .map(|site| site.address.clone())
                    .collect()
            },
        })
        .collect();
    ExpandedFlow {
        version: 1,
        root_symbol: function.symbol,
        chart,
        occurrences: vec![occurrence],
        sources,
        step_origins,
        blocked_selections: vec![],
    }
}

#[test]
fn root_only_contract_roundtrips_with_the_exact_ordinary_chart_bytes() {
    let result = root_only();
    result.validate().unwrap();
    assert_eq!(
        serde_json::to_string(&result.chart).unwrap(),
        include_str!("fixtures/invocation-facts/chart.json")
    );
    let bytes = serde_json::to_vec(&result).unwrap();
    let saved: ExpandedFlow = serde_json::from_slice(&bytes).unwrap();
    saved.validate().unwrap();
    assert_eq!(serde_json::to_vec(&saved).unwrap(), bytes);
}

#[test]
fn source_interning_preserves_exact_utf8_bytes_and_ownership() {
    let mut table = SourceTable::default();
    let text = "\u{feff}é😀\r\nf()";
    let first = table.intern(owner(), text).unwrap();
    assert_eq!(table.intern(owner(), text).unwrap(), first);
    assert_eq!(
        table.get(first).unwrap().content.as_bytes(),
        text.as_bytes()
    );
    assert_eq!(
        table.get(first).unwrap().digest,
        blake3::hash(text.as_bytes()).to_hex().to_string()
    );
    let mut other = owner();
    other.project = "other".into();
    assert_ne!(table.intern(other, text).unwrap(), first);
    assert_eq!(table.byte_count().unwrap(), text.len() * 2);
    assert!(table.intern(owner(), "changed").is_err());
    table.validate().unwrap();
}

#[test]
fn structural_sites_use_ir_fields_and_slots_not_spans_or_enumeration() {
    let function = function();
    let sites = flow::call_sites(&function);
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].address.call_slot, 0);
    assert!(sites[0].address.cleanup.is_empty());
    assert_eq!(
        serde_json::to_value(&sites[0].address.path).unwrap(),
        serde_json::json!([
            {"field":"body"},{"index":0},{"field":"value"}
        ])
    );
    assert_eq!(sites[0].site.label, "f()");
}

#[test]
fn each_visible_call_retains_its_structural_origin_in_the_result_wire() {
    let result = root_only();
    let input = function();
    let sites = flow::call_sites(&input);
    let address = serde_json::to_value(&sites[0].address).unwrap();
    let mut wire = serde_json::to_value(result).unwrap();
    let index = wire["chart"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .position(|step| !step["calls"].as_array().unwrap().is_empty())
        .unwrap();
    wire["step_origins"][index]["calls"] = serde_json::json!([address]);
    let saved: ExpandedFlow = serde_json::from_value(wire.clone()).unwrap();
    saved.validate().unwrap();
    assert_eq!(
        serde_json::to_value(saved).unwrap()["step_origins"][index]["calls"],
        wire["step_origins"][index]["calls"]
    );
}

#[test]
fn visible_call_origins_require_exact_cardinality_and_distinct_full_addresses() {
    let mut wire = serde_json::to_value(root_only()).unwrap();
    let index = wire["chart"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .position(|step| !step["calls"].as_array().unwrap().is_empty())
        .unwrap();
    wire["step_origins"][index]["calls"] = serde_json::json!([]);
    let missing: ExpandedFlow = serde_json::from_value(wire.clone()).unwrap();
    assert!(
        missing.validate().is_err(),
        "A visible call cannot lose its structural origin"
    );
    let call = wire["chart"]["steps"][index]["calls"][0].clone();
    wire["chart"]["steps"][index]["calls"] = serde_json::json!([call, call]);
    let address = serde_json::to_value(&flow::call_sites(&function())[0].address).unwrap();
    wire["step_origins"][index]["calls"] = serde_json::json!([address, address]);
    let duplicate: ExpandedFlow = serde_json::from_value(wire).unwrap();
    assert!(
        duplicate.validate().is_err(),
        "Identical full call addresses must not alias"
    );
}

#[test]
fn stopped_occurrences_require_an_explicit_visible_marker() {
    let mut result = root_only();
    let mut child = result.occurrences[0].clone();
    child.id = OccurrenceId(1);
    child.symbol = "callee".into();
    child.parent = Some(OccurrenceId(0));
    child.path = flow::OccurrencePath(vec![flow::CallChoice {
        site: flow::call_sites(&function())[0].address.clone(),
        target: flow::CallTarget {
            symbol: "callee".into(),
            kind: "call".into(),
            certainty: "possible".into(),
            provenance: "compiler".into(),
        },
    }]);
    child.outcome = OccurrenceOutcome::Stopped {
        reason: flow::ExpansionStop::Unavailable,
        message: "No saved body".into(),
    };
    child.boundaries = BoundaryRefs::default();
    result.occurrences.push(child);
    assert!(
        result.validate().is_err(),
        "Stopped work must remain visible in the chart"
    );
}

#[test]
fn a_site_address_must_identify_an_ir_owner_instead_of_an_unindexed_list() {
    let address = flow::SiteAddress {
        path: flow::IrPath::default().field(flow::IrField::Body),
        call_slot: 0,
        cleanup: vec![],
    };
    assert!(address.validate().is_err());
}

#[path = "flow_expanded_contract/identity.rs"]
mod identity;
#[path = "flow_expanded_contract/validation.rs"]
mod validation;
