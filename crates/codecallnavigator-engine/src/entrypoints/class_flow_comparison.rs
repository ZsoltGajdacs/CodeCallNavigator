//! Read-only class calls from the endpoint pair captured by the change outline.
use super::{comparison::endpoints::EndpointRequest, query, response_encoding::Reply};
use codecallnavigator_core::{
    artifact::{comparison_changes::ClassGraph, revision::Endpoint},
    graph::{MAX_CANVAS_EDGES, MAX_CANVAS_NODES},
    query::{
        CLASS_FLOW_COMPARISON_VERSION, DEFAULT_CLASS_COMPARISON_DEPTH, Error, Query, Request,
        Response, Result, validate_class_comparison_depth,
    },
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
        depth,
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
    Some(
        execute(request, endpoints, id, *depth)
            .unwrap_or_else(|error| query::envelope(None, Err(error))),
    )
}

fn execute(
    request: &Request,
    endpoints: EndpointRequest<'_>,
    id: &str,
    depth: u32,
) -> Result<Response> {
    endpoints.validate_base()?;
    endpoints.validate_references()?;
    endpoints.validate_expected()?;
    validate_class_comparison_depth(depth)
        .map_err(|error| Error::new("invalid_request", error.to_string()))?;
    if id.trim().is_empty() || id.len() > 16384 {
        return Err(Error::new(
            "invalid_request",
            "Class ID must contain 1..16384 bytes",
        ));
    }
    let pair = endpoints.load(request)?;
    let graph =
        ClassGraph::between_at_depth(&pair.before.artifact, &pair.after.artifact, id, depth)
            .map_err(|error| Error::new("comparison_failed", format!("{error:#}")))?
            .ok_or_else(|| {
                Error::new("selection_unavailable", "No such class in this comparison")
            })?;
    let fits_canvas =
        graph.nodes.len() <= MAX_CANVAS_NODES && graph.links.len() <= MAX_CANVAS_EDGES;
    if depth > DEFAULT_CLASS_COMPARISON_DEPTH && !fits_canvas {
        return Err(Error::new(
            "response_too_large",
            "Class diagram exceeds the graph size limit. Choose a lower depth.",
        ));
    }
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
