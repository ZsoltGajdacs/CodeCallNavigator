use codegraph::model::{relationship_is_reference, relationship_is_uncertain};

#[test]
fn virtual_calls_are_visible_when_other_uncertain_targets_are_hidden() {
    let filter = codegraph::graph::Filter {
        include_uncertain: false,
        ..Default::default()
    };
    let edge = codegraph::model::Edge {
        evidence: Vec::new(),
        from: 0,
        to: 1,
        kind: "VIR".into(),
        certainty: "dispatch".into(),
        count: 1,
        provenance: String::new(),
    };
    assert!(filter.accepts_edge(&edge));
}

#[test]
fn virtual_and_other_uncertain_calls_have_independent_filters() {
    use codegraph::{graph::Filter, model::Edge};
    for virtual_calls in [false, true] {
        for uncertain in [false, true] {
            let filter = Filter {
                include_virtual_calls: virtual_calls,
                include_uncertain: uncertain,
                ..Default::default()
            };
            for (certainty, visible) in [
                ("direct", true),
                ("proven-direct", true),
                ("dispatch", virtual_calls),
                ("framework-possible", uncertain),
                ("analyzer-reported", uncertain),
            ] {
                let mut edge = Edge {
                    evidence: Vec::new(),
                    from: 0,
                    to: 1,
                    kind: "call".into(),
                    certainty: certainty.into(),
                    count: 1,
                    provenance: String::new(),
                };
                assert_eq!(
                    filter.accepts_edge(&edge),
                    visible,
                    "{filter:?} {certainty}"
                );
                edge.kind = "reference".into();
                assert_eq!(filter.accepts_edge(&edge), visible);
                let no_references = Filter {
                    include_references: false,
                    ..filter.clone()
                };
                assert!(!no_references.accepts_edge(&edge));
            }
        }
    }
    assert!(
        relationship_is_uncertain("dispatch"),
        "keep dashed rendering"
    );
}

#[test]
fn old_saved_filters_enable_virtual_calls_and_explicit_false_round_trips() {
    use codegraph::graph::Filter;
    let old = serde_json::json!({
        "include_tests": false,
        "include_generated": false,
        "include_unavailable": false,
        "include_uncertain": false,
        "include_references": true
    });
    let mut filter: Filter = serde_json::from_value(old).unwrap();
    assert!(filter.include_virtual_calls);
    assert!(!filter.include_uncertain);
    filter.include_virtual_calls = false;
    let restored: Filter = serde_json::from_str(&serde_json::to_string(&filter).unwrap()).unwrap();
    assert_eq!(restored, filter);
    assert!(!restored.include_virtual_calls);
}

#[test]
fn references_and_certainty_have_one_shared_policy() {
    for kind in [
        "methodgroup",
        "reference",
        "callback",
        "_LM",
        "_BSM",
        "react-event",
        "react-prop",
    ] {
        assert!(relationship_is_reference(kind), "{kind}");
    }
    assert!(!relationship_is_reference("call"));
    assert!(!relationship_is_reference("react-render"));
    assert!(!relationship_is_uncertain("direct"));
    assert!(!relationship_is_uncertain("proven-direct"));
    assert!(relationship_is_uncertain("framework-possible"));
}
