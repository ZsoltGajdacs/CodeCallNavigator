//! Git source survives missing graphs. Graph endpoints only enrich a captured edit.
use super::{
    cancellation::CANCELLED,
    comparison::endpoints::{self, EndpointRequest},
    comparison_changes, query,
    response_encoding::Reply,
};
use codecallnavigator_core::{
    artifact::{
        comparison_changes::GitComparison,
        revision::{self, Repository, Snapshot},
    },
    query::{
        ComparisonChangesSelection, Error, GIT_COMPARISON_CHANGES_VERSION, Query, Request,
        Response, Result,
    },
};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

pub(super) fn respond(request: &Request) -> Option<Response> {
    let Query::GitComparisonChanges {
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
    validate(request, &endpoints, selection)?;
    let project = endpoints::project(request)?;
    let repository = Repository::open(&project).map_err(endpoints::revision_error)?;
    let base = repository
        .resolve(endpoints.base)
        .map_err(endpoints::revision_error)?;
    let head = (endpoints.head != "WORKTREE")
        .then(|| repository.resolve(endpoints.head))
        .transpose()
        .map_err(endpoints::revision_error)?;
    let source = repository
        .source_changes(&base, head.as_deref(), &CANCELLED)
        .map_err(endpoints::revision_error)?;
    let graphs = Graphs::load(&project, &endpoints, &base, head.as_deref());
    let comparison_id = endpoints::identity(
        &(
            "git_comparison_changes",
            GIT_COMPARISON_CHANGES_VERSION,
            &source.digest,
        ),
        &graphs.identities(),
    )?;
    endpoints.check_binding(&comparison_id)?;
    let header = Header {
        version: GIT_COMPARISON_CHANGES_VERSION,
        base: SourceEndpoint {
            requested_ref: endpoints.base,
            commit: Some(&base),
        },
        head: SourceEndpoint {
            requested_ref: endpoints.head,
            commit: head.as_deref(),
        },
        comparison_id: &comparison_id,
        graph_comparison_id: graphs.comparison_id()?,
        graph_freshness: graphs.freshness(request, &endpoints),
    };
    let comparison = GitComparison::new(
        source,
        graphs.before.as_ref().map(|graph| &graph.artifact),
        graphs.after.as_ref().map(|graph| &graph.artifact),
    )
    .map_err(|error| Error::new("comparison_failed", format!("{error:#}")))?;
    let reply = Reply {
        generation: None,
        freshness: None,
    };
    comparison_changes::respond_selection(
        (&reply, header),
        comparison_changes::ChangeRows {
            rows: &comparison.changes,
            entry_point_search_complete: comparison.entry_point_search_complete,
        },
        selection,
        |id| comparison.source_diff(id),
    )
}

fn validate(
    request: &Request,
    endpoints: &EndpointRequest<'_>,
    selection: &ComparisonChangesSelection,
) -> Result<()> {
    comparison_changes::validate(endpoints, selection)?;
    if request.expected_generation.is_some() {
        return Err(Error::new(
            "invalid_request",
            "Use expected_comparison to bind a Git source comparison",
        ));
    }
    if endpoints.head == "WORKTREE" && endpoints.head_snapshot.is_some() {
        return Err(Error::new(
            "invalid_request",
            "WORKTREE does not take a snapshot ID",
        ));
    }
    Ok(())
}

fn optional_graph(
    project: &Path,
    requested: &str,
    resolved: &str,
    variant: Option<&str>,
) -> Option<Snapshot> {
    let mut graph = revision::inspect_cached(project, resolved, variant)
        .ok()?
        .into_snapshot()
        .ok()?;
    graph.endpoint.requested_ref = requested.into();
    Some(graph)
}

struct Graphs {
    before: Option<Snapshot>,
    after: Option<Snapshot>,
}

impl Graphs {
    fn load(
        project: &Path,
        endpoints: &EndpointRequest<'_>,
        base: &str,
        head: Option<&str>,
    ) -> Self {
        Self {
            before: optional_graph(project, endpoints.base, base, endpoints.base_snapshot),
            after: optional_graph(
                project,
                endpoints.head,
                head.unwrap_or("WORKTREE"),
                endpoints.head_snapshot,
            ),
        }
    }

    fn identities(&self) -> [Option<(&str, Option<&str>)>; 2] {
        [&self.before, &self.after].map(|graph| {
            graph.as_ref().map(|graph| {
                let endpoint = &graph.endpoint;
                (
                    endpoint.digest.as_str(),
                    endpoint.snapshot.as_ref().map(|record| record.id.as_str()),
                )
            })
        })
    }

    fn comparison_id(&self) -> Result<Option<String>> {
        self.before
            .as_ref()
            .zip(self.after.as_ref())
            .map(|(a, b)| endpoints::identity(&a.endpoint, &b.endpoint))
            .transpose()
    }

    fn freshness(&self, request: &Request, endpoints: &EndpointRequest<'_>) -> Option<Value> {
        let after = self.after.as_ref()?;
        endpoints
            .freshness(request, &after.artifact, &after.endpoint.digest)
            .ok()
            .flatten()
    }
}

#[derive(Serialize)]
struct Header<'a> {
    version: u32,
    base: SourceEndpoint<'a>,
    head: SourceEndpoint<'a>,
    comparison_id: &'a str,
    graph_comparison_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    graph_freshness: Option<Value>,
}

#[derive(Serialize)]
struct SourceEndpoint<'a> {
    requested_ref: &'a str,
    commit: Option<&'a str>,
}
