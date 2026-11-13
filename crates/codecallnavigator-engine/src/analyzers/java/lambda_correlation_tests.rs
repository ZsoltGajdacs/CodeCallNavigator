//! Behavioral checks for the compiled/source evidence boundary and native bindings.
use super::{
    flow_inventory::{self, BindingContext, SourceFlow},
    lambda_binding::{self, DeclarationIndex},
    lambda_correlation::PendingCorrelations,
    source_inventory::SourceInventory,
};
use crate::{
    discovery::BuildTarget,
    model::{Edge, GraphData, Node},
};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};

const OWNER: &str = "fixture.Flow";
const IMPLEMENTATION: &str = "fixture.Flow:lambda$run$0()|void";
const PREFIX: &str = "context:";

fn source(offset: u32, key: &str) -> SourceFlow {
    serde_json::from_value(json!({
        "symbol": format!("@lambda:{OWNER}:{offset}"),
        "file": "/repo/src/Flow.java", "language": "java",
        "lambda_owner": OWNER, "export_identity": key,
        "lambda_start_line": 10, "lambda_end_line": 10
    }))
    .unwrap()
}

fn pending(
    status: &str,
    groups: &[(&str, Vec<usize>)],
    flows: &[SourceFlow],
) -> PendingCorrelations {
    let mut pending = PendingCorrelations::default();
    pending
        .correlation(&json!({"owner": OWNER, "status": status, "reason": "probe"}).to_string())
        .unwrap();
    for (implementation, sources) in groups {
        let sources: Vec<_> = sources
            .iter()
            .map(|index| flows[*index].function.symbol.clone())
            .collect();
        pending
            .binding(&json!({"implementation": implementation, "sources": sources}).to_string())
            .unwrap();
    }
    pending
}

fn inventory(status: &str, keys: &[&str], groups: &[(&str, Vec<usize>)]) -> SourceInventory {
    let mut inventory = SourceInventory::default();
    inventory.flows = keys
        .iter()
        .enumerate()
        .map(|(index, key)| source(index as u32, key))
        .collect();
    inventory.lambda_correlations = pending(status, groups, &inventory.flows)
        .validate(&inventory.flows, true)
        .unwrap();
    inventory.has_flow = true;
    inventory
}

fn graph(prefix: &str, line: u32) -> GraphData {
    GraphData {
        nodes: vec![Node {
            id: format!("{prefix}{IMPLEMENTATION}"),
            signature: "fixture.Flow:lambda$run$0() -> void".into(),
            file: "/repo/src/Flow.java".into(),
            line,
            ..Default::default()
        }],
        edges: vec![Edge {
            from: 0,
            to: 0,
            kind: "STA".into(),
            certainty: "direct".into(),
            count: 2,
            evidence: Vec::new(),
            provenance: "javac".into(),
        }],
        ..Default::default()
    }
}

#[test]
fn verified_singleton_binds_without_lines_and_preserves_existing_key_bytes() {
    let inventory = inventory("verified", &["old-key"], &[(IMPLEMENTATION, vec![0])]);
    let graph = graph(PREFIX, 0);
    let result =
        lambda_binding::resolve(&inventory, PREFIX, &DeclarationIndex::new(&graph, PREFIX));
    assert_eq!(result.flows[&0].id, graph.nodes[0].id);
    assert_eq!(
        result.export_identities[&graph.nodes[0].id],
        "context:old-key"
    );
    assert_eq!((result.shared, result.unresolved), (0, 0));
}

#[test]
fn grouped_identity_is_order_independent_and_never_creates_arbitrary_source_flow() {
    let graph = graph(PREFIX, 10);
    let nodes = DeclarationIndex::new(&graph, PREFIX);
    let mut ids = Vec::new();
    for members in [vec![0, 1], vec![1, 0]] {
        let inventory = inventory(
            "verified",
            &["key-b", "key-a"],
            &[(IMPLEMENTATION, members)],
        );
        let result = lambda_binding::resolve(&inventory, PREFIX, &nodes);
        assert!(result.flows.is_empty());
        assert_eq!((result.shared, result.unresolved), (2, 0));
        ids.push(result.export_identities[&graph.nodes[0].id].clone());
    }
    assert_eq!(ids[0], ids[1]);
    assert_eq!(
        ids[0],
        "context:{\"source_lambda_group\":[\"key-a\",\"key-b\"]}"
    );
    let partial = inventory("verified", &["key", ""], &[(IMPLEMENTATION, vec![0, 1])]);
    assert!(
        lambda_binding::resolve(&partial, PREFIX, &nodes)
            .export_identities
            .is_empty()
    );
}

