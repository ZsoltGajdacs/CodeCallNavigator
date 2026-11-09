//! Combine declarations while retaining only caller-owned context relationships.
use crate::model::{EntryPoints, GraphData};
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};

pub(super) fn append(graph: &mut GraphData, part: GraphData) -> Result<()> {
    let mut indices: HashMap<_, _> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index as u32))
        .collect();
    let mut remap = Vec::new();
    for node in part.nodes {
        let index = if let Some(&index) = indices.get(&node.id) {
            merge_declaration(&mut graph.nodes[index as usize], node)?;
            index
        } else {
            let index = u32::try_from(graph.nodes.len())?;
            indices.insert(node.id.clone(), index);
            graph.nodes.push(node);
            index
        };
        remap.push(index);
    }
    for mut edge in part.edges {
        edge.from = *remap
            .get(edge.from as usize)
            .context("Missing Java caller")?;
        edge.to = *remap.get(edge.to as usize).context("Missing Java target")?;
        graph.edges.push(edge);
    }
    let entries = graph.entry_points.get_or_insert_with(EntryPoints::default);
    if let Some(part) = part.entry_points {
        entries.application_starts.extend(part.application_starts);
    }
    graph.diagnostics.extend(part.diagnostics);
    Ok(())
}

fn merge_declaration(old: &mut crate::model::Node, node: crate::model::Node) -> Result<()> {
    if old.body_unavailable && !node.body_unavailable {
        *old = node;
    } else if !node.body_unavailable && !old.body_unavailable {
        ensure!(
            serde_json::to_value(&node)? == serde_json::to_value(&*old)?,
            "Java declaration changed between contexts: {}",
            node.id
        );
    }
    Ok(())
}

pub(super) fn finalize_entries(graph: &mut GraphData) {
    let called: HashSet<_> = graph.edges.iter().map(|edge| edge.to).collect();
    let entries = graph.entry_points.get_or_insert_with(EntryPoints::default);
    entries.application_starts.sort();
    entries.application_starts.dedup();
    entries.candidates = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            !node.body_unavailable
                && !node.signature.contains(":<clinit>()")
                && !called.contains(&(*index as u32))
        })
        .map(|(_, node)| node.id.clone())
        .collect();
    entries.candidates.sort();
}

pub(super) fn finish(graph: &mut GraphData) -> Result<()> {
    finalize_entries(graph);
    graph.diagnostics.push("Java combined declarations use each caller's owning compile context. Paths may cross incompatible consumer contexts and do not describe a single runtime classpath.".into());
    graph.validate()
}
