//! One retained graph plus tracked Git changes. No head graph is opened.
use super::{comparison::endpoints, query, response_encoding};
use codecallnavigator_core::{
    artifact::{difference::MAX_RESPONSE_BYTES, impact, revision},
    query::{Error, Query, Request, Response, Result, VERSION},
};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

pub(super) fn cli(project: PathBuf, query: Query) -> anyhow::Result<i32> {
    let response = respond(&Request {
        version: VERSION,
        project,
        roots: Vec::new(),
        expected_generation: None,
        query,
    })
    .expect("impact preview command constructs an impact preview query");
    if let Some(error) = response.error {
        anyhow::bail!("{}: {}", error.code, error.message);
    }
    println!("{}", serde_json::to_string_pretty(&response.result)?);
    Ok(0)
}

pub(super) fn respond(request: &Request) -> Option<Response> {
    let Query::ImpactPreview {
        base,
        head,
        base_snapshot,
    } = &request.query
    else {
        return None;
    };
    Some(query::envelope(
        None,
        execute(request, base, head, base_snapshot.as_deref()),
    ))
}

fn execute(request: &Request, base: &str, head: &str, selected: Option<&str>) -> Result<Value> {
    let refs = endpoints::EndpointRequest {
        base,
        head,
        base_snapshot: selected,
        head_snapshot: None,
        expected: None,
    };
    refs.validate_base()?;
    refs.validate_references()?;
    if request.expected_generation.is_some() {
        return Err(Error::new(
            "invalid_request",
            "Select an impact preview baseline with base_snapshot",
        ));
    }
    let repository = revision::Repository::open(&endpoints::project(request)?)
        .map_err(endpoints::revision_error)?;
    let baseline =
        revision::load_retained(&repository, base, selected).map_err(endpoints::revision_error)?;
    let base_commit = baseline.endpoint.commit.as_deref().ok_or_else(|| {
        Error::new(
            "snapshot_unavailable",
            "Baseline snapshot is missing its commit",
        )
    })?;
    let head_commit = (head != "WORKTREE")
        .then(|| repository.resolve(head))
        .transpose()
        .map_err(endpoints::revision_error)?;
    let changes = repository
        .tracked_changes(base_commit, head_commit.as_deref())
        .map_err(|error| Error::new("diff_unavailable", format!("{error:#}")))?;
    let preview = impact::preview(&baseline.artifact, changes.records)
        .map_err(|error| Error::new("invalid_graph", format!("{error:#}")))?;
    let result = Report {
        version: 1,
        base: baseline.endpoint,
        head: Head {
            requested_ref: head,
            commit: head_commit,
        },
        coverage: Coverage {
            tracked_only: true,
            changed_paths: changes.changed_paths,
            mapped_ranges: preview.mapped_ranges,
            unmapped_ranges: preview.unmapped_ranges,
            truncated: changes.truncated || preview.truncated,
        },
        records: preview.records,
    };
    if response_encoding::record_size(&result, MAX_RESPONSE_BYTES - 2048)?.is_none() {
        return Err(Error::new(
            "response_too_large",
            "Impact preview exceeds the response budget",
        ));
    }
    serde_json::to_value(result)
        .map_err(|error| Error::new("serialization_failed", error.to_string()))
}

#[derive(Serialize)]
struct Report<'a, T> {
    version: u32,
    base: revision::Endpoint,
    head: Head<'a>,
    coverage: Coverage,
    records: T,
}

#[derive(Serialize)]
struct Head<'a> {
    requested_ref: &'a str,
    commit: Option<String>,
}

#[derive(Serialize)]
struct Coverage {
    tracked_only: bool,
    changed_paths: usize,
    mapped_ranges: usize,
    unmapped_ranges: usize,
    truncated: bool,
}
