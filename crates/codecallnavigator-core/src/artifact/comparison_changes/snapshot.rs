use super::{
    class::{Class, simple_name},
    source::Side,
};
use crate::{
    architecture::FileFacts,
    artifact::flow::View,
    flow::FunctionFlowIR,
    model::{GraphData, Node, RelationshipKind, Visibility},
};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialOrd, Ord, PartialEq, Eq)]
pub(super) struct FileId<'a>(pub &'a str, pub &'a str, pub &'a str);

impl<'a> FileId<'a> {
    pub fn of_node(node: &'a Node) -> Self {
        Self(&node.build, &node.project, &node.file)
    }

    fn of_file(file: &'a FileFacts) -> Self {
        Self(&file.build, &file.project, &file.file)
    }
}

type Call<'a> = (&'a str, &'a str, &'a str, u32, &'a str);

pub(super) struct Snapshot<'a> {
    pub graph: &'a GraphData,
    pub files: BTreeMap<FileId<'a>, usize>,
    pub file_nodes: BTreeMap<FileId<'a>, Vec<usize>>,
    pub symbols: BTreeMap<&'a str, usize>,
    pub classes: BTreeMap<Class, Vec<usize>>,
    pub facts: &'a [FileFacts],
    pub flows: View<'a>,
    pub functions: BTreeMap<&'a str, &'a FunctionFlowIR>,
    pub calls: BTreeMap<&'a str, Vec<Call<'a>>>,
    missing_source: &'a BTreeSet<usize>,
}

impl<'a> Snapshot<'a> {
    pub fn new(graph: &'a GraphData, missing_source: &'a BTreeSet<usize>) -> Result<Self> {
        let facts = graph
            .architecture
            .as_ref()
            .map_or(&[][..], |facts| facts.files.as_slice());
        let mut snapshot = Self {
            graph,
            files: facts
                .iter()
                .enumerate()
                .map(|(index, file)| (FileId::of_file(file), index))
                .collect(),
            file_nodes: BTreeMap::new(),
            symbols: BTreeMap::new(),
            classes: BTreeMap::new(),
            facts,
            flows: View::from_parts(graph, graph.flow_inventory.as_ref())?,
            functions: graph
                .flow_inventory
                .iter()
                .flat_map(|inventory| &inventory.functions)
                .map(|function| (function.symbol.as_str(), function))
                .collect(),
            calls: call_index(graph),
            missing_source,
        };
        for (index, node) in graph.nodes.iter().enumerate() {
            snapshot.symbols.insert(node.id.as_str(), index);
            snapshot
                .file_nodes
                .entry(FileId::of_node(node))
                .or_default()
                .push(index);
            if let Some(class) = Class::from_node(node) {
                snapshot.classes.entry(class).or_default().push(index);
            }
        }
        Ok(snapshot)
    }

    pub fn file(&self, id: FileId<'_>) -> Option<&'a FileFacts> {
        self.files.get(&id).map(|index| &self.facts[*index])
    }

    pub fn source(&self, id: FileId<'_>) -> Side {
        match self.files.get(&id) {
            Some(index) if !self.missing_source.contains(index) => Side::Recorded(*index),
            Some(_) => Side::Unavailable,
            None if self.file_nodes.contains_key(&id) || self.graph.architecture.is_none() => {
                Side::Unavailable
            }
            None => Side::Missing,
        }
    }

    pub fn text(&self, id: FileId<'_>) -> Option<&'a str> {
        match self.source(id) {
            Side::Recorded(index) => Some(self.facts[index].content.as_str()),
            Side::Missing => Some(""),
            Side::Unavailable => None,
        }
    }

    pub fn node(&self, id: &str) -> Option<&'a Node> {
        self.symbols.get(id).map(|index| &self.graph.nodes[*index])
    }

    pub fn flow_source(&self, id: &str) -> Option<(&str, crate::flow::SourceSpan)> {
        let node = self.node(id)?;
        if !matches!(self.source(FileId::of_node(node)), Side::Recorded(_)) {
            return None;
        }
        let (text, span) = self.flows.source_range(id)?;
        (text.len() as u64 <= crate::flow::MAX_SOURCE_BYTES).then_some((text, span))
    }

    pub fn has_class(&self, class: &Class) -> bool {
        self.classes.contains_key(class)
            || self.facts.iter().any(|file| {
                self.single_class(FileId::of_file(file))
                    .is_some_and(|declaration| declaration.can_confirm(class))
            })
    }

    pub fn class_visibility(&self, class: &Class) -> Option<Visibility> {
        class.visibility(self.graph, self.classes.get(class)?.as_slice())
    }

    pub fn single_class(&self, id: FileId<'_>) -> Option<Class> {
        let facts = self.file(id)?;
        let declared = unique_declaration(facts)?;
        let known: BTreeSet<_> = self
            .file_nodes
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|index| Class::from_node(&self.graph.nodes[*index]))
            .collect();
        if known.len() > 1 {
            return None;
        }
        if let Some(key) = known.into_iter().next() {
            return (key.name() == simple_name(declared)).then_some(key);
        }
        Some(Class::from_declaration(facts, declared))
    }
}

fn unique_declaration(facts: &FileFacts) -> Option<&str> {
    // Adapters may emit short and qualified names for one declaration. More than
    // one qualified owner remains ambiguous even when short names collide.
    let qualified: BTreeSet<_> = facts
        .names
        .iter()
        .filter(|name| name.contains('.'))
        .collect();
    let first = facts.names.iter().max_by_key(|name| name.len())?;
    (!facts.kinds.is_empty()
        && qualified.len() <= 1
        && facts
            .names
            .iter()
            .all(|name| simple_name(name) == simple_name(first)))
    .then_some(first)
}

pub(super) fn is_method(node: &Node) -> bool {
    !matches!(
        node.kind.as_str(),
        "field" | "fieldinit" | "initializer" | "inherited-reference" | "delegate" | "module"
    )
}

fn call_index(graph: &GraphData) -> BTreeMap<&str, Vec<Call<'_>>> {
    let mut by_caller: BTreeMap<&str, Vec<Call<'_>>> = BTreeMap::new();
    for edge in graph
        .edges
        .iter()
        .filter(|edge| edge.classification().kind == RelationshipKind::Call)
    {
        let from = &graph.nodes[edge.from as usize].id;
        let to = &graph.nodes[edge.to as usize].id;
        by_caller.entry(from).or_default().push((
            to,
            &edge.kind,
            &edge.certainty,
            edge.count,
            &edge.provenance,
        ));
    }
    by_caller
        .values_mut()
        .for_each(|calls| calls.sort_unstable());
    by_caller
}