#[test]
fn verified_bindings_work_without_sourcefile_and_reject_contradictory_known_files() {
    for keys in [vec!["first"], vec!["first", "second"]] {
        let members: Vec<_> = (0..keys.len()).collect();
        let inventory = inventory("verified", &keys, &[(IMPLEMENTATION, members)]);
        let mut graph = graph(PREFIX, 0);
        graph.nodes[0].file.clear();
        let result =
            lambda_binding::resolve(&inventory, PREFIX, &DeclarationIndex::new(&graph, PREFIX));
        assert_eq!(result.export_identities.len(), 1);
        assert_eq!(result.flows.len(), usize::from(keys.len() == 1));
        assert!(graph.nodes[0].file.is_empty());
        graph.nodes[0].file = "/other/Flow.java".into();
        let result =
            lambda_binding::resolve(&inventory, PREFIX, &DeclarationIndex::new(&graph, PREFIX));
        assert!(result.export_identities.is_empty());
        assert_eq!(result.unresolved, keys.len());
    }
}

#[test]
fn rejected_proof_blocks_line_fallback_while_unavailable_and_legacy_retain_it() {
    let graph = graph(PREFIX, 10);
    let nodes = DeclarationIndex::new(&graph, PREFIX);
    let rejected = inventory("rejected", &["key"], &[]);
    let unavailable = inventory("unavailable", &["key"], &[]);
    assert_eq!(
        lambda_binding::resolve(&rejected, PREFIX, &nodes).unresolved,
        1
    );
    assert_eq!(
        lambda_binding::resolve(&unavailable, PREFIX, &nodes)
            .flows
            .len(),
        1
    );
    let mut legacy = SourceInventory::default();
    legacy.flows = vec![source(0, "key")];
    assert_eq!(
        lambda_binding::resolve(&legacy, PREFIX, &nodes).flows.len(),
        1
    );
    assert!(
        lambda_binding::resolve(
            &unavailable,
            "other:",
            &DeclarationIndex::new(&graph, "other:")
        )
        .flows
        .is_empty()
    );
}

#[test]
fn incomplete_conflicting_and_cross_owner_evidence_is_rejected() {
    let flows = vec![source(0, "first"), source(1, "second")];
    for groups in [
        vec![(IMPLEMENTATION, vec![0])],
        vec![(IMPLEMENTATION, vec![0, 0, 1])],
        vec![(IMPLEMENTATION, vec![0]), (IMPLEMENTATION, vec![1])],
        vec![("other.Flow:lambda$run$0()|void", vec![0, 1])],
    ] {
        assert!(
            pending("verified", &groups, &flows)
                .validate(&flows, true)
                .is_err()
        );
    }
    assert!(
        pending("rejected", &[(IMPLEMENTATION, vec![0, 1])], &flows)
            .validate(&flows, true)
            .is_err()
    );
    let mut unknown = pending("verified", &[], &flows);
    unknown
        .binding(
            &json!({"implementation": IMPLEMENTATION, "sources": ["@lambda:fixture.Flow:999"]})
                .to_string(),
        )
        .unwrap();
    assert!(unknown.validate(&flows, true).is_err());
    let missing_owner = PendingCorrelations::default();
    assert!(missing_owner.validate(&flows, true).is_err());
    let mut cross_file = flows.clone();
    cross_file[1].function.file = "/other/Flow.java".into();
    let error = pending("verified", &[(IMPLEMENTATION, vec![0, 1])], &cross_file)
        .validate(&cross_file, true)
        .err()
        .expect("Cross-file owner must remain invalid")
        .to_string();
    for detail in [OWNER, "/repo/src/Flow.java", "/other/Flow.java"] {
        assert!(error.contains(detail), "Missing {detail} in {error}");
    }
}

#[test]
fn binding_keeps_complete_native_nodes_and_edges_unchanged_for_legacy_and_v8() {
    let build = BuildTarget {
        path: "/repo".into(),
        language: "java".into(),
        kind: "maven".into(),
        label: "fixture".into(),
    };
    let source_set = super::build::JavaSourceSet {
        project: "fixture".into(),
        application: None,
        source_set: "main".into(),
        compile_classpath: vec![],
        classpath_aliases: vec![],
        project_dir: "/repo".into(),
        is_test: false,
        source_roots: vec![],
        generated_roots: vec![],
        class_dirs: vec![],
        raw_class_dirs: None,
        build_inputs: vec![],
    };
    let prefix = super::normalize::identity_prefix(&build, &source_set);
    let inputs = [super::JavaInput {
        path: "/repo/classes".into(),
        source_set: Some(source_set),
    }];
    let graph = graph(&prefix, 10);
    let native = serde_json::to_value((&graph.nodes, &graph.edges)).unwrap();
    for v8 in [false, true] {
        let mut inventory = inventory("verified", &["key"], &[(IMPLEMENTATION, vec![0])]);
        if !v8 {
            inventory.lambda_correlations = Default::default();
        }
        let inventories = HashMap::from([(1, Arc::new(inventory))]);
        let result = flow_inventory::bind(
            BindingContext {
                build: &build,
                inputs: &inputs,
                owner: None,
            },
            &graph,
            &inventories,
        );
        assert_eq!(
            result.inventory.unwrap().functions[0].symbol,
            graph.nodes[0].id
        );
        assert_eq!(
            serde_json::to_value((&graph.nodes, &graph.edges)).unwrap(),
            native
        );
    }
}
