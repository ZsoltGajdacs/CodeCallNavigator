//! Desktop adapters for generation-bound CLI answers. Fixture use is explicit.
pub mod directory;
mod render;
mod search;
use crate::{engine_client::QueryContext, projection, source};
use anyhow::{Context, Result, ensure};
use codecallnavigator::{
    architecture, flow,
    graph::{Filter, GraphIndex},
    query::Query,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

#[derive(Clone)]
pub struct GraphQueries {
    graph: Arc<GraphIndex>,
    backend: Backend,
    layer_selection: Option<architecture::Selection>,
    layer_signature: Option<String>,
}
#[derive(Clone)]
enum Backend {
    Cli(QueryContext),
    Fixture,
}

impl GraphQueries {
    pub fn cli(graph: Arc<GraphIndex>, context: QueryContext) -> Self {
        Self {
            graph,
            backend: Backend::Cli(context),
            layer_selection: None,
            layer_signature: None,
        }
    }
    pub fn with_layers(
        mut self,
        selection: Option<architecture::Selection>,
        signature: Option<String>,
    ) -> Self {
        self.layer_selection = selection;
        self.layer_signature = signature;
        self
    }
    pub fn fixture(graph: Arc<GraphIndex>) -> Self {
        Self {
            graph,
            backend: Backend::Fixture,
            layer_selection: None,
            layer_signature: None,
        }
    }

    pub fn inspect(&self, node: u32, filter: &Filter, cancel: &AtomicBool) -> Result<Inspection> {
        let Backend::Cli(context) = &self.backend else {
            let (incoming, outgoing, relationships) =
                crate::fixture_queries::inspect(&self.graph, node, filter);
            let classifications = relationships
                .iter()
                .map(|id| (*id, self.graph.classification(*id)))
                .collect();
            return Ok(Inspection {
                incoming,
                outgoing,
                relationships,
                classifications,
            });
        };
        #[derive(Deserialize)]
        struct WireInspection {
            incoming: Vec<String>,
            outgoing: Vec<String>,
            snapshot_edges: Vec<u32>,
            relationships: Vec<render::SemanticEdge>,
        }
        let result: WireInspection = context.request(
            Query::Inspect {
                symbol: self.graph.data.nodes[node as usize].id.clone(),
                filter: filter.clone(),
            },
            cancel,
        )?;
        validate_indices(&self.graph, &[], &result.snapshot_edges)?;
        ensure!(
            result.snapshot_edges.len() == result.relationships.len(),
            "CLI inspector classifications do not match returned edges"
        );
        let classifications = result
            .snapshot_edges
            .iter()
            .copied()
            .zip(
                result
                    .relationships
                    .into_iter()
                    .map(|edge| edge.classification),
            )
            .collect();
        Ok(Inspection {
            incoming: self.indices(&result.incoming)?,
            outgoing: self.indices(&result.outgoing)?,
            relationships: result.snapshot_edges,
            classifications,
        })
    }

    fn indices(&self, symbols: &[String]) -> Result<Vec<u32>> {
        symbols
            .iter()
            .map(|id| {
                self.graph
                    .node_index(id)
                    .with_context(|| format!("CLI referenced missing snapshot symbol {id}"))
            })
            .collect()
    }

    pub fn flow(
        &self,
        symbol: &str,
        cancel: &AtomicBool,
    ) -> Result<(flow::FlowChart, String, String)> {
        let Backend::Cli(context) = &self.backend else {
            return crate::fixture_queries::flow(&self.graph, symbol);
        };
        #[derive(Deserialize)]
        struct Source {
            content: String,
            file: String,
        }
        #[derive(Deserialize)]
        struct Flow {
            chart: flow::FlowChart,
            source: Source,
        }
        let result: Flow = context.request(
            Query::Flow {
                symbol: symbol.into(),
            },
            cancel,
        )?;
        ensure!(
            result.source.content.len() as u64 <= source::MAX_SOURCE_BYTES,
            "The analyzed source exceeds the 16 MiB flow source limit"
        );
        Ok((result.chart, result.source.content, result.source.file))
    }

    pub fn source(&self, path: &str, cancel: &AtomicBool) -> Result<source::SourceFile> {
        match &self.backend {
            Backend::Cli(context) => live_source(context, path, cancel),
            Backend::Fixture => source::load_fixture(path).map_err(anyhow::Error::msg),
        }
    }

    pub fn stale(&self, cancel: &AtomicBool) -> Result<bool> {
        match &self.backend {
            Backend::Cli(context) => context.stale(cancel),
            Backend::Fixture => Ok(false),
        }
    }

    pub fn architecture(
        &self,
        previous: Option<String>,
        cancel: &AtomicBool,
    ) -> Result<ArchitectureResult> {
        let Backend::Cli(context) = &self.backend else {
            anyhow::bail!("Fixture architecture is prepared explicitly");
        };
        context.request(
            Query::Architecture {
                selection: self.layer_selection,
                previous_signature: previous,
                configuration_root: None,
            },
            cancel,
        )
    }
}

#[derive(Deserialize)]
pub struct ArchitectureResult {
    pub selection: architecture::Selection,
    pub signature: String,
    pub changed: bool,
    pub assignments: Option<architecture::LayerAssignments>,
}

fn validate_indices(graph: &GraphIndex, nodes: &[u32], edges: &[u32]) -> Result<()> {
    ensure!(
        nodes
            .iter()
            .all(|id| (*id as usize) < graph.data.nodes.len()),
        "CLI returned invalid render node offsets"
    );
    ensure!(
        edges
            .iter()
            .all(|id| (*id as usize) < graph.data.edges.len()),
        "CLI returned invalid render edge offsets"
    );
    Ok(())
}
fn text(value: &Value, key: &str) -> Result<String> {
    Ok(value[key]
        .as_str()
        .with_context(|| format!("CLI result omitted {key}"))?
        .into())
}

fn live_source(
    context: &QueryContext,
    path: &str,
    cancel: &AtomicBool,
) -> Result<source::SourceFile> {
    let result: Value = context.request(
        Query::LiveSource {
            path: PathBuf::from(path),
        },
        cancel,
    )?;
    Ok(source::from_snapshot(
        text(&result, "content")?,
        Path::new(path),
    ))
}

pub struct Inspection {
    pub incoming: Vec<u32>,
    pub outgoing: Vec<u32>,
    pub relationships: Vec<u32>,
    pub classifications:
        std::collections::HashMap<u32, codecallnavigator::model::RelationshipClassification>,
}
