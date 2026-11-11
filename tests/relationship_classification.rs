use codecallnavigator::{
    graph::{Direction, Filter, GraphIndex, Neighborhood},
    model::{Edge, GraphData, Node, RelationshipKind as Kind, TargetResolution as Resolution},
};
use std::sync::Arc;

fn edge(kind: &str, certainty: &str) -> Edge {
    Edge {
        evidence: Vec::new(),
        from: 0,
        to: 1,
        kind: kind.into(),
        certainty: certainty.into(),
        count: 1,
        provenance: String::new(),
    }
}

const KNOWN_KINDS: &[(Kind, &[&str])] = &[
    (
        Kind::Call,
        &[
            "call",
            "new",
            "construct",
            "ctorinit",
            "get",
            "set",
            "event",
            "accessor",
            "STA",
            "SPE",
            "VIR",
            "INT",
            "DYN",
            "_ACT_I",
            "_ACT_C",
            "_SPR_ACT_I",
            "_SPR_ACT_C",
            "_RIR1",
            "_CIC1",
            "_TCID1",
            "_TCWRID1",
            "_CCS_SPE",
        ],
    ),
    (
        Kind::FunctionReference,
        &[
            "methodgroup",
            "reference",
            "callback",
            "_LM",
            "react-event",
            "react-prop",
        ],
    ),
    (
        Kind::FrameworkLifecycle,
        &[
            "react-render",
            "react-effect",
            "react-cleanup",
            "react-memo",
            "react-initializer",
            "react-reducer",
            "spring-event",
            "_RIR2",
            "_CIC2",
            "_TCID2",
            "_TCWRID2",
            "_TSR",
        ],
    ),
    (Kind::HttpNavigation, &["http", "navigation"]),
    (
        Kind::Other,
        &[
            "_BSM", "_ITF", "_SCC", "_CCS", "_CCS_I", "_CCID", "_ICID", "_MA", "_MAA", "ILLEGAL",
            "field", "unknown",
        ],
    ),
];

#[test]
fn relationship_meaning_preserves_raw_facts_and_covers_known_analyzer_kinds() {
    for &(expected, kinds) in KNOWN_KINDS {
        for kind in kinds {
            let edge = edge(kind, "analyzer-reported");
            let original = serde_json::to_value(&edge).unwrap();
            assert_eq!(edge.classification().kind, expected, "{kind}");
            assert_eq!(serde_json::to_value(&edge).unwrap(), original);
        }
    }
}

#[test]
fn resolution_never_upgrades_unknown_or_reference_certainty() {
    for (certainty, expected) in [
        ("direct", Resolution::Direct),
        ("proven-direct", Resolution::Direct),
        ("dispatch", Resolution::VirtualDispatch),
        ("possible", Resolution::Possible),
        ("reference", Resolution::Other),
        ("delegate", Resolution::Other),
        ("analyzer-reported", Resolution::Other),
        ("future-certainty", Resolution::Other),
    ] {
        for kind in ["call", "methodgroup", "react-event", "http", "unknown"] {
            assert_eq!(edge(kind, certainty).classification().resolution, expected);
        }
    }
}

#[test]
fn explicit_evidence_and_provenance_obey_classification_precedence() {
    use codecallnavigator::model::{RelationshipEvidence, SourceSite};
    for semantics in ["deferred", "async-boundary"] {
        let evidence = RelationshipEvidence {
            rule: "fixture".into(),
            version: 1,
            invocation: SourceSite {
                file: "fixture.ts".into(),
                line: 1,
                column: None,
                span: None,
            },
            registration: None,
            supporting_symbols: Vec::new(),
            channel: None,
            qualifiers: Default::default(),
            semantics: semantics.into(),
        };
        for (kind, expected) in [
            ("http", Kind::HttpNavigation),
            ("reference", Kind::FunctionReference),
            ("call", Kind::FrameworkLifecycle),
            ("unknown", Kind::FrameworkLifecycle),
        ] {
            let mut edge = edge(kind, "possible");
            edge.evidence.push(evidence.clone());
            assert_eq!(edge.classification().kind, expected, "{kind}/{semantics}");
        }
    }
    let mut framework = edge("call", "possible");
    framework.provenance = "codecallnavigator.mediatr.v1".into();
    assert_eq!(framework.classification().kind, Kind::FrameworkLifecycle);
    framework.kind = "methodgroup".into();
    assert_eq!(framework.classification().kind, Kind::FunctionReference);
    framework.kind = "navigation".into();
    assert_eq!(framework.classification().kind, Kind::HttpNavigation);
}

