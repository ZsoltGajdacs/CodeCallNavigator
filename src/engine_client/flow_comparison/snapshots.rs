//! Read-only snapshot and graph queries use the public engine wire contract.
use super::Header;
use crate::engine_client::QueryContext;
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator::{
    artifact::local_snapshots::SnapshotRecord,
    protocol::{self, Event, Request},
    query::Query,
};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, ops::Range, path::PathBuf, sync::atomic::AtomicBool};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComparisonRefs {
    pub base: String,
    pub head: String,
    pub base_snapshot: Option<String>,
    pub head_snapshot: Option<String>,
}

impl ComparisonRefs {
    pub(super) fn validate(&self, header: &Header) -> Result<()> {
        header.validate(&self.base, &self.head)?;
        for (requested, endpoint) in [
            (&self.base_snapshot, &header.base),
            (&self.head_snapshot, &header.head),
        ] {
            if let Some(id) = requested {
                ensure!(
                    endpoint
                        .snapshot
                        .as_ref()
                        .is_some_and(|snapshot| &snapshot.id == id),
                    "CLI returned another selected snapshot"
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct SnapshotStatus {
    pub cache_path: PathBuf,
    pub commit: String,
    pub snapshots: Vec<SnapshotRecord>,
}

pub struct SnapshotPreparation {
    pub revision: String,
    pub targets: Vec<PathBuf>,
}

#[derive(Default)]
pub struct GraphCursor {
    pub offset: usize,
    pub binding: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GraphPage {
    #[serde(flatten)]
    pub header: Header,
    pub graph_equal: bool,
    pub metadata_equal: bool,
    pub summary: BTreeMap<String, ChangeSummary>,
    #[serde(deserialize_with = "format_changes")]
    pub changes: Vec<GraphChange>,
    pub offset: usize,
    pub limit: usize,
    pub returned: usize,
    pub total: usize,
    pub more: bool,
    pub next_offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeSummary {
    pub changed_entities: usize,
    pub records: usize,
    pub added: usize,
    pub removed: usize,
    pub replaced: usize,
}

pub struct GraphChange {
    pub summary: String,
    pub document: String,
    lines: Vec<Range<usize>>,
}

impl std::fmt::Debug for GraphChange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.summary, formatter)
    }
}

impl GraphChange {
    fn from_value(change: Value) -> serde_json::Result<Self> {
        let summary = ["operation", "section", "entity", "path"]
            .into_iter()
            .filter_map(|field| change[field].as_str().filter(|value| !value.is_empty()))
            .collect::<Vec<_>>()
            .join(" · ");
        let document = serde_json::to_string_pretty(&change)?;
        let lines = display_lines(&document);
        Ok(Self {
            summary,
            document,
            lines,
        })
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line(&self, index: usize) -> Option<&str> {
        self.lines
            .get(index)
            .and_then(|range| self.document.get(range.clone()))
    }
}

fn display_lines(document: &str) -> Vec<Range<usize>> {
    const COLUMNS: usize = 100;
    let mut rows = Vec::new();
    let mut start = 0;
    for line in document.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let mut row = 0;
        for (column, (index, _)) in content.char_indices().enumerate() {
            if column > 0 && column.is_multiple_of(COLUMNS) {
                rows.push(start + row..start + index);
                row = index;
            }
        }
        rows.push(start + row..start + content.len());
        start += line.len();
    }
    rows
}

fn format_changes<'de, D>(deserializer: D) -> std::result::Result<Vec<GraphChange>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Vec::<Value>::deserialize(deserializer)?
        .into_iter()
        .map(|change| GraphChange::from_value(change).map_err(serde::de::Error::custom))
        .collect()
}

impl QueryContext {
    pub fn require_local_snapshots(&self, cancel: &AtomicBool) -> Result<()> {
        ensure!(
            self.client.local_snapshots_supported(cancel)?,
            "unsupported_capability: update the CLI to use local snapshots"
        );
        Ok(())
    }
    pub fn snapshot_status(&self, revision: String, cancel: &AtomicBool) -> Result<SnapshotStatus> {
        self.require_local_snapshots(cancel)?;
        let result: SnapshotStatus = self.request(Query::SnapshotStatus { revision }, cancel)?;
        ensure!(
            result.cache_path.is_absolute(),
            "CLI returned an invalid cache path"
        );
        ensure!(
            matches!(result.commit.len(), 40 | 64)
                && result.commit.bytes().all(|byte| byte.is_ascii_hexdigit())
                && result
                    .snapshots
                    .iter()
                    .all(|snapshot| snapshot.identity.commit == result.commit),
            "CLI returned inconsistent snapshot status"
        );
        Ok(result)
    }

    pub fn prepare_local_snapshot(
        &self,
        selection: SnapshotPreparation,
        cancel: &AtomicBool,
        progress: &dyn Fn(String),
    ) -> Result<SnapshotRecord> {
        self.require_local_snapshots(cancel)?;
        let request = Request::PrepareSnapshot {
            version: protocol::VERSION,
            project: self.descriptor.project.clone(),
            revision: selection.revision,
            targets: selection.targets,
        };
        match super::super::exchange(self.client.runtime(), &request, cancel, progress)? {
            Event::SnapshotPrepared { snapshot } => Ok(snapshot),
            _ => bail!("Engine returned an unexpected result for snapshot preparation"),
        }
    }

    pub fn comparison_graph_page(
        &self,
        references: &ComparisonRefs,
        cursor: GraphCursor,
        cancel: &AtomicBool,
    ) -> Result<GraphPage> {
        const LIMIT: usize = 50;
        ensure!(
            self.client.graph_comparison_supported(cancel)?,
            "unsupported_capability: update the CLI to compare graph differences"
        );
        if references.base_snapshot.is_some() || references.head_snapshot.is_some() {
            self.require_local_snapshots(cancel)?;
        }
        let response = self.response(
            Query::Compare {
                base: references.base.clone(),
                head: references.head.clone(),
                base_snapshot: references.base_snapshot.clone(),
                head_snapshot: references.head_snapshot.clone(),
                offset: cursor.offset,
                limit: LIMIT,
                expected_comparison: cursor.binding.clone(),
            },
            cancel,
        )?;
        let page: GraphPage =
            serde_json::from_value(response.result.context("CLI omitted graph differences")?)?;
        references.validate(&page.header)?;
        ensure!(
            cursor
                .binding
                .is_none_or(|id| id == page.header.comparison_id)
                && response.generation.as_deref() == Some(&page.header.head.digest),
            "CLI returned another graph comparison binding"
        );
        page.validate_page(cursor.offset, LIMIT)?;
        Ok(page)
    }
}

impl GraphPage {
    fn validate_page(&self, offset: usize, limit: usize) -> Result<()> {
        ensure!(
            self.offset == offset
                && self.limit == limit
                && self.returned == self.changes.len()
                && self.returned <= limit,
            "CLI returned an invalid graph comparison page"
        );
        let end = offset
            .checked_add(self.returned)
            .context("Invalid graph page length")?;
        ensure!(
            self.more == (end < self.total)
                && self.next_offset == self.more.then_some(end)
                && (!self.more || end > offset),
            "CLI returned an invalid graph comparison cursor"
        );
        Ok(())
    }
}
