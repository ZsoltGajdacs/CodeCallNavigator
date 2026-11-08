use codegraph::{
    graph::{Direction, Filter, GraphIndex, MultiNeighborhood, Neighborhood},
    model::{Edge, GraphData, Node},
};
use std::sync::Arc;

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
fn multi_root_traversal_deduplicates_roots_and_shared_descendants() {
    let graph = GraphIndex::new(Arc::new(sample())).unwrap();
    let filter = Filter::default();
    let slice = graph.multi_neighborhood(&MultiNeighborhood {
        roots: &[2, 0, 0, 3, 999],
        direction: Direction::Outgoing,
        depth: 1,
        filter: &filter,
    });
    assert_eq!(slice.nodes, vec![0, 2, 1]);
    assert_eq!(slice.edges, vec![0, 1, 2]);
    assert_eq!(slice.omitted_nodes, 0);
    let single = graph.multi_neighborhood(&MultiNeighborhood {
        roots: &[0],
        direction: Direction::Outgoing,
        depth: 2,
        filter: &filter,
    });
    assert_eq!(
        single.nodes,
        graph
            .neighborhood(&Neighborhood {
                node: 0,
                direction: Direction::Outgoing,
                depth: 2,
                filter: &filter,
            })
            .nodes
    );
    assert!(
        graph
            .multi_neighborhood(&MultiNeighborhood {
                roots: &[],
                direction: Direction::Incoming,
                depth: 1,
                filter: &filter
            })
            .nodes
            .is_empty()
    );
}

#[test]
fn multi_root_budget_keeps_roots_before_descendants_in_stable_identity_order() {
    let mut data = sample();
    data.nodes = (0..1602)
        .map(|index| Node {
            id: format!("method-{index:04}"),
            is_test: false,
            ..data.nodes[0].clone()
        })
        .collect();
    data.edges = (1..1601)
        .map(|to| Edge {
            from: 0,
            to,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
        })
        .collect();
    let graph = GraphIndex::new(Arc::new(data)).unwrap();
    let slice = graph.multi_neighborhood(&MultiNeighborhood {
        roots: &[1601, 0],
        direction: Direction::Outgoing,
        depth: 1,
        filter: &Filter::default(),
    });
    assert_eq!(&slice.nodes[..3], &[0, 1601, 1]);
    assert_eq!(
        (slice.nodes.len(), slice.omitted_nodes, slice.omitted_edges),
        (1500, 102, 102)
    );
}
