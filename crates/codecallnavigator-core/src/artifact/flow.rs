//! Shared typed access to source-derived flows from one immutable generation.
//! Live sessions borrow their resident inventory; saved revisions decode it once.
mod model;
mod ownership;
mod saved;
use super::{Artifact, SCHEMA};
use crate::{
    architecture::ArchitectureFacts,
    flow::{self, FlowInventory, FunctionFlowIR},
    model::{GraphData, Node},
};
pub use model::*;
use serde::Deserialize;
use std::{borrow::Cow, collections::BTreeMap};

pub struct View<'a> {
    nodes: Cow<'a, [Node]>,
    sources: Option<Cow<'a, ArchitectureFacts>>,
    inventory: Option<Cow<'a, FlowInventory>>,
    symbols: BTreeMap<String, Option<usize>>,
    functions: Cow<'a, InventoryIndex>,
}
/// Symbol positions owned alongside a resident inventory and released with it.
#[derive(Clone, Debug, Default)]
pub struct InventoryIndex(BTreeMap<String, Option<usize>>);

impl InventoryIndex {
    pub fn new(inventory: &FlowInventory) -> Self {
        Self(saved::index(Some(inventory)))
    }
}

/// Decode portable symbol/source ownership without copying edges or flow bodies.
/// Saved source presence is checked before analyzer-oriented defaults can erase it.
pub fn read_structure(artifact: &Artifact) -> Result<GraphData, Unavailable> {
    if artifact.schema != SCHEMA {
        return Err(Unavailable::new(
            Reason::InvalidArtifact,
            "Unsupported saved artifact schema",
        ));
    }
    let nodes = Vec::<Node>::deserialize(&artifact.graph["nodes"])
        .map_err(|error| Unavailable::new(Reason::InvalidArtifact, error.to_string()))?;
    let architecture = saved::decode_sources(&artifact.graph["architecture"])?;
    Ok(GraphData {
        nodes,
        architecture,
        ..Default::default()
    })
}

struct Facts<'a> {
    node: Symbol<'a>,
    function: &'a FunctionFlowIR,
    content: &'a str,
}
impl<'a> View<'a> {
    /// Saved-revision ownership is request-scoped; no raw JSON is kept by this view.
    pub fn new(artifact: &'a Artifact) -> Result<Self, Unavailable> {
        let structure = read_structure(artifact)?;
        let inventory = Option::<FlowInventory>::deserialize(&artifact.graph["flow_inventory"])
            .map_err(|error| Unavailable::new(Reason::InvalidFlow, error.to_string()))?;
        Self::build(
            Cow::Owned(structure.nodes),
            structure.architecture.map(Cow::Owned),
            inventory.map(Cow::Owned),
        )
    }

    /// Structure and flows must use the same path domain and generation.
    pub fn from_parts(
        graph: &'a GraphData,
        inventory: Option<&'a FlowInventory>,
    ) -> Result<Self, Unavailable> {
        Self::build(
            Cow::Borrowed(&graph.nodes),
            graph.architecture.as_ref().map(Cow::Borrowed),
            inventory.map(Cow::Borrowed),
        )
    }

    pub fn from_indexed_parts(
        graph: &'a GraphData,
        inventory: Option<&'a FlowInventory>,
        index: &'a InventoryIndex,
    ) -> Result<Self, Unavailable> {
        Ok(Self {
            nodes: Cow::Borrowed(&graph.nodes),
            sources: graph.architecture.as_ref().map(Cow::Borrowed),
            inventory: inventory.map(Cow::Borrowed),
            symbols: ownership::index(&graph.nodes)?,
            functions: Cow::Borrowed(index),
        })
    }

    fn build(
        nodes: Cow<'a, [Node]>,
        sources: Option<Cow<'a, ArchitectureFacts>>,
        inventory: Option<Cow<'a, FlowInventory>>,
    ) -> Result<Self, Unavailable> {
        let symbols = ownership::index(&nodes)?;
        let functions = Cow::Owned(InventoryIndex(saved::index(inventory.as_deref())));
        Ok(Self {
            nodes,
            sources,
            inventory,
            symbols,
            functions,
        })
    }

