//! Structural graph pruning. Language adapters own all exclusion decisions.
use crate::model::GraphData;
use anyhow::Result;
use std::collections::HashSet;

/// Persisted generations without this policy must be rebuilt from source.
pub const VERSION: u32 = 3;

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
        for evidence in &mut edge.evidence {
            evidence
                .supporting_symbols
                .retain(|id| !excluded.contains(id));
        }
        true
    });
    prune_boundaries(graph, excluded);
    if let Some(entries) = &mut graph.entry_points {
        for list in [
            &mut entries.candidates,
            &mut entries.application_starts,
            &mut entries.framework_entries,
        ] {
            list.retain(|id| !excluded.contains(id));
        }
    }
    if let Some(inventory) = &mut graph.flow_inventory {
        let symbols = graph.nodes.iter().map(|node| node.id.as_str()).collect();
        inventory.retain_symbols(&symbols);
    }
    graph.validate()
}

fn prune_boundaries(graph: &mut GraphData, excluded: &HashSet<String>) {
    let Some(inventory) = &mut graph.boundaries else {
        return;
    };
    inventory
        .uses
        .retain(|usage| !excluded.contains(&usage.symbol));
    for route in &mut inventory.routes {
        if route
            .symbol
            .as_ref()
            .is_some_and(|id| excluded.contains(id))
        {
            route.symbol = None;
            route
                .restrictions
                .push("target excluded by declaration policy".into());
        }
    }
}
