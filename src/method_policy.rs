//! Structural graph pruning. Language adapters own all exclusion decisions.
use crate::model::GraphData;
use anyhow::Result;
use std::collections::HashSet;

/// Persisted generations without this policy must be rebuilt from source.
pub const VERSION: u32 = 1;

/// Remove identities and incident relationships without synthesizing shortcuts.
/// Validate before mutation so malformed analyzer output cannot be hidden by filtering.
pub fn prune(graph: &mut GraphData, excluded: &HashSet<String>) -> Result<()> {
    graph.validate()?;
    let mut remap = Vec::with_capacity(graph.nodes.len());
    let mut next = 0;
    for node in &graph.nodes {
        let index = (!excluded.contains(&node.id)).then_some(next);
        next += u32::from(index.is_some());
        remap.push(index);
    }
    graph.nodes.retain(|node| !excluded.contains(&node.id));
    graph.edges.retain_mut(|edge| {
        let Some((from, to)) = remap[edge.from as usize].zip(remap[edge.to as usize]) else {
            return false;
        };
        edge.from = from;
        edge.to = to;
        true
    });
    if let Some(entries) = &mut graph.entry_points {
        for list in [
            &mut entries.candidates,
            &mut entries.application_starts,
            &mut entries.framework_entries,
        ] {
            list.retain(|id| !excluded.contains(id));
        }
    }
    graph.validate()
}
