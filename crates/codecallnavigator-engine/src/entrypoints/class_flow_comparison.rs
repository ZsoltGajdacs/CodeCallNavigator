//! Read-only class calls from the endpoint pair captured by the change outline.
use super::{comparison::endpoints::EndpointRequest, query, response_encoding::Reply};
use codecallnavigator_core::{
    artifact::{comparison_changes::ClassGraph, revision::Endpoint},
    query::{CLASS_FLOW_COMPARISON_VERSION, Error, Query, Request, Response, Result},
};
use serde::Serialize;

pub(super) fn respond(request: &Request) -> Option<Response> {
    let Query::ClassFlowComparison {
        base,
        head,
        base_snapshot,
        head_snapshot,
        expected_comparison,
        id,
    } = &request.query
    else {
        return None;
    };
    let endpoints = EndpointRequest {
        base,
        head,
        base_snapshot: base_snapshot.as_deref(),
        head_snapshot: head_snapshot.as_deref(),
        expected: Some(expected_comparison),
    };
    Some(execute(request, endpoints, id).unwrap_or_else(|error| query::envelope(None, Err(error))))
}

fn execute(request: &Request, endpoints: EndpointRequest<'_>, id: &str) -> Result<Response> {
    endpoints.validate_base()?;
    endpoints.validate_references()?;
    endpoints.validate_expected()?;
    if id.trim().is_empty() || id.len() > 16384 {
        return Err(Error::new(
            "invalid_request",
            "Class ID must contain 1..16384 bytes",
        ));
    }
    let pair = endpoints.load(request)?;
    let graph = ClassGraph::between(&pair.before.artifact, &pair.after.artifact, id)
        .map_err(|error| Error::new("comparison_failed", format!("{error:#}")))?
        .ok_or_else(|| Error::new("selection_unavailable", "No such class in this comparison"))?;
    let freshness =
        endpoints.freshness(request, &pair.after.artifact, &pair.after.endpoint.digest)?;
    let reply = Reply {
        generation: Some(&pair.after.endpoint.digest),
        freshness: freshness.as_ref(),
    };
    reply.finish_with(
        &Detail {
            version: CLASS_FLOW_COMPARISON_VERSION,
            base: &pair.before.endpoint,
            head: &pair.after.endpoint,
            comparison_id: &pair.comparison_id,
            graph,
        },
        Error::new(
            "response_too_large",
            "Class flow comparison exceeds the response budget",
        ),
    )
}

#[derive(Serialize)]
struct Detail<'a> {
    version: u32,
    base: &'a Endpoint,
    head: &'a Endpoint,
    comparison_id: &'a str,
    graph: ClassGraph,
}
