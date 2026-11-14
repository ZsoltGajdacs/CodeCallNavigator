//! Read-only source flows bound to saved Git endpoints before current-graph preparation.
mod catalog;
mod encoding;
mod endpoint;
mod model;
mod validation;
use super::{comparison::endpoints::EndpointRequest, query};
use codecallnavigator_core::{
    artifact::revision::ArtifactResult,
    query::{FlowComparisonSelection, Query, Request, Response, Result},
};
use encoding::Reply;
use endpoint::Views;
use model::{Correspondence, Detail, Header};

pub(super) fn respond(request: &Request) -> Option<Response> {
    let Query::FlowComparison {
        base,
        head,
        expected_comparison,
        selection,
    } = &request.query
    else {
        return None;
    };
    let endpoints = EndpointRequest {
        base,
        head,
        expected: expected_comparison.as_deref(),
    };
    Some(
        execute(request, endpoints, selection)
            .unwrap_or_else(|error| query::envelope(None, Err(error))),
    )
}
fn execute(
    request: &Request,
    endpoints: EndpointRequest<'_>,
    selection: &FlowComparisonSelection,
) -> Result<Response> {
    validation::validate(&endpoints, selection)?;
    let pair = endpoints.inspect(request)?;
    // WORKTREE freshness may observe live inputs; charts and source always use saved facts.
    let freshness = match &pair.after.artifact {
        ArtifactResult::Available(artifact) => {
            endpoints.freshness(request, artifact, &pair.after.endpoint.digest)?
        }
        ArtifactResult::Unavailable(_) => None,
    };
    let views = Views::new(&pair.before, &pair.after);
    let header = Header {
        version: 1,
        base: &pair.before.endpoint,
        head: &pair.after.endpoint,
        comparison_id: &pair.comparison_id,
    };
    let reply = Reply {
        generation: &pair.after.endpoint.digest,
        freshness: freshness.as_ref(),
    };
    match selection {
        FlowComparisonSelection::Catalog {
            text,
            offset,
            limit,
        } => catalog::respond(
            header,
            &views,
            catalog::CatalogRequest {
                text,
                offset: *offset,
                limit: *limit,
            },
            &reply,
        ),
        FlowComparisonSelection::Detail {
            symbol,
            head_symbol,
        } => {
            let selected = head_symbol.as_deref().unwrap_or(symbol);
            let correspondence = Correspondence {
                kind: if selected == symbol {
                    "exact"
                } else {
                    "manual"
                },
                symbol,
                head_symbol: selected,
            };
            reply.finish(&Detail {
                header,
                kind: "detail",
                correspondence,
                before: views.before.detail(symbol),
                after: views.after.detail(selected),
            })
        }
    }
}
