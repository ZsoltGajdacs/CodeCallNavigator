//! Borrow selected flow facts from a structurally validated portable artifact.
//! No hydration, graph index, source filesystem access, or source-text copy is needed.
mod model;
mod ownership;
mod saved;
use super::{Artifact, SCHEMA};
use crate::flow::{self, FunctionFlowIR};
pub use model::*;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub struct View<'a> {
    graph: &'a Value,
    nodes: BTreeMap<&'a str, Option<&'a Value>>,
}
struct Facts<'a> {
    node: Symbol<'a>,
    function: &'a Value,
    content: &'a str,
}
impl<'a> View<'a> {
    pub fn new(artifact: &'a Artifact) -> Result<Self, Unavailable> {
        if artifact.schema != SCHEMA {
            return Err(Unavailable::new(
                Reason::InvalidArtifact,
                "Unsupported saved artifact schema",
            ));
        }
        Ok(Self {
            graph: &artifact.graph,
            nodes: ownership::index(&artifact.graph)?,
        })
    }

    pub fn select(&self, symbol: &str) -> Selection<'a> {
        match self.prepare(symbol) {
            Ok(Some(ready)) => Selection::Ready(Box::new(ready)),
            Ok(None) => Selection::Missing,
            Err(reason) => Selection::Unavailable(reason),
        }
    }

    pub fn symbols(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.nodes.keys().copied()
    }

    pub fn describe(&self, symbol: &str) -> Result<Option<Symbol<'a>>, Unavailable> {
        self.node(symbol)?.map(ownership::symbol).transpose()
    }

    pub fn recording(&self, symbol: &str) -> Recording<'a> {
        match self.facts(symbol) {
            Ok(Some(facts)) => Recording::Recorded(facts.node),
            Ok(None) => Recording::Missing,
            Err(error) => Recording::Unavailable(error),
        }
    }

    fn facts(&self, symbol: &str) -> Result<Option<Facts<'a>>, Unavailable> {
        let Some(raw_node) = self.node(symbol)? else {
            return Ok(None);
        };
        let node = ownership::symbol(raw_node)?;
        ownership::available(&node)?;
        let function = saved::function(self.graph, symbol)?;
        saved::owner(function, node.owner)?;
        let content = saved::source(self.graph, node.owner)?;
        Ok(Some(Facts {
            node,
            function,
            content,
        }))
    }

    fn prepare(&self, symbol: &str) -> Result<Option<Ready<'a>>, Unavailable> {
        let Some(Facts {
            node,
            function: raw,
            content,
        }) = self.facts(symbol)?
        else {
            return Ok(None);
        };
        let mut function = FunctionFlowIR::deserialize(raw).map_err(|error| {
            Unavailable::new(
                Reason::InvalidFlow,
                format!("Malformed saved flow: {error}"),
            )
        })?;
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

    fn node(&self, symbol: &str) -> Result<Option<&'a Value>, Unavailable> {
        match self.nodes.get(symbol) {
            None => Ok(None),
            Some(Some(node)) => Ok(Some(*node)),
            Some(None) => Err(Unavailable::new(
                Reason::DuplicateNode,
                "Saved symbol has multiple node owners",
            )),
        }
    }

    fn targets(&self, function: &mut FunctionFlowIR) -> Result<Vec<Symbol<'a>>, Unavailable> {
        let mut targets = BTreeMap::new();
        let mut failure = None;
        // Reuse the shared IR traversal without rewriting any compiler binding.
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

    fn target(&self, symbol: &str) -> Result<Symbol<'a>, Unavailable> {
        self.node(symbol)
            .and_then(|node| {
                node.ok_or_else(|| {
                    Unavailable::new(
                        Reason::InvalidTarget,
                        "Flow target is absent from this endpoint",
                    )
                })
            })
            .and_then(ownership::symbol)
            .map_err(|error| Unavailable::new(Reason::InvalidTarget, error.message))
    }
}
