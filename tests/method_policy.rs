use codegraph::{
    graph::Filter,
    method_policy::prune,
    model::{Edge, EntryPoints, GraphData, Node},
    store::{SqliteStore, SymbolSearch},
};
use std::{collections::HashSet, sync::atomic::AtomicBool};

fn graph() -> GraphData {
    GraphData {
        nodes: ["caller", "constructor", "leaf"]
            .into_iter()
            .map(|id| Node {
                id: id.into(),
                label: id.into(),
                ..Default::default()
            })
            .collect(),
        edges: [(0, 1), (1, 2), (0, 2)]
            .into_iter()
            .map(|(from, to)| Edge {
                from,
                to,
                count: 2,
                kind: "call".into(),
                certainty: "direct".into(),
                provenance: "fixture".into(),
            })
            .collect(),
        entry_points: Some(EntryPoints {
            candidates: vec!["caller".into(), "constructor".into()],
            application_starts: vec!["caller".into()],
            framework_entries: vec!["constructor".into()],
        }),
        ..Default::default()
    }
}

#[test]
fn pruning_remaps_edges_and_entries_without_reconnecting_removed_methods() {
    let mut data = graph();
    prune(&mut data, &HashSet::from(["constructor".into()])).unwrap();
    assert_eq!(
        data.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
        ["caller", "leaf"]
    );
    assert_eq!(data.edges.len(), 1);
    assert_eq!(
        (data.edges[0].from, data.edges[0].to, data.edges[0].count),
        (0, 1, 2)
    );
    assert_eq!(data.edges[0].provenance, "fixture");
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(directory.path()).unwrap();
    store.publish(&data, &AtomicBool::new(false)).unwrap();
    let all_visible = Filter {
        include_tests: true,
        include_generated: true,
        include_unavailable: true,
        ..Default::default()
    };
    assert!(
        store
            .search(&SymbolSearch {
                prefix: "constructor",
                filter: &all_visible,
                limit: 10,
                offset: 0
            })
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .search(&SymbolSearch {
                prefix: "leaf",
                filter: &all_visible,
                limit: 10,
                offset: 0
            })
            .unwrap(),
        [1]
    );
    let entries = data.entry_points.unwrap();
    assert_eq!(entries.candidates, ["caller"]);
    assert!(entries.framework_entries.is_empty());
}

#[test]
fn fully_pruned_graph_can_be_published_and_searched() {
    let mut data = graph();
    let removed = data.nodes.iter().map(|n| n.id.clone()).collect();
    prune(&mut data, &removed).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(directory.path()).unwrap();
    store.publish(&data, &AtomicBool::new(false)).unwrap();
    assert!(store.load_active().unwrap().unwrap().nodes.is_empty());
    assert!(
        store
            .search(&SymbolSearch {
                prefix: "caller",
                filter: &Filter::default(),
                limit: 10,
                offset: 0,
            })
            .unwrap()
            .is_empty()
    );
}

#[test]
fn old_cached_policy_requires_refresh_without_deleting_data() {
    let directory = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(directory.path()).unwrap();
    let generation = store.publish(&graph(), &AtomicBool::new(false)).unwrap();
    let connection = rusqlite::Connection::open(directory.path().join(&generation)).unwrap();
    // Emulate a legacy generation, which has no separate scalar policy entry.
    connection
        .execute("DELETE FROM metadata WHERE key='method_policy'", [])
        .unwrap();
    connection
        .execute(
            "UPDATE metadata SET value=json_remove(value, '$.method_policy') WHERE key='graph'",
            [],
        )
        .unwrap();
    assert!(
        store
            .load_generation(&generation)
            .unwrap_err()
            .to_string()
            .contains("Refresh")
    );
    assert!(
        store
            .search_generation(
                &generation,
                &SymbolSearch {
                    prefix: "caller",
                    filter: &Filter::default(),
                    limit: 10,
                    offset: 0,
                }
            )
            .unwrap_err()
            .to_string()
            .contains("Refresh")
    );
    assert_eq!(store.active_generation().unwrap(), Some(generation));
}

#[test]
fn pruning_does_not_hide_invalid_engine_output() {
    let mut data = graph();
    data.edges[0].to = 99;
    let excluded = data.nodes.iter().map(|node| node.id.clone()).collect();
    assert!(prune(&mut data, &excluded).is_err());
    assert_eq!(data.nodes.len(), 3);
}
