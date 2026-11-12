//! Adapt complete CLI semantic membership to presentation-local offsets.
use super::*;
use codecallnavigator::query::{RenderLayer, SelectionKind};

#[derive(Deserialize)]
pub(super) struct RenderResult {
    pub snapshot_nodes: Vec<u32>,
    pub snapshot_edges: Vec<u32>,
    pub roots: Vec<String>,
    pub omitted_nodes: usize,
    pub omitted_edges: usize,
    pub summary: RenderSummary,
    pub edges: Vec<SemanticEdge>,
}
#[derive(Deserialize)]
pub(super) struct SemanticEdge {
    pub classification: codecallnavigator::model::RelationshipClassification,
}
#[derive(Deserialize)]
pub(super) struct RenderSummary {
    cyclic_symbols: Vec<String>,
    hidden_nodes: usize,
    hidden_edges: usize,
}

impl GraphQueries {
    fn render_layer(&self, request: &projection::Request) -> Result<Option<RenderLayer>> {
        if !projection::layer_scope(request) {
            return Ok(None);
        }
        ensure!(
            self.layer_signature.is_some(),
            "architecture_pending: Waiting for CLI layer assignments"
        );
        Ok(Some(RenderLayer {
            selection: self.layer_selection,
            tab: request.layer_tab.clone(),
            expected_signature: self.layer_signature.clone(),
            configuration_root: None,
        }))
    }

    pub fn file_visible(
        &self,
        file: &str,
        request: &projection::Request,
        cancel: &AtomicBool,
    ) -> Result<bool> {
        let Backend::Cli(context) = &self.backend else {
            return Ok(projection::fixture_file_classes_visible(
                &self.graph,
                request,
                file,
            ));
        };
        let layer = self.render_layer(request)?;
        let result: RenderResult = context.request(
            Query::RenderSelection {
                selection: SelectionKind::File { file: file.into() },
                filter: request.filter.clone(),
                layer,
            },
            cancel,
        )?;
        validate_indices(&self.graph, &result.snapshot_nodes, &result.snapshot_edges)?;
        Ok(!result.snapshot_nodes.is_empty())
    }
    pub fn render(
        &self,
        request: &projection::Request,
        cancel: &AtomicBool,
    ) -> Result<projection::Selection> {
        let Backend::Cli(context) = &self.backend else {
            return Ok(crate::fixture_queries::selection(&self.graph, request));
        };
        let layer = self.render_layer(request)?;
        let result: RenderResult = context.request(
            Query::RenderSelection {
                selection: projection::query_kind(&self.graph, request),
                filter: request.filter.clone(),
                layer,
            },
            cancel,
        )?;
        validate_indices(&self.graph, &result.snapshot_nodes, &result.snapshot_edges)?;
        let summary = projection::Summary {
            cyclic: self
                .indices(&result.summary.cyclic_symbols)?
                .into_iter()
                .collect(),
            hidden_nodes: result.summary.hidden_nodes,
            hidden_edges: result.summary.hidden_edges,
        };
        let selection = codecallnavigator::query::Selection {
            nodes: result.snapshot_nodes,
            edges: result.snapshot_edges,
            roots: self.indices(&result.roots)?,
            omitted_nodes: result.omitted_nodes,
            omitted_edges: result.omitted_edges,
        };
        ensure!(
            result.edges.len() == selection.edges.len(),
            "CLI render classifications do not match returned edges"
        );
        let classifications = selection
            .edges
            .iter()
            .copied()
            .zip(result.edges.into_iter().map(|edge| edge.classification))
            .collect();
        let mut prepared = projection::prepare_selection(&self.graph, request, selection, summary);
        prepared.classifications = classifications;
        if projection::needs_overview_fallback(&self.graph, request, &prepared) {
            let mut overview = request.clone();
            overview.scope = projection::Scope::Overview;
            overview.focus_target = None;
            return self.render(&overview, cancel);
        }
        Ok(prepared)
    }

    pub fn class_members(
        &self,
        identity: &codecallnavigator::search::ClassIdentity,
        cancel: &AtomicBool,
    ) -> Result<Arc<[u32]>> {
        let Backend::Cli(context) = &self.backend else {
            return Ok(self
                .graph
                .data
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| {
                    codecallnavigator::search::ClassIdentity::from_node(node) == *identity
                })
                .map(|(id, _)| id as u32)
                .collect::<Vec<_>>()
                .into());
        };
        let filter = Filter {
            include_tests: true,
            include_generated: true,
            include_unavailable: true,
            ..Default::default()
        };
        let result: RenderResult = context.request(
            Query::RenderSelection {
                selection: SelectionKind::Class {
                    build: identity.build.clone(),
                    project: identity.project.clone(),
                    container: identity.container.clone(),
                },
                filter,
                layer: None,
            },
            cancel,
        )?;
        validate_indices(&self.graph, &result.snapshot_nodes, &result.snapshot_edges)?;
        Ok(result.snapshot_nodes.into())
    }
}
