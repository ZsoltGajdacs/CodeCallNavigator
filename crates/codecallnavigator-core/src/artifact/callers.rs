//! Incoming invocation edges shared by snapshot comparison and impact preview.
use crate::model::{Edge, GraphData, RelationshipKind, TargetResolution};

pub(super) fn possible(edge: &Edge) -> bool {
    edge.classification().resolution != TargetResolution::Direct
}

pub(super) fn incoming(graph: &GraphData) -> Vec<Vec<usize>> {
    let mut incoming = vec![Vec::new(); graph.nodes.len()];
    for (index, edge) in graph.edges.iter().enumerate() {
        if edge.classification().kind == RelationshipKind::Call {
            incoming[edge.to as usize].push(index);
        }
    }
    for edges in &mut incoming {
        edges.sort_by_key(|&index| {
            let edge = &graph.edges[index];
            (
                &graph.nodes[edge.from as usize].id,
                possible(edge),
                &edge.kind,
                &edge.certainty,
                &edge.provenance,
            )
        });
    }
    incoming
}
