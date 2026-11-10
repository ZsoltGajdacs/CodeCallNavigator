//! Accumulate selected targets without rebuilding identity indexes for each target.
use crate::model::{Edge, EntryPoints, GraphData, Node, RelationshipEvidence};
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};

type EdgeKey = (
    u32,
    u32,
    String,
    String,
    String,
    u32,
    Vec<RelationshipEvidence>,
);

pub(super) struct GraphComposition {
    graph: GraphData,
    identities: HashMap<String, u32>,
    shared_edges: Option<HashSet<EdgeKey>>,
}

impl Default for GraphComposition {
    fn default() -> Self {
        Self {
            graph: GraphData {
                entry_points: Some(EntryPoints::default()),
                architecture: super::architecture::empty(),
                ..Default::default()
            },
            identities: HashMap::new(),
            shared_edges: None,
        }
    }
}

impl GraphComposition {
    pub(super) fn append(&mut self, mut part: GraphData) -> Result<()> {
        let shared = part.nodes.iter().all(|node| node.language == "typescript");
        let remap = self.append_nodes(std::mem::take(&mut part.nodes))?;
        self.append_edges(std::mem::take(&mut part.edges), &remap, shared)?;
        self.graph.entry_points = merge_entries(self.graph.entry_points.take(), part.entry_points);
        super::architecture::merge(&mut self.graph.architecture, part.architecture)?;
        crate::flow::merge_inventory(&mut self.graph.flow_inventory, part.flow_inventory)?;
        self.graph.diagnostics.extend(part.diagnostics);
        self.graph.export_identities.extend(part.export_identities);
        self.graph.roots.extend(part.roots);
        self.graph.fingerprints.extend(part.fingerprints);
        Ok(())
    }

    fn append_nodes(&mut self, nodes: Vec<Node>) -> Result<Vec<u32>> {
        let mut remap = Vec::with_capacity(nodes.len());
        for node in nodes {
            let index = if let Some(&index) = self.identities.get(&node.id) {
                ensure!(
                    node.language == "typescript" && node == self.graph.nodes[index as usize],
                    "Duplicate graph identity: {}",
                    node.id
                );
                index
            } else {
                let index = u32::try_from(self.graph.nodes.len())?;
                self.identities.insert(node.id.clone(), index);
                self.graph.nodes.push(node);
                index
            };
            remap.push(index);
        }
        Ok(remap)
    }

    fn append_edges(&mut self, edges: Vec<Edge>, remap: &[u32], shared: bool) -> Result<()> {
        if shared && self.shared_edges.is_none() {
            self.shared_edges = Some(self.graph.edges.iter().map(edge_key).collect());
        }
        for mut edge in edges {
            edge.from = *remap
                .get(edge.from as usize)
                .context("Adapter emitted missing caller")?;
            edge.to = *remap
                .get(edge.to as usize)
                .context("Adapter emitted missing target")?;
            let fresh = self
                .shared_edges
                .as_mut()
                .is_none_or(|existing| existing.insert(edge_key(&edge)));
            if !shared || fresh {
                self.graph.edges.push(edge);
            }
        }
        Ok(())
    }

    pub(super) fn finish(self) -> GraphData {
        self.graph
    }
}

fn edge_key(edge: &Edge) -> EdgeKey {
    let mut evidence = edge.evidence.clone();
    for item in &mut evidence {
        item.supporting_symbols.sort();
        item.supporting_symbols.dedup();
    }
    evidence.sort();
    evidence.dedup();
    (
        edge.from,
        edge.to,
        edge.kind.clone(),
        edge.certainty.clone(),
        edge.provenance.clone(),
        edge.count,
        evidence,
    )
}

fn merge_entries(old: Option<EntryPoints>, new: Option<EntryPoints>) -> Option<EntryPoints> {
    let (Some(mut entries), Some(part)) = (old, new) else {
        return None;
    };
    entries.framework_entries.extend(part.framework_entries);
    entries.candidates.extend(part.candidates);
    entries.application_starts.extend(part.application_starts);
    for values in [
        &mut entries.framework_entries,
        &mut entries.candidates,
        &mut entries.application_starts,
    ] {
        values.sort();
        values.dedup();
    }
    Some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part() -> GraphData {
        GraphData {
            nodes: ["caller", "target"]
                .into_iter()
                .map(|id| Node {
                    id: id.into(),
                    language: "typescript".into(),
                    label: id.into(),
                    ..Default::default()
                })
                .collect(),
            edges: vec![Edge {
                evidence: Vec::new(),
                from: 0,
                to: 1,
                kind: "call".into(),
                certainty: "direct".into(),
                provenance: "compiler".into(),
                count: 2,
            }],
            entry_points: Some(EntryPoints {
                candidates: vec!["caller".into()],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn shared_referenced_targets_preserve_edge_counts_and_provenance() {
        let mut composition = GraphComposition::default();
        composition.append(part()).unwrap();
        composition.append(part()).unwrap();
        let mut changed = part();
        changed.edges[0].provenance = "framework".into();
        composition.append(changed).unwrap();
        let graph = composition.finish();
        graph.validate().unwrap();
        assert_eq!(graph.nodes, part().nodes);
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.edges.iter().all(|edge| edge.count == 2));
        assert_eq!(graph.entry_points.unwrap().candidates, ["caller"]);
    }

    #[test]
    fn inconsistent_shared_declarations_and_non_typescript_duplicates_are_rejected() {
        let mut composition = GraphComposition::default();
        composition.append(part()).unwrap();
        let mut changed = part();
        changed.nodes[0].is_generated = true;
        assert!(composition.append(changed).is_err());
        let mut changed = part();
        changed.nodes[0].language = "java".into();
        assert!(composition.append(changed).is_err());
    }

    #[test]
    fn shared_edges_preserve_distinct_evidence_and_native_supporting_identities() {
        use crate::model::SourceSite;
        let mut first = part();
        first.edges[0].evidence = vec![RelationshipEvidence {
            rule: "react.render".into(),
            version: 1,
            semantics: "possible-invocation".into(),
            invocation: SourceSite {
                file: "/fixture.ts".into(),
                line: 1,
                column: None,
                span: None,
            },
            registration: None,
            supporting_symbols: vec!["caller".into(), "target".into()],
            channel: None,
        }];
        let mut reordered = first.clone();
        reordered.nodes.swap(0, 1);
        reordered.edges[0].from = 1;
        reordered.edges[0].to = 0;
        reordered.edges[0].evidence[0].supporting_symbols.reverse();
        let mut distinct = first.clone();
        distinct.edges[0].evidence[0].invocation.line = 2;
        let mut composition = GraphComposition::default();
        composition.append(first).unwrap();
        composition.append(reordered).unwrap();
        composition.append(distinct).unwrap();
        let graph = composition.finish();
        graph.validate().unwrap();
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.edges.iter().all(|edge| edge.count == 2));
        assert!(graph.edges.iter().all(|edge| {
            edge.evidence[0]
                .supporting_symbols
                .contains(&"target".into())
        }));
    }
}
