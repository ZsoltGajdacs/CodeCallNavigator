//! Source comparisons are bound independently of the saved graph catalog.
use super::{
    ChangeList, ComparisonRefs, Error, SourceDiff,
    changes::{ChangesPage, DiffResponse, PAGE_LIMIT},
    client::Result,
    model::is_hex_id,
};
use crate::engine_client::QueryContext;
use anyhow::{Context, ensure};
use codecallnavigator::query::{ComparisonChangesSelection as Selection, Query};
use serde::{Deserialize, de::DeserializeOwned};
use std::sync::atomic::AtomicBool;

#[derive(Debug, Deserialize)]
struct SourceHeader {
    version: u32,
    base: SourceEndpoint,
    head: SourceEndpoint,
    comparison_id: String,
    graph_comparison_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SourceEndpoint {
    requested_ref: String,
    commit: Option<String>,
}

#[derive(Clone)]
pub struct BoundSourceComparison {
    context: QueryContext,
    references: ComparisonRefs,
    comparison_id: String,
    graph_comparison_id: Option<String>,
}

impl QueryContext {
    pub fn open_source_comparison(
        mut self,
        mut references: ComparisonRefs,
        cancel: &AtomicBool,
    ) -> Result<Option<(BoundSourceComparison, ChangeList)>> {
        if !self.client.git_comparison_changes_supported(cancel)? {
            return Ok(None);
        }
        self.descriptor.generation = None;
        let page: ChangesPage<SourceHeader> =
            request(&self, source_query(&references, None, list(0)), cancel)?;
        page.header.validate(&references)?;
        page.page.validate(0)?;
        let SourceHeader {
            base,
            head,
            comparison_id,
            graph_comparison_id,
            ..
        } = page.header;
        references.base = base.commit.expect("validated base commit");
        if let Some(commit) = head.commit {
            references.head = commit;
        }
        Ok(Some((
            BoundSourceComparison {
                context: self,
                references,
                comparison_id,
                graph_comparison_id,
            },
            page.page,
        )))
    }
}

impl BoundSourceComparison {
    pub fn is_worktree(&self) -> bool {
        self.references.head == "WORKTREE"
    }

    pub fn graph_comparison_id(&self) -> Option<&str> {
        self.graph_comparison_id.as_deref()
    }

    pub fn changes(&self, offset: usize, cancel: &AtomicBool) -> Result<ChangeList> {
        let page: ChangesPage<SourceHeader> = self.request(list(offset), cancel)?;
        self.check(&page.header)?;
        page.page.validate(offset)?;
        Ok(page.page)
    }

    pub fn source_diff(&self, id: String, cancel: &AtomicBool) -> Result<SourceDiff> {
        let selection = Selection::SourceDiff { id: id.clone() };
        let reply: DiffResponse<SourceHeader> = self.request(selection, cancel)?;
        self.check(&reply.header)?;
        reply.diff.validate(&id)?;
        Ok(reply.diff)
    }

    fn check(&self, header: &SourceHeader) -> anyhow::Result<()> {
        header.validate(&self.references)?;
        ensure!(
            header.comparison_id == self.comparison_id,
            "CLI returned another source comparison binding"
        );
        Ok(())
    }

    fn request<T: DeserializeOwned>(&self, selection: Selection, cancel: &AtomicBool) -> Result<T> {
        let query = source_query(
            &self.references,
            Some(self.comparison_id.clone()),
            selection,
        );
        request(&self.context, query, cancel)
    }
}

impl SourceHeader {
    fn validate(&self, refs: &ComparisonRefs) -> anyhow::Result<()> {
        ensure!(
            self.version == 1,
            "Unsupported CLI source comparison version"
        );
        ensure!(
            self.base.requested_ref == refs.base && self.head.requested_ref == refs.head,
            "CLI returned different source comparison references"
        );
        ensure!(
            is_hex_id(&self.comparison_id, 64)
                && self
                    .graph_comparison_id
                    .as_deref()
                    .is_none_or(|id| is_hex_id(id, 64)),
            "Invalid CLI source comparison binding"
        );
        ensure!(
            self.base.matches_commit_ref(&refs.base)
                && if refs.head == "WORKTREE" {
                    self.head.commit.is_none()
                } else {
                    self.head.matches_commit_ref(&refs.head)
                },
            "Invalid CLI source comparison commit"
        );
        Ok(())
    }
}

impl SourceEndpoint {
    fn matches_commit_ref(&self, reference: &str) -> bool {
        let exact_commit = |value: &str| is_hex_id(value, 40) || is_hex_id(value, 64);
        self.commit.as_deref().is_some_and(|commit| {
            exact_commit(commit)
                && (!exact_commit(reference) || commit.eq_ignore_ascii_case(reference))
        })
    }
}

fn list(offset: usize) -> Selection {
    Selection::List {
        offset,
        limit: PAGE_LIMIT,
    }
}

fn source_query(
    refs: &ComparisonRefs,
    expected_comparison: Option<String>,
    selection: Selection,
) -> Query {
    Query::GitComparisonChanges {
        base: refs.base.clone(),
        head: refs.head.clone(),
        base_snapshot: refs.base_snapshot.clone(),
        head_snapshot: refs.head_snapshot.clone(),
        expected_comparison,
        selection,
    }
}

fn request<T: DeserializeOwned>(
    context: &QueryContext,
    query: Query,
    cancel: &AtomicBool,
) -> Result<T> {
    let response = context.client.query(&context.wire_request(query), cancel)?;
    if let Some(error) = response.error {
        return Err(Error::Cli(error));
    }
    if response.generation.is_some() {
        return Err(anyhow::anyhow!("CLI source comparison returned a graph generation").into());
    }
    let result = response
        .result
        .context("CLI omitted the source comparison result")?;
    Ok(serde_json::from_value(result).context("Invalid CLI source comparison result")?)
}
