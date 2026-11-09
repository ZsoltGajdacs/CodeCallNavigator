//! Combine declarations while retaining only caller-owned context relationships.
use crate::model::{EntryPoints, GraphData};
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct Composer {
    indices: HashMap<String, u32>,
}

impl Composer {
    pub(super) fn append(&mut self, graph: &mut GraphData, part: GraphData) -> Result<()> {
        let indices = &mut self.indices;
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
        crate::analyzers::architecture::merge(&mut graph.architecture, part.architecture)?;
        graph.diagnostics.extend(part.diagnostics);
        Ok(())
    }
}

fn merge_declaration(old: &mut crate::model::Node, node: crate::model::Node) -> Result<()> {
    if old.body_unavailable && !node.body_unavailable {
        *old = node;
    } else if !node.body_unavailable && !old.body_unavailable {
        ensure!(
            node == *old,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Edge, Node};

    fn declaration(id: &str, unavailable: bool) -> Node {
        Node {
            id: id.into(),
            label: id.into(),
            body_unavailable: unavailable,
            ..Node::default()
        }
    }

    #[test]
    fn later_context_replaces_placeholder_and_remaps_relationships() {
        let mut graph = GraphData::default();
        let mut composer = Composer::default();
        composer
            .append(
                &mut graph,
                GraphData {
                    nodes: vec![declaration("caller", false), declaration("target", true)],
                    edges: vec![Edge {
                        from: 0,
                        to: 1,
                        kind: "call".into(),
                        certainty: "direct".into(),
                        count: 2,
                        provenance: String::new(),
                    }],
                    ..GraphData::default()
                },
            )
            .unwrap();
        composer
            .append(
                &mut graph,
                GraphData {
                    nodes: vec![declaration("target", false), declaration("leaf", false)],
                    edges: vec![Edge {
                        from: 0,
                        to: 1,
                        kind: "call".into(),
                        certainty: "direct".into(),
                        count: 1,
                        provenance: String::new(),
                    }],
                    ..GraphData::default()
                },
            )
            .unwrap();
        assert_eq!(graph.nodes.len(), 3);
        assert!(!graph.nodes[1].body_unavailable);
        assert_eq!(
            (graph.edges[0].from, graph.edges[0].to, graph.edges[0].count),
            (0, 1, 2)
        );
        assert_eq!((graph.edges[1].from, graph.edges[1].to), (1, 2));
        graph.validate().unwrap();
        let mut changed = declaration("target", false);
        changed.is_generated = true;
        assert!(
            composer
                .append(
                    &mut graph,
                    GraphData {
                        nodes: vec![changed],
                        ..GraphData::default()
                    }
                )
                .is_err()
        );
    }
}