    pub fn select(&self, symbol: &str) -> Selection<'_> {
        match self.prepare(symbol) {
            Ok(Some(ready)) => Selection::Ready(Box::new(ready)),
            Ok(None) => Selection::Missing,
            Err(reason) => Selection::Unavailable(reason),
        }
    }

    pub fn symbols(&self) -> impl Iterator<Item = &str> + '_ {
        self.symbols.keys().map(String::as_str)
    }

    pub fn describe(&self, symbol: &str) -> Result<Option<Symbol<'_>>, Unavailable> {
        Ok(self.node(symbol)?.map(ownership::symbol))
    }

    pub fn recording(&self, symbol: &str) -> Recording<'_> {
        match self.facts(symbol) {
            Ok(Some(facts)) => Recording::Recorded(facts.node),
            Ok(None) => Recording::Missing,
            Err(error) => Recording::Unavailable(error),
        }
    }

    fn facts(&self, symbol: &str) -> Result<Option<Facts<'_>>, Unavailable> {
        let Some(raw_node) = self.node(symbol)? else {
            return Ok(None);
        };
        let node = ownership::symbol(raw_node);
        ownership::available(&node)?;
        let function = saved::function(self.inventory.as_deref(), &self.functions.0, symbol)?;
        saved::owner(function, node.owner)?;
        let content = saved::source(self.sources.as_deref(), node.owner)?;
        Ok(Some(Facts {
            node,
            function,
            content,
        }))
    }

    pub(super) fn source_range(&self, symbol: &str) -> Option<(&str, crate::flow::SourceSpan)> {
        let facts = self.facts(symbol).ok()??;
        let span = facts.function.span;
        (span.start < span.end && facts.content.get(span.start..span.end).is_some())
            .then_some((facts.content, span))
    }

    /// Clone only requested IR; source text and the resident inventory stay borrowed.
    pub fn input(&self, symbol: &str) -> Result<Option<Input<'_>>, Unavailable> {
        let Some(facts) = self.facts(symbol)? else {
            return Ok(None);
        };
        let mut function = facts.function.clone();
        self.targets(&mut function)?;
        Ok(Some(Input {
            function,
            content: facts.content,
        }))
    }

    fn prepare(&self, symbol: &str) -> Result<Option<Ready<'_>>, Unavailable> {
        let Some(Facts {
            node,
            function,
            content,
        }) = self.facts(symbol)?
        else {
            return Ok(None);
        };
        let mut function = function.clone();
        let targets = self.targets(&mut function)?;
        let chart = flow::checked_chart(&function, content)
            .map_err(|error| Unavailable::new(Reason::InvalidFlow, format!("{error:#}")))?;
        Ok(Some(Ready {
            node,
            function,
            chart,
            content,
            targets,
        }))
    }

    fn node(&self, symbol: &str) -> Result<Option<&Node>, Unavailable> {
        match self.symbols.get(symbol) {
            None => Ok(None),
            Some(Some(index)) => Ok(Some(&self.nodes[*index])),
            Some(None) => Err(Unavailable::new(
                Reason::DuplicateNode,
                "Saved symbol has multiple node owners",
            )),
        }
    }

    fn targets(&self, function: &mut FunctionFlowIR) -> Result<Vec<Symbol<'_>>, Unavailable> {
        let mut targets = BTreeMap::new();
        let mut failure = None;
        flow::visit_calls_mut(function, &mut |call| {
            for target in &call.targets {
                match self.target(&target.symbol) {
                    Ok(node) => {
                        targets.insert(node.symbol, node);
                    }
                    Err(error) => {
                        failure.get_or_insert(error);
                    }
                }
            }
        });
        match failure {
            Some(error) => Err(error),
            None => Ok(targets.into_values().collect()),
        }
    }

    fn target(&self, symbol: &str) -> Result<Symbol<'_>, Unavailable> {
        self.node(symbol)
            .and_then(|node| {
                node.ok_or_else(|| {
                    Unavailable::new(
                        Reason::InvalidTarget,
                        "Flow target is absent from this endpoint",
                    )
                })
            })
            .map(ownership::symbol)
            .map_err(|error| Unavailable::new(Reason::InvalidTarget, error.message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{architecture::FileFacts, flow::SourceSpan};

    fn graph() -> GraphData {
        GraphData {
            nodes: vec![Node {
                id: "method".into(),
                label: "method".into(),
                signature: "method()".into(),
                language: "typescript".into(),
                file: "@repo/main.ts".into(),
                build: "@repo/tsconfig.json".into(),
                project: "example".into(),
                ..Default::default()
            }],
            architecture: Some(ArchitectureFacts {
                version: crate::architecture::FACTS_VERSION,
                files: vec![FileFacts {
                    file: "@repo/main.ts".into(),
                    build: "@repo/tsconfig.json".into(),
                    project: "example".into(),
                    content: "function method() {}".into(),
                    ..Default::default()
                }],
            }),
            flow_inventory: Some(FlowInventory {
                version: flow::INVENTORY_VERSION,
                functions: vec![FunctionFlowIR {
                    symbol: "method".into(),
                    file: "@repo/main.ts".into(),
                    build: "@repo/tsconfig.json".into(),
                    project: "example".into(),
                    language: "typescript".into(),
                    span: SourceSpan { start: 0, end: 20 },
                    ..Default::default()
                }],
            }),
            ..Default::default()
        }
    }

    #[test]
    fn borrowed_and_saved_views_produce_identical_flow_answers() {
        let graph = graph();
        let saved = Artifact {
            schema: SCHEMA.into(),
            graph: serde_json::to_value(&graph).unwrap(),
            extensions: Default::default(),
        };
        let borrowed = View::from_parts(&graph, graph.flow_inventory.as_ref()).unwrap();
        let decoded = View::new(&saved).unwrap();
        assert_eq!(
            crate::query::function_flow(&borrowed, "method").unwrap(),
            crate::query::function_flow(&decoded, "method").unwrap(),
        );
        let input = borrowed.input("method").unwrap().unwrap();
        assert!(std::ptr::eq(
            input.content.as_ptr(),
            graph.architecture.as_ref().unwrap().files[0]
                .content
                .as_ptr()
        ));
    }

    #[test]
    fn duplicate_function_and_mismatched_owner_remain_unavailable() {
        let mut graph = graph();
        let inventory = graph.flow_inventory.as_mut().unwrap();
        inventory.functions.push(inventory.functions[0].clone());
        let view = View::from_parts(&graph, graph.flow_inventory.as_ref()).unwrap();
        assert!(matches!(
            view.recording("method"),
            Recording::Unavailable(Unavailable {
                reason: Reason::DuplicateFunction,
                ..
            })
        ));
        drop(view);
        let inventory = graph.flow_inventory.as_mut().unwrap();
        inventory.functions.pop();
        inventory.functions[0].project = "other".into();
        let view = View::from_parts(&graph, graph.flow_inventory.as_ref()).unwrap();
        assert!(matches!(
            view.recording("method"),
            Recording::Unavailable(Unavailable {
                reason: Reason::FunctionOwnership,
                ..
            })
        ));
    }

    #[test]
    fn saved_source_absence_is_rejected_before_defaults_erase_it() {
        let mut value = serde_json::to_value(graph()).unwrap();
        value["architecture"]["files"][0]
            .as_object_mut()
            .unwrap()
            .remove("content");
        let artifact = Artifact {
            schema: SCHEMA.into(),
            graph: value,
            extensions: Default::default(),
        };
        assert!(matches!(
            read_structure(&artifact),
            Err(Unavailable {
                reason: Reason::MissingSource,
                ..
            })
        ));
        assert!(matches!(
            View::new(&artifact),
            Err(Unavailable {
                reason: Reason::MissingSource,
                ..
            })
        ));
    }
}
