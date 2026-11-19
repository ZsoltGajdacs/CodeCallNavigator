//! Comparison orchestration loads portable snapshots, never a runtime graph index.
pub(super) mod endpoints;
use super::query;
use codecallnavigator_core::{
    artifact::difference,
    query::{Error, Query, Request, Response},
};
use serde_json::{Value, json};

pub(super) fn respond(request: &Request) -> Option<Response> {
    let Query::Compare {
        base,
        head,
        base_snapshot,
        head_snapshot,
        offset,
        limit,
        expected_comparison,
    } = &request.query
    else {
        return None;
    };
    let options = Options {
        endpoints: endpoints::EndpointRequest {
            base,
            head,
            base_snapshot: base_snapshot.as_deref(),
            head_snapshot: head_snapshot.as_deref(),
            expected: expected_comparison.as_deref(),
        },
        offset: *offset,
        limit: *limit,
    };
    let result = execute(request, options);
    Some(result.unwrap_or_else(|error| query::envelope(None, Err(error))))
}

struct Options<'a> {
    endpoints: endpoints::EndpointRequest<'a>,
    offset: usize,
    limit: usize,
}

fn execute(
    request: &Request,
    options: Options<'_>,
) -> codecallnavigator_core::query::Result<Response> {
    validate(&options)?;
    let Options {
        endpoints,
        offset,
        limit,
    } = options;
    let endpoints::Pair {
        before,
        after,
        comparison_id,
    } = endpoints.load(request)?;
    let freshness = endpoints.freshness(request, &after.artifact, &after.endpoint.digest)?;
    // Binding and freshness describe the exact loaded bytes. Once recorded,
    // comparison owns the artifacts and can canonicalize without full copies.
    let diff = difference::compare_owned(before.artifact, after.artifact)
        .map_err(|e| Error::new("comparison_failed", format!("{e:#}")))?;
    let total = diff.changes.len();
    let result = json!({
        "version":difference::VERSION,"base":before.endpoint,"head":after.endpoint,
        "comparison_id":comparison_id,"graph_equal":diff.graph_equal,"metadata_equal":diff.metadata_equal,
        "summary":diff.summary,"changes":[],"offset":offset,"limit":limit,
        "returned":0,"total":total,"more":false,"next_offset":null
    });
    let response = Response {
        version: codecallnavigator_core::query::VERSION,
        generation: Some(after.endpoint.digest),
        freshness,
        result: Some(result),
        error: None,
    };
    page(
        response,
        diff.changes.iter().skip(offset).take(limit),
        offset,
        total,
    )
}

fn validate(options: &Options<'_>) -> codecallnavigator_core::query::Result<()> {
    options.endpoints.validate_base()?;
    if !(1..=500).contains(&options.limit) {
        return Err(Error::new(
            "invalid_request",
            "Comparison limit must be 1..500",
        ));
    }
    if options.offset > 0 && options.endpoints.expected.is_none() {
        return Err(Error::new(
            "invalid_request",
            "Paged comparisons require expected_comparison",
        ));
    }
    options.endpoints.validate_references()?;
    options.endpoints.validate_expected()
}

fn page<'a>(
    mut response: Response,
    changes: impl Iterator<Item = &'a difference::Change>,
    offset: usize,
    total: usize,
) -> codecallnavigator_core::query::Result<Response> {
    let overhead = serde_json::to_vec(&response).map_err(encoding_error)?.len() + 1024;
    let mut size = overhead;
    let mut records = Vec::new();
    for change in changes {
        let record = serde_json::to_value(change).map_err(encoding_error)?;
        let record_size = serde_json::to_vec(&record).map_err(encoding_error)?.len() + 1;
        if size.saturating_add(record_size) > difference::MAX_RESPONSE_BYTES {
            if records.is_empty() {
                return Err(Error::new(
                    "response_too_large",
                    "A complete comparison record exceeds the response budget",
                ));
            }
            break;
        }
        size += record_size;
        records.push(record);
    }
    if overhead > difference::MAX_RESPONSE_BYTES {
        return Err(Error::new(
            "response_too_large",
            "Comparison metadata exceeds the response budget",
        ));
    }
    let returned = records.len();
    let next = offset.saturating_add(returned);
    let result = response.result.as_mut().expect("comparison result exists");
    result["changes"] = Value::Array(records);
    result["returned"] = returned.into();
    result["more"] = (next < total).into();
    result["next_offset"] = if next < total {
        next.into()
    } else {
        Value::Null
    };
    Ok(response)
}

fn encoding_error(error: serde_json::Error) -> Error {
    Error::new("serialization_failed", error.to_string())
}
