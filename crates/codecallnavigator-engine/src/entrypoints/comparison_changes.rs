//! Snapshot-bound entity changes and unified source hunks for public query clients.
use super::{comparison::endpoints::EndpointRequest, query, response_encoding};
use codecallnavigator_core::{
    artifact::{
        comparison_changes::{Change, Comparison, SourceDiff},
        difference::MAX_RESPONSE_BYTES,
        revision::Endpoint,
    },
    query::{
        COMPARISON_CHANGES_VERSION, ComparisonChangesSelection, Error, Query, Request, Response,
        Result,
    },
};
use response_encoding::Reply;
use serde::Serialize;

pub(super) fn respond(request: &Request) -> Option<Response> {
    let Query::ComparisonChanges {
        base,
        head,
        base_snapshot,
        head_snapshot,
        expected_comparison,
        selection,
    } = &request.query
    else {
        return None;
    };
    let endpoints = EndpointRequest {
        base,
        head,
        base_snapshot: base_snapshot.as_deref(),
        head_snapshot: head_snapshot.as_deref(),
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
    selection: &ComparisonChangesSelection,
) -> Result<Response> {
    validate(&endpoints, selection)?;
    let pair = endpoints.load(request)?;
    let freshness =
        endpoints.freshness(request, &pair.after.artifact, &pair.after.endpoint.digest)?;
    let comparison = Comparison::new(&pair.before.artifact, &pair.after.artifact)
        .map_err(|error| Error::new("comparison_failed", format!("{error:#}")))?;
    let header = Header {
        version: COMPARISON_CHANGES_VERSION,
        base: &pair.before.endpoint,
        head: &pair.after.endpoint,
        comparison_id: &pair.comparison_id,
    };
    let reply = Reply {
        generation: Some(&pair.after.endpoint.digest),
        freshness: freshness.as_ref(),
    };
    respond_selection(
        (&reply, header),
        ChangeRows {
            rows: &comparison.changes,
            entry_point_search_complete: comparison.entry_point_search_complete,
        },
        selection,
        |id| comparison.source_diff(id),
    )
}

pub(super) struct ChangeRows<'a> {
    pub rows: &'a [Change],
    pub entry_point_search_complete: bool,
}

pub(super) fn respond_selection<'a>(
    (reply, header): (&Reply<'_>, impl Serialize),
    changes: ChangeRows<'a>,
    selection: &ComparisonChangesSelection,
    source_diff: impl FnOnce(&str) -> Option<SourceDiff<'a>>,
) -> Result<Response> {
    match selection {
        ComparisonChangesSelection::List { offset, limit } => list(
            header,
            changes,
            reply,
            PageWindow {
                offset: *offset,
                limit: *limit,
            },
        ),
        ComparisonChangesSelection::SourceDiff { id } => {
            let diff = source_diff(id).ok_or_else(|| {
                Error::new("selection_unavailable", "No such change in this comparison")
            })?;
            reply.finish_with(
                &SourceResult {
                    header,
                    kind: "source_diff",
                    diff,
                },
                too_large(),
            )
        }
    }
}

pub(super) fn validate(
    endpoints: &EndpointRequest<'_>,
    selection: &ComparisonChangesSelection,
) -> Result<()> {
    endpoints.validate_base()?;
    endpoints.validate_references()?;
    endpoints.validate_expected()?;
    match selection {
        ComparisonChangesSelection::List { limit, .. } if !(1..=500).contains(limit) => Err(
            Error::new("invalid_request", "Comparison limit must be 1..500"),
        ),
        ComparisonChangesSelection::SourceDiff { id }
            if id.trim().is_empty() || id.len() > 16384 =>
        {
            Err(Error::new(
                "invalid_request",
                "Change ID must contain 1..16384 bytes",
            ))
        }
        ComparisonChangesSelection::List { offset: 0, .. } => Ok(()),
        _ if endpoints.expected.is_none() => Err(Error::new(
            "invalid_request",
            "Comparison follow-up requests require expected_comparison",
        )),
        _ => Ok(()),
    }
}

#[derive(Serialize)]
struct Header<'a> {
    version: u32,
    base: &'a Endpoint,
    head: &'a Endpoint,
    comparison_id: &'a str,
}

#[derive(Serialize)]
struct Coverage {
    entry_point_search_complete: bool,
}

#[derive(Serialize)]
struct Page<'a, H> {
    #[serde(flatten)]
    header: H,
    kind: &'static str,
    coverage: Coverage,
    changes: Vec<&'a Change>,
    offset: usize,
    limit: usize,
    returned: usize,
    total: usize,
    more: bool,
    next_offset: Option<usize>,
}

#[derive(Serialize)]
struct SourceResult<'a, H> {
    #[serde(flatten)]
    header: H,
    kind: &'static str,
    #[serde(flatten)]
    diff: SourceDiff<'a>,
}

struct PageWindow {
    offset: usize,
    limit: usize,
}

fn list(
    header: impl Serialize,
    changes: ChangeRows<'_>,
    reply: &Reply<'_>,
    window: PageWindow,
) -> Result<Response> {
    let PageWindow { offset, limit } = window;
    let mut page = Page {
        header,
        kind: "list",
        coverage: Coverage {
            entry_point_search_complete: changes.entry_point_search_complete,
        },
        changes: Vec::new(),
        offset,
        limit,
        // Reserve enough bytes for any actual pagination counters.
        returned: limit,
        total: changes.rows.len(),
        more: false,
        next_offset: Some(usize::MAX),
    };
    let mut used = reply.size(&page)?.ok_or_else(too_large)?;
    for change in changes.rows.iter().skip(offset).take(limit) {
        let remaining = MAX_RESPONSE_BYTES.saturating_sub(used).saturating_sub(1);
        let Some(size) = response_encoding::record_size(change, remaining)? else {
            if page.changes.is_empty() {
                return Err(too_large());
            }
            break;
        };
        used += size + 1;
        page.changes.push(change);
    }
    page.returned = page.changes.len();
    let next = offset.saturating_add(page.returned);
    page.more = next < page.total;
    page.next_offset = page.more.then_some(next);
    reply.finish_with(&page, too_large())
}

fn too_large() -> Error {
    Error::new(
        "response_too_large",
        "A complete comparison changes response exceeds the 24 MiB response budget",
    )
}
