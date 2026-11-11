//! Validate known portable fields once and retain the endpoint index for hydration.
use super::{
    paths,
    portable::{PortableEdge, PortableGraph},
};
use crate::{
    architecture::ArchitectureFacts,
    boundary::BoundaryInventory,
    flow::{self, FlowInventory, FunctionFlowIR},
    model::{EntryPoints, Node, RelationshipEvidence},
};
use anyhow::{Result, ensure};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(super) fn validate(graph: &mut PortableGraph) -> Result<HashMap<String, u32>> {
    let ids = node_index(&graph.nodes)?;
    let symbols: HashSet<&str> = ids.keys().map(String::as_str).collect();
    validate_edges(&graph.edges, &ids, &symbols)?;
    validate_roots(graph)?;
    if let Some(entries) = &graph.entry_points {
        validate_entries(entries, &ids)?;
    }
    if let Some(boundaries) = &graph.boundaries {
        validate_boundaries(boundaries, &symbols)?;
    }
    if let Some(facts) = &graph.architecture {
        validate_architecture(facts)?;
    }
    if let Some(inventory) = &mut graph.flow_inventory {
        validate_flows(inventory, &ids)?;
    }
    Ok(ids)
}

fn node_index(nodes: &[Node]) -> Result<HashMap<String, u32>> {
    let mut ids = HashMap::new();
    for (index, node) in nodes.iter().enumerate() {
        ensure!(!node.id.is_empty(), "A graph symbol has no identity");
        ensure!(
            ids.insert(node.id.clone(), u32::try_from(index)?).is_none(),
            "Duplicate artifact identity: {}",
            node.id
        );
        path(&node.file)?;
        path(&node.build)?;
    }
    Ok(ids)
}

fn validate_edges(
    edges: &[PortableEdge],
    ids: &HashMap<String, u32>,
    symbols: &HashSet<&str>,
) -> Result<()> {
    ensure!(edges.len() <= u32::MAX as usize, "Too many graph edges");
    for edge in edges {
        symbol(&edge.from, ids)?;
        symbol(&edge.to, ids)?;
        ensure!(edge.count > 0, "Relationship count must be positive");
        for evidence in &edge.evidence {
            validate_evidence(evidence, symbols)?;
        }
    }
    Ok(())
}

fn validate_evidence(evidence: &RelationshipEvidence, symbols: &HashSet<&str>) -> Result<()> {
    evidence.validate(symbols)?;
    path(&evidence.invocation.file)?;
    if let Some(site) = &evidence.registration {
        path(&site.file)?;
    }
    Ok(())
}

fn validate_roots(graph: &PortableGraph) -> Result<()> {
    graph
        .roots
        .iter()
        .chain(&graph.identity_roots)
        .chain(&graph.folder_project_builds)
        .try_for_each(|root| path(root))
}

fn validate_boundaries(boundaries: &BoundaryInventory, symbols: &HashSet<&str>) -> Result<()> {
    boundaries.validate(symbols)?;
    for route in &boundaries.routes {
        path(&route.build)?;
        path(&route.site.file)?;
    }
    for usage in &boundaries.uses {
        path(&usage.build)?;
        path(&usage.site.file)?;
    }
    Ok(())
}

fn validate_architecture(facts: &ArchitectureFacts) -> Result<()> {
    ensure!(
        facts.version == crate::architecture::FACTS_VERSION,
        "Unsupported architecture declaration facts version"
    );
    for file in &facts.files {
        path(&file.file)?;
        path(&file.build)?;
    }
    Ok(())
}

fn validate_flows(inventory: &mut FlowInventory, ids: &HashMap<String, u32>) -> Result<()> {
    ensure!(
        inventory.version == flow::INVENTORY_VERSION,
        "Unsupported function flow inventory version"
    );
    for function in &mut inventory.functions {
        validate_function(function, ids)?;
    }
    Ok(())
}

fn validate_function(function: &mut FunctionFlowIR, ids: &HashMap<String, u32>) -> Result<()> {
    symbol(&function.symbol, ids)?;
    path(&function.file)?;
    path(&function.build)?;
    let mut result = Ok(());
    flow::visit_calls_mut(function, &mut |call| {
        if result.is_ok() {
            result = call
                .targets
                .iter()
                .try_for_each(|target| symbol(&target.symbol, ids));
        }
    });
    result
}

fn path(text: &str) -> Result<()> {
    paths::validate_field("file", &Value::String(text.into()))
}

fn symbol(text: &str, ids: &HashMap<String, u32>) -> Result<()> {
    ensure!(ids.contains_key(text), "Missing artifact symbol: {text}");
    Ok(())
}

fn validate_entries(entries: &EntryPoints, ids: &HashMap<String, u32>) -> Result<()> {
    for values in [
        &entries.candidates,
        &entries.framework_entries,
        &entries.application_starts,
    ] {
        let mut seen = HashSet::new();
        for value in values {
            symbol(value, ids)?;
            ensure!(
                seen.insert(value),
                "Duplicate entry point identity: {value}"
            );
        }
    }
    Ok(())
}