#[test]
fn every_type_and_resolution_combination_is_independently_selectable() {
    for kind in Kind::ALL {
        for resolution in Resolution::ALL {
            let mut filter = Filter::default();
            let classification =
                codecallnavigator::model::RelationshipClassification { kind, resolution };
            assert!(filter.accepts_classification(classification));
            filter.relationship_kinds.remove(&kind);
            assert!(!filter.accepts_classification(classification));
            filter.relationship_kinds.insert(kind);
            filter.target_resolutions.remove(&resolution);
            assert!(!filter.accepts_classification(classification));
            filter.target_resolutions.insert(resolution);
            assert!(filter.accepts_classification(classification));
        }
    }
}

#[test]
fn independent_filters_control_traversal_paths_cycles_and_counts() {
    let mut reverse = edge("call", "direct");
    reverse.from = 1;
    reverse.to = 0;
    let data = GraphData {
        nodes: (0..2)
            .map(|id| Node {
                id: id.to_string(),
                ..Default::default()
            })
            .collect(),
        edges: vec![edge("methodgroup", "dispatch"), reverse],
        ..Default::default()
    };
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let mut filter = Filter::default();
    assert_eq!(graph.shortest_path(0, 1, &filter), Some(vec![0, 1]));
    assert_eq!(graph.cycles(&filter), vec![vec![0, 1]]);
    for hide_reference in [false, true] {
        if hide_reference {
            filter.relationship_kinds.remove(&Kind::FunctionReference);
        } else {
            filter
                .target_resolutions
                .remove(&Resolution::VirtualDispatch);
        }
        assert!(graph.neighbors(0, Direction::Outgoing, &filter).is_empty());
        assert!(
            graph
                .incident_edges(0, Direction::Outgoing, &filter)
                .is_empty()
        );
        assert!(graph.shortest_path(0, 1, &filter).is_none());
        assert!(graph.cycles(&filter).is_empty());
        let slice = graph.neighborhood(&Neighborhood {
            node: 0,
            direction: Direction::Outgoing,
            depth: 3,
            filter: &filter,
        });
        assert_eq!(slice.nodes, vec![0]);
        assert!(slice.edges.is_empty());
        assert_eq!(graph.filter_summary(&filter).hidden_edges, 1);
        filter = Filter::default();
    }
}

#[test]
fn legacy_filters_reset_relationship_choices_once_and_preserve_code_flags() {
    let legacy = serde_json::json!({
        "include_tests": true, "include_generated": false, "include_unavailable": true,
        "include_virtual_calls": false, "include_uncertain": false, "include_references": false
    });
    let mut filter: Filter = serde_json::from_value(legacy).unwrap();
    assert!(filter.include_tests && filter.include_unavailable);
    assert!(!filter.include_generated);
    assert_eq!(filter.relationship_kinds, Kind::ALL.into());
    assert_eq!(filter.target_resolutions, Resolution::ALL.into());
    filter.relationship_kinds.clear();
    filter.target_resolutions.clear();
    let current = serde_json::to_value(&filter).unwrap();
    assert_eq!(current["version"], 2);
    let restored: Filter = serde_json::from_value(current).unwrap();
    assert_eq!(restored, filter);
    assert!(!restored.accepts_edge(&edge("call", "direct")));
}

#[test]
fn unsupported_or_incomplete_current_filter_settings_are_rejected() {
    assert!(serde_json::from_value::<Filter>(serde_json::json!({"version": 3})).is_err());
    assert!(serde_json::from_value::<Filter>(serde_json::json!({"version": 2})).is_err());
}
