//! Read the class graph from the captured comparison before preparing its layout.
use super::{BoundComparison, Header, client::Result};
use codecallnavigator::{
    artifact::{comparison_changes::ClassGraph, verification::InputStatus},
    query::Query,
};
use serde::Deserialize;
use std::sync::atomic::AtomicBool;

#[derive(Deserialize)]
struct Reply {
    #[serde(flatten)]
    header: Header,
    graph: ClassGraph,
}

pub(crate) struct ClassDetail {
    pub graph: ClassGraph,
    pub freshness: Option<InputStatus>,
    pub maximum_depth: u32,
}

impl BoundComparison {
    pub fn class_graph(&self, id: &str, depth: u32, cancel: &AtomicBool) -> Result<ClassDetail> {
        let maximum_depth = self
            .class_flow_depth(cancel)?
            .ok_or_else(|| anyhow::anyhow!("Class flow comparison is unavailable with this CLI"))?
            .min(self.class_source_depth(cancel)?.unwrap_or(1));
        if !(1..=maximum_depth).contains(&depth) {
            return Err(anyhow::anyhow!(
                "The configured CLI supports class diagram depths up to {maximum_depth}"
            )
            .into());
        }
        let reply = self.bound_reply(
            |references, expected| Query::ClassFlowComparison {
                base: references.base.clone(),
                head: references.head.clone(),
                base_snapshot: references.base_snapshot.clone(),
                head_snapshot: references.head_snapshot.clone(),
                expected_comparison: expected.into(),
                id: id.into(),
                depth,
            },
            cancel,
            |reply: &Reply| &reply.header,
        )?;
        let graph = reply.value.graph;
        if graph.root != id || !graph.nodes.iter().any(|node| node.id == id) {
            return Err(anyhow::anyhow!("CLI returned a class graph for another selection").into());
        }
        Ok(ClassDetail {
            graph,
            freshness: reply.freshness,
            maximum_depth,
        })
    }
}
