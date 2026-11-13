//! Comparison orchestration loads portable snapshots, never a runtime graph index.
use super::query::{self, freshness};
use codecallnavigator_core::{
    artifact::{difference, revision, verification},
    query::{Error, Query, Request, Response},
};
use serde_json::{Value, json};

pub(super) fn respond(request: &Request) -> Option<Response> {
    let Query::Compare {
        base,
        head,
        offset,
        limit,
        expected_comparison,
    } = &request.query
    else {
        return None;
    };
    let options = Options {
        base,
        head,
        offset: *offset,
        limit: *limit,
        expected: expected_comparison.as_deref(),
    };
    let result = execute(request, options);
    Some(result.unwrap_or_else(|error| query::envelope(None, Err(error))))
}

struct Options<'a> {
    base: &'a str,
    head: &'a str,
    offset: usize,
    limit: usize,
    expected: Option<&'a str>,
}

fn execute(
    request: &Request,
    options: Options<'_>,
) -> codecallnavigator_core::query::Result<Response> {
    validate(&options)?;
    let Options {
        base,
        head,
        offset,
        limit,
        expected,
    } = options;
    let project = request
        .project
        .canonicalize()
        .map_err(|e| Error::new("project_unavailable", e.to_string()))?;
    let before = revision::load(&project, base).map_err(revision_error)?;
    let after = revision::load(&project, head).map_err(revision_error)?;
    query::check_generation(request, &after.endpoint.digest)?;
    let binding = serde_json::to_vec(&(difference::VERSION, &before.endpoint, &after.endpoint))
        .map_err(encoding_error)?;
    let comparison_id = blake3::hash(&binding).to_hex().to_string();
    if expected.is_some_and(|expected| expected != comparison_id) {
        return Err(Error::new(
            "comparison_mismatch",
            "Comparison endpoints changed; restart pagination",
        ));
    }
    let freshness = if head == "WORKTREE" {
        let roots = query::roots(request)?;
        Some(freshness::summary(
            verification::verify_artifact(&after.artifact, &after.endpoint.digest, &roots)
                .map_err(|e| Error::new("verification_failed", format!("{e:#}")))?,
        )?)
    } else {
        None
    };
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
        version: 1,
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
    if options.base == "WORKTREE" {
        return Err(Error::new(
            "invalid_request",
            "Comparison base must resolve to a commit",
        ));
    }
    if !(1..=500).contains(&options.limit) {
        return Err(Error::new(
            "invalid_request",
            "Comparison limit must be 1..500",
        ));
    }
    if options.offset > 0 && options.expected.is_none() {
        return Err(Error::new(
            "invalid_request",
            "Paged comparisons require expected_comparison",
        ));
    }
    validate_reference_lengths(options)?;
    if options.expected.is_some_and(|id| !valid_digest(id)) {
        return Err(Error::new(
            "invalid_request",
            "expected_comparison must be a 64-character hexadecimal digest",
        ));
    }
    Ok(())
}

fn validate_reference_lengths(options: &Options<'_>) -> codecallnavigator_core::query::Result<()> {
    for reference in [options.base, options.head] {
        if reference.trim().is_empty() || reference.len() > 16384 {
            return Err(Error::new(
                "invalid_request",
                "Comparison references must contain 1..16384 bytes",
            ));
        }
    }
    Ok(())
}

fn valid_digest(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
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

fn revision_error(error: anyhow::Error) -> Error {
    Error::new(revision::error_code(&error), format!("{error:#}"))
}
fn encoding_error(error: serde_json::Error) -> Error {
    Error::new("serialization_failed", error.to_string())
}
