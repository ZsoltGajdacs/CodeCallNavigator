use codegraph::{
    graph::{Direction, Filter, GraphIndex, Neighborhood},
    model::{Edge, GraphData, Node},
    store::{SqliteStore, SymbolSearch},
};
use std::sync::{Arc, atomic::AtomicBool};

fn sample() -> GraphData {
    let nodes = (0..4)
        .map(|i| Node {
            id: format!("build/project/{i}"),
            project: "project".into(),
            build: "build".into(),
            language: "csharp".into(),
            kind: "method".into(),
            label: format!("Method{i}"),
            signature: format!("Example.Method{i}()"),
            namespace: "Example".into(),
            container: "Example".into(),
            file: "example.cs".into(),
            line: i + 1,
            is_test: i == 3,
            is_generated: false,
            body_unavailable: false,
        })
        .collect();
    let edges = [(0, 1), (1, 2), (2, 1), (0, 3)]
        .into_iter()
        .map(|(from, to)| Edge {
            provenance: String::new(),
            from,
            to,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
        })
        .collect();
    GraphData {
        nodes,
        edges,
        ..Default::default()
    }
}

#[test]
fn directed_path_cycles_and_filters_use_the_same_graph() {
    let graph = GraphIndex::new(Arc::new(sample())).unwrap();
    let filter = Filter::default();
    assert_eq!(graph.shortest_path(0, 2, &filter), Some(vec![0, 1, 2]));
    assert_eq!(graph.shortest_path(2, 0, &filter), None);
    assert_eq!(graph.shortest_path(0, 3, &filter), None);
    assert_eq!(graph.neighbors(0, Direction::Outgoing, &filter), vec![1]);
    assert_eq!(graph.cycles(&filter), vec![vec![1, 2]]);
    assert_eq!(graph.node_index("build/project/2"), Some(2));
    let slice = graph.neighborhood(&Neighborhood {
        node: 0,
        direction: Direction::Outgoing,
        depth: 2,
        filter: &filter,
    });
    assert_eq!(slice.nodes, vec![0, 1, 2]);
    assert_eq!(slice.edges, vec![0, 1, 2]);
    assert_eq!(slice.omitted_nodes, 0);
}

#[test]
fn sqlite_roundtrip_search_session_and_failed_refresh_preserve_active_graph() {
    let temp = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(temp.path()).unwrap();
    assert!(store.load_active().unwrap().is_none());
    let data = sample();
    store.publish(&data, &AtomicBool::new(false)).unwrap();
    assert_eq!(store.load_active().unwrap().unwrap().nodes.len(), 4);
    assert_eq!(
        store
            .search(&SymbolSearch {
                prefix: "Method",
                filter: &Filter::default(),
                limit: 10,
                offset: 0
            })
            .unwrap(),
        vec![0, 1, 2]
    );
    assert_eq!(
        store
            .search(&SymbolSearch {
                prefix: "Method3",
                filter: &Filter::default(),
                limit: 10,
                offset: 0
            })
            .unwrap(),
        Vec::<u32>::new()
    );
    let session = serde_json::json!({"selection": "build/project/2", "zoom": 1.2});
    store.save_session(&session).unwrap();
    assert_eq!(store.load_session().unwrap(), Some(session));
    let mut invalid = sample();
    invalid.edges[0].to = 999;
    assert!(store.publish(&invalid, &AtomicBool::new(false)).is_err());
    assert!(
        store
            .publish(&GraphData::default(), &AtomicBool::new(false))
            .is_err()
    );
    assert!(
        store
            .publish(&GraphData::default(), &AtomicBool::new(true))
            .is_err()
    );
    assert_eq!(store.load_active().unwrap().unwrap().nodes.len(), 4);
}

#[test]
fn reference_and_uncertain_edges_remain_distinct() {
    let mut data = sample();
    data.edges[0].kind = "methodgroup".into();
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let filter = Filter {
        include_references: false,
        ..Default::default()
    };
    assert_eq!(graph.shortest_path(0, 2, &filter), None);
    assert_eq!(
        graph.shortest_path(0, 2, &Filter::default()),
        Some(vec![0, 1, 2])
    );
    let mut uncertain = sample();
    uncertain.edges[0].certainty = "dispatch".into();
    let uncertain = GraphIndex::new(Arc::new(uncertain)).unwrap();
    let filter = Filter {
        include_uncertain: false,
        ..Default::default()
    };
    assert_eq!(uncertain.shortest_path(0, 2, &filter), None);
    assert_eq!(
        uncertain.shortest_path(0, 2, &Filter::default()),
        Some(vec![0, 1, 2])
    );
}

#[test]
fn canvas_budget_keeps_omitted_nodes_reachable_by_identity() {
    let mut data = sample();
    data.nodes = (0..1_601)
        .map(|index| Node {
            id: format!("build/project/{index}"),
            label: format!("Method{index}"),
            ..data.nodes[0].clone()
        })
        .collect();
    data.edges = (1..1_601)
        .map(|to| Edge {
            provenance: String::new(),
            from: 0,
            to,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
        })
        .collect();
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let slice = graph.neighborhood(&Neighborhood {
        node: 0,
        direction: Direction::Outgoing,
        depth: 1,
        filter: &Filter::default(),
    });
    assert_eq!(slice.nodes.len(), 1_500);
    assert_eq!(slice.omitted_nodes, 101);
    assert_eq!(slice.omitted_edges, 101);
    assert_eq!(graph.node_index("build/project/1600"), Some(1_600));
    assert_eq!(
        graph.shortest_path(0, 1_600, &Filter::default()),
        Some(vec![0, 1_600])
    );
}

#[test]
fn search_pins_generation_while_a_new_analysis_is_published() {
    let temp = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(temp.path()).unwrap();
    let old = sample();
    let pinned = store.publish(&old, &AtomicBool::new(false)).unwrap();
    let mut replacement = sample();
    replacement.nodes.swap(0, 2);
    store
        .publish(&replacement, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        store
            .search_generation(
                &pinned,
                &SymbolSearch {
                    prefix: "Method0",
                    filter: &Filter::default(),
                    limit: 10,
                    offset: 0
                }
            )
            .unwrap(),
        vec![0]
    );
    assert_eq!(
        store
            .search(&SymbolSearch {
                prefix: "Method0",
                filter: &Filter::default(),
                limit: 10,
                offset: 0
            })
            .unwrap(),
        vec![2]
    );
    assert_eq!(
        store.load_generation(&pinned).unwrap().nodes[0].id,
        old.nodes[0].id
    );
    assert!(
        store
            .search_generation(
                "../catalog.sqlite3",
                &SymbolSearch {
                    prefix: "Method0",
                    filter: &Filter::default(),
                    limit: 10,
                    offset: 0
                }
            )
            .is_err()
    );
}
