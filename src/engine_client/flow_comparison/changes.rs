//! Revision-bound change summaries and library-generated source hunks.
use super::{BoundComparison, Header};
use anyhow::{Result, ensure};
use codecallnavigator::query::ComparisonChangesSelection;
use serde::Deserialize;
use std::sync::atomic::AtomicBool;

pub const PAGE_LIMIT: usize = 100;

#[derive(Debug, Deserialize)]
pub struct ChangesPage {
    #[serde(flatten)]
    pub(super) header: Header,
    pub(super) kind: String,
    pub(super) offset: usize,
    limit: usize,
    pub(super) returned: usize,
    pub(super) total: usize,
    pub(super) more: bool,
    pub next_offset: Option<usize>,
    pub coverage: Coverage,
    pub changes: Vec<Change>,
}

#[derive(Debug, Deserialize)]
pub struct Coverage {
    pub entry_point_search_complete: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Change {
    pub id: String,
    pub kind: ChangeKind,
    pub name: String,
    pub file: String,
    pub status: ChangeStatus,
    pub class_id: Option<String>,
    pub symbol: Option<String>,
    #[serde(default)]
    pub flow_available: bool,
    pub entry_points: Vec<EntryPoint>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Class,
    Method,
    File,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    Added,
    Removed,
    Modified,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EntryPoint {
    pub id: String,
    pub name: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SourceDiff {
    #[serde(flatten)]
    pub(super) header: Header,
    pub(super) kind: String,
    pub id: String,
    pub status: SourceStatus,
    pub file: String,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Ready,
    Unavailable,
}

pub use codecallnavigator::artifact::comparison_changes::{Hunk, LineKind};

impl BoundComparison {
    pub fn changes_supported(&self, cancel: &AtomicBool) -> Result<bool> {
        self.changes_capability(cancel)
    }

    pub fn changes(
        &self,
        offset: usize,
        cancel: &AtomicBool,
    ) -> super::client::Result<ChangesPage> {
        let page: ChangesPage = self.change_request(
            ComparisonChangesSelection::List {
                offset,
                limit: PAGE_LIMIT,
            },
            cancel,
            |page: &ChangesPage| &page.header,
        )?;
        page.validate(offset)?;
        Ok(page)
    }

    pub fn source_diff(
        &self,
        id: String,
        cancel: &AtomicBool,
    ) -> super::client::Result<SourceDiff> {
        let result: SourceDiff = self.change_request(
            ComparisonChangesSelection::SourceDiff { id: id.clone() },
            cancel,
            |result: &SourceDiff| &result.header,
        )?;
        ensure_diff(&result, &id)?;
        Ok(result)
    }
}

impl ChangesPage {
    fn validate(&self, offset: usize) -> Result<()> {
        ensure!(
            (self.kind.as_str(), self.offset, self.limit, self.returned)
                == ("list", offset, PAGE_LIMIT, self.changes.len()),
            "CLI change summary page metadata disagrees with its records"
        );
        ensure!(
            self.returned <= PAGE_LIMIT,
            "CLI exceeded the requested change page size"
        );
        let end = offset.saturating_add(self.returned);
        let next = (end < self.total).then_some(end);
        ensure!(
            (self.more, self.next_offset) == (next.is_some(), next),
            "CLI returned an invalid change page continuation"
        );
        ensure!(next != Some(offset), "CLI change page did not advance");
        Ok(())
    }
}

fn ensure_diff(diff: &SourceDiff, id: &str) -> Result<()> {
    ensure!(
        diff.kind == "source_diff" && diff.id == id,
        "CLI returned the source diff for another change"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_classifies_only_the_edit_run_at_the_step() {
        let hunk: Hunk = serde_json::from_value(serde_json::json!({
            "before": { "start": 10, "count": 3 },
            "after": { "start": 10, "count": 4 },
            "lines": [
                { "kind": "delete", "text": "old limit" },
                { "kind": "insert", "text": "new limit" },
                { "kind": "context", "text": "validate" },
                { "kind": "insert", "text": "check stock" },
                { "kind": "context", "text": "return" }
            ]
        }))
        .unwrap();

        assert_eq!(hunk.changed_at(false, &(12..=12)), Some(false));
        assert_eq!(hunk.changed_at(false, &(10..=10)), Some(true));
        assert_eq!(hunk.changed_at(true, &(10..=10)), Some(true));
        assert_eq!(hunk.changed_at(false, &(11..=11)), None);
        assert_eq!(hunk.changed_at(true, &(12..=12)), None);
    }
}
