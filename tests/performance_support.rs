//! Observable equivalence checks for job-local traversal and generation-local summaries.
use codecallnavigator::{
    fingerprint,
    graph::{Filter, GraphIndex},
    model::{Edge, GraphData, Node},
    store::{SqliteStore, SymbolSearch},
};
use std::{
    collections::HashSet,
    fs,
    sync::{Arc, atomic::AtomicBool},
};

fn graph() -> GraphData {
    let nodes = (0..5)
        .map(|index| Node {
            id: index.to_string(),
            label: "shared_method".into(),
            is_test: index == 1,
            is_generated: index == 2,
            body_unavailable: index == 3,
            ..Default::default()
        })
        .collect();
    let edges = [
        (0, 1, "call", "direct"),
        (1, 0, "call", "direct"),
        (0, 2, "call", "possible"),
        (2, 0, "call", "direct"),
        (0, 3, "reference", "direct"),
        (3, 0, "call", "direct"),
        (4, 4, "call", "direct"),
        (4, 0, "call", "dispatch"),
    ]
    .into_iter()
    .map(|(from, to, kind, certainty)| Edge {
        evidence: Vec::new(),
        from,
        to,
        kind: kind.into(),
        certainty: certainty.into(),
        count: 1,
        provenance: "fixture".into(),
    })
    .collect();
    GraphData {
        nodes,
        edges,
        ..Default::default()
    }
}

fn filter(bits: u8) -> Filter {
    Filter {
        include_tests: bits & 1 != 0,
        include_generated: bits & 2 != 0,
        include_unavailable: bits & 4 != 0,
        include_uncertain: bits & 8 != 0,
        include_references: bits & 16 != 0,
        include_virtual_calls: bits & 32 != 0,
    }
}

fn reference_cycles(data: &GraphData, filter: &Filter) -> Vec<bool> {
    let visible_edges: Vec<_> = data
        .edges
        .iter()
        .filter(|edge| {
            filter.accepts_edge(edge)
                && filter.accepts_node(&data.nodes[edge.from as usize])
                && filter.accepts_node(&data.nodes[edge.to as usize])
        })
        .collect();
    (0..data.nodes.len() as u32)
        .map(|origin| {
            let mut pending: Vec<_> = visible_edges
                .iter()
                .filter(|edge| edge.from == origin)
                .map(|edge| edge.to)
                .collect();
            let mut visited = HashSet::new();
            while let Some(node) = pending.pop() {
                if node == origin {
                    return true;
                }
                if visited.insert(node) {
                    pending.extend(
                        visible_edges
                            .iter()
                            .filter(|edge| edge.from == node)
                            .map(|edge| edge.to),
                    );
                }
            }
            false
        })
        .collect()
}

#[test]
fn summaries_preserve_all_filter_combinations_and_do_not_cross_generations() {
    let data = Arc::new(graph());
    let index = GraphIndex::new(data.clone()).unwrap();
    let first = index.filter_summary(&filter(0));
    assert!(Arc::ptr_eq(&first, &index.filter_summary(&filter(0))));
    for bits in 0..64 {
        let filter = filter(bits);
        let summary = index.filter_summary(&filter);
        assert_eq!(
            summary.cyclic,
            reference_cycles(&data, &filter),
            "filter {bits}"
        );
        assert_eq!(
            summary.hidden_nodes,
            data.nodes
                .iter()
                .filter(|node| !filter.accepts_node(node))
                .count()
        );
        assert_eq!(
            summary.hidden_edges,
            data.edges
                .iter()
                .filter(|edge| !filter.accepts_edge(edge)
                    || !filter.accepts_node(&data.nodes[edge.from as usize])
                    || !filter.accepts_node(&data.nodes[edge.to as usize]))
                .count()
        );
    }
    assert!(
        !Arc::ptr_eq(&first, &index.filter_summary(&filter(0))),
        "unbounded cache retained every filter"
    );
    let mut next = graph();
    next.edges.clear();
    let next = GraphIndex::new(Arc::new(next)).unwrap();
    assert!(
        next.filter_summary(&filter(63))
            .cyclic
            .iter()
            .all(|cyclic| !cyclic)
    );
    assert!(
        index
            .filter_summary(&filter(63))
            .cyclic
            .iter()
            .all(|cyclic| *cyclic)
    );
}

#[test]
fn overlapping_snapshot_roots_preserve_explicit_generated_inputs_and_same_time_edits() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("src/sub/App.java");
    let output = directory.path().join("custom-output");
    let generated = output.join("generated");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::create_dir_all(&generated).unwrap();
    fs::write(&source, "class One {}").unwrap();
    let generated_source = generated.join("Generated.java");
    fs::write(&generated_source, "class Generated {}").unwrap();
    let roots = vec![
        directory.path().into(),
        source.parent().unwrap().into(),
        generated.clone(),
        source.parent().unwrap().into(),
    ];
    let paths = fingerprint::source_paths_excluding(&roots, std::slice::from_ref(&output)).unwrap();
    let expected = fingerprint::source_paths_excluding(
        &[directory.path().into(), generated],
        std::slice::from_ref(&output),
    )
    .unwrap();
    assert_eq!(paths, expected);
    assert!(paths.contains(&generated_source));
    let snapshot = fingerprint::capture_excluding(paths, &[output]).unwrap();
    let modified = fs::metadata(&source).unwrap().modified().unwrap();
    fs::write(&source, "class Two {}").unwrap();
    fs::File::open(source)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    assert!(fingerprint::is_stale(&snapshot).unwrap());
}

fn search(
    store: &SqliteStore,
    generation: &str,
    filter: &Filter,
    window: std::ops::Range<usize>,
) -> Vec<u32> {
    store
        .search_generation(
            generation,
            &SymbolSearch {
                prefix: "shared",
                filter,
                offset: window.start,
                limit: window.len(),
            },
        )
        .unwrap()
}

#[test]
fn scalar_and_legacy_policy_paths_preserve_search_order_paging_and_filters() {
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(directory.path()).unwrap();
    let generation = store.publish(&graph(), &AtomicBool::new(false)).unwrap();
    let all = filter(63);
    let full = search(&store, &generation, &all, 0..10);
    assert_eq!(full.len(), 5);
    let pages: Vec<_> = (0..5)
        .flat_map(|offset| search(&store, &generation, &all, offset..offset + 1))
        .collect();
    assert_eq!(pages, full);
    assert_eq!(search(&store, &generation, &filter(0), 0..10), [0, 4]);
    let connection = rusqlite::Connection::open(directory.path().join(&generation)).unwrap();
    connection
        .execute("DELETE FROM metadata WHERE key='method_policy'", [])
        .unwrap();
    assert_eq!(search(&store, &generation, &all, 0..10), full);
    assert_eq!(
        store.load_generation(&generation).unwrap().nodes,
        graph().nodes
    );
    connection
        .execute("INSERT INTO metadata VALUES ('method_policy','0')", [])
        .unwrap();
    assert!(store.load_generation(&generation).is_err());
    assert!(
        store
            .search_generation(
                &generation,
                &SymbolSearch {
                    prefix: "shared",
                    filter: &all,
                    offset: 0,
                    limit: 10
                }
            )
            .is_err()
    );
}
