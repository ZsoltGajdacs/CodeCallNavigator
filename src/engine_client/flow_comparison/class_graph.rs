//! Read the class graph from the captured comparison before preparing its layout.
use super::{BoundComparison, Header, client::Result};
use codecallnavigator::{artifact::comparison_changes::ClassGraph, query::Query};
use serde::Deserialize;
use std::sync::atomic::AtomicBool;

#[derive(Deserialize)]
struct Reply {
    #[serde(flatten)]
    header: Header,
    graph: ClassGraph,
}

impl BoundComparison {
    pub fn class_graph(&self, id: &str, cancel: &AtomicBool) -> Result<ClassGraph> {
        if !self.class_flow_capability(cancel)? {
            return Err(
                anyhow::anyhow!("Class flow comparison is unavailable with this CLI").into(),
            );
        }
        let reply: Reply = self.bound_request(
            |references, expected| Query::ClassFlowComparison {
                base: references.base.clone(),
                head: references.head.clone(),
                base_snapshot: references.base_snapshot.clone(),
                head_snapshot: references.head_snapshot.clone(),
                expected_comparison: expected.into(),
                id: id.into(),
            },
            cancel,
            |reply: &Reply| &reply.header,
        )?;
        let graph = reply.graph;
        if graph.root != id || !graph.nodes.iter().any(|node| node.id == id) {
            return Err(anyhow::anyhow!("CLI returned a class graph for another selection").into());
        }
        Ok(graph)
    }
}
