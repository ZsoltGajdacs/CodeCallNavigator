//! On-demand source previews from the same revisions as the class graph.
use super::{comparison::endpoints::EndpointRequest, query, response_encoding::Reply};
use codecallnavigator_core::{
    artifact::{
        comparison_changes::{ClassSource, ClassSourceTarget},
        revision::Endpoint,
    },
    query::{
        CLASS_COMPARISON_SOURCE_VERSION, ComparisonSide, Error, Query, Request, Response, Result,
    },
};
use serde::Serialize;

pub(super) fn respond(request: &Request) -> Option<Response> {
    let Query::ClassComparisonSource {
        base,
        head,
        base_snapshot,
        head_snapshot,
        expected_comparison,
        id,
        node,
        side,
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
    let target = ClassSourceTarget {
        root: id,
        node,
        side: *side,
    };
    Some(
        execute(request, endpoints, target)
            .unwrap_or_else(|error| query::envelope(None, Err(error))),
    )
}

fn execute(
    request: &Request,
    endpoints: EndpointRequest<'_>,
    target: ClassSourceTarget<'_>,
) -> Result<Response> {
    endpoints.validate_base()?;
    endpoints.validate_references()?;
    endpoints.validate_expected()?;
    if [target.root, target.node]
        .iter()
        .any(|value| value.trim().is_empty() || value.len() > 16384)
    {
        return Err(Error::new(
            "invalid_request",
            "Class IDs must contain 1..16384 bytes",
        ));
    }
    let pair = endpoints.load(request)?;
    let source = ClassSource::between(&pair.before.artifact, &pair.after.artifact, target)
        .map_err(|error| Error::new("comparison_failed", format!("{error:#}")))?
        .ok_or_else(|| {
            Error::new(
                "selection_unavailable",
                "No such class in this side of the comparison graph",
            )
        })?;
    Reply {
        generation: &pair.after.endpoint.digest,
        freshness: None,
    }
    .finish_with(
        &Preview {
            version: CLASS_COMPARISON_SOURCE_VERSION,
            base: &pair.before.endpoint,
            head: &pair.after.endpoint,
            comparison_id: &pair.comparison_id,
            id: target.root,
            node: target.node,
            side: target.side,
            source,
        },
        Error::new(
            "response_too_large",
            "The class source preview exceeds the 24 MiB response budget",
        ),
    )
}

#[derive(Serialize)]
struct Preview<'a> {
    version: u32,
    base: &'a Endpoint,
    head: &'a Endpoint,
    comparison_id: &'a str,
    id: &'a str,
    node: &'a str,
    side: ComparisonSide,
    source: ClassSource,
}
