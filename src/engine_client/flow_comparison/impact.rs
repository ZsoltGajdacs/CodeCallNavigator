//! Read-only potential impact from the retained base graph.
use super::{ComparisonRefs, Endpoint, EndpointKind, model};
use crate::{
    engine_client::{QueryContext, capabilities},
    runtime::Runtime,
};
use anyhow::{Context, Result, ensure};
use codecallnavigator::query::Query;
use serde::Deserialize;
use std::{sync::atomic::AtomicBool, time::Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactRefs {
    pub base: String,
    pub head: String,
    pub base_snapshot: Option<String>,
}

impl From<ComparisonRefs> for ImpactRefs {
    fn from(references: ComparisonRefs) -> Self {
        Self {
            base: references.base,
            head: references.head,
            base_snapshot: references.base_snapshot,
        }
    }
}

#[derive(Debug)]
pub struct ImpactPreview {
    pub base: Endpoint,
    pub head: Head,
    pub coverage: Coverage,
    pub rows: Vec<ImpactRow>,
}

#[derive(Debug, Deserialize)]
pub struct Head {
    pub requested_ref: String,
    pub commit: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Coverage {
    pub tracked_only: bool,
    pub changed_paths: usize,
    pub mapped_ranges: usize,
    pub unmapped_ranges: usize,
    pub truncated: bool,
}

#[derive(Debug)]
pub enum ImpactRow {
    Range {
        label: String,
        unresolved: Option<String>,
    },
    Symbol {
        id: String,
        label: String,
    },
    Caller {
        label: String,
        path: String,
        evidence: String,
        possible: bool,
    },
}

#[derive(Deserialize)]
struct WirePreview {
    version: u32,
    base: Endpoint,
    head: Head,
    coverage: Coverage,
    records: Vec<WireRange>,
}

#[derive(Deserialize)]
struct WireRange {
    path: String,
    status: String,
    old_start: Option<u32>,
    old_count: Option<u32>,
    symbols: Vec<WireSymbol>,
    unresolved: Option<String>,
}

#[derive(Deserialize)]
struct WireSymbol {
    id: String,
    label: String,
    line: u32,
    coverage: SymbolCoverage,
    entry_kind: Option<EntryKind>,
    callers: Vec<WireCaller>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum SymbolCoverage {
    AnalyzedRange,
    DeclarationLine,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum EntryKind {
    ApplicationStart,
    Framework,
    Candidate,
}

impl EntryKind {
    fn label(&self) -> &'static str {
        match self {
            Self::ApplicationStart => "application start",
            Self::Framework => "framework entry",
            Self::Candidate => "entry candidate",
        }
    }
}

#[derive(Deserialize)]
struct WireCaller {
    id: String,
    label: String,
    path: Vec<String>,
    possible: bool,
    entry_kind: Option<EntryKind>,
    edges: Vec<WireEvidence>,
}

#[derive(Deserialize)]
struct WireEvidence {
    kind: String,
    certainty: String,
    provenance: String,
}

pub fn installed_impact_preview(runtime: &Runtime, cancel: &AtomicBool) -> Result<bool> {
    Ok(capabilities::check(runtime, cancel, Instant::now())?.impact_preview)
}

impl QueryContext {
    pub fn impact_preview(&self, refs: ImpactRefs, cancel: &AtomicBool) -> Result<ImpactPreview> {
        ensure!(
            self.client.impact_preview_supported(cancel)?,
            "unsupported_capability: update the CLI to preview changed-symbol impact"
        );
        let response = self.response(
            Query::ImpactPreview {
                base: refs.base.clone(),
                head: refs.head.clone(),
                base_snapshot: refs.base_snapshot.clone(),
            },
            cancel,
        )?;
        let preview: WirePreview =
            serde_json::from_value(response.result.context("CLI omitted the impact preview")?)
                .context("Invalid CLI impact preview result")?;
        preview.validate(&refs, response.generation.as_deref())?;
        Ok(preview.into_preview())
    }
}

impl WirePreview {
    fn validate(&self, refs: &ImpactRefs, generation: Option<&str>) -> Result<()> {
        ensure!(self.version == 1, "Unsupported CLI impact preview version");
        ensure!(
            self.base.kind == EndpointKind::Commit
                && self.base.requested_ref == refs.base
                && self.head.requested_ref == refs.head,
            "CLI returned different impact preview references"
        );
        model::endpoint(&self.base)?;
        ensure!(
            generation.is_none_or(|digest| digest == self.base.digest),
            "CLI impact preview generation differs from the base graph"
        );
        if let Some(id) = &refs.base_snapshot {
            ensure!(
                self.base
                    .snapshot
                    .as_ref()
                    .is_some_and(|snapshot| &snapshot.id == id),
                "CLI used another impact preview base snapshot"
            );
        }
        validate_head(&self.head)
    }

    fn into_preview(self) -> ImpactPreview {
        let mut rows = Vec::new();
        for range in self.records {
            let label = range.label();
            rows.push(ImpactRow::Range {
                label,
                unresolved: range.unresolved.map(|reason| reason.replace('_', " ")),
            });
            for symbol in range.symbols {
                rows.push(ImpactRow::Symbol {
                    label: symbol.label(),
                    id: symbol.id,
                });
                rows.extend(symbol.callers.into_iter().map(WireCaller::into_row));
            }
        }
        ImpactPreview {
            base: self.base,
            head: self.head,
            coverage: self.coverage,
            rows,
        }
    }
}

impl WireSymbol {
    fn label(&self) -> String {
        let coverage = match self.coverage {
            SymbolCoverage::AnalyzedRange => "analyzed range",
            SymbolCoverage::DeclarationLine => "declaration line only",
        };
        let label = format!("{} · base line {} · {coverage}", self.label, self.line);
        with_entry(label, self.entry_kind.as_ref())
    }
}

impl WireCaller {
    fn into_row(self) -> ImpactRow {
        let path = if self.path.is_empty() {
            self.id
        } else {
            self.path.join(" → ")
        };
        let mut evidence = path.clone();
        for edge in self.edges {
            use std::fmt::Write as _;
            let _ = write!(
                evidence,
                "\n{} · {} · {}",
                edge.kind, edge.certainty, edge.provenance
            );
        }
        ImpactRow::Caller {
            label: with_entry(self.label, self.entry_kind.as_ref()),
            path,
            evidence,
            possible: self.possible,
        }
    }
}

fn with_entry(mut label: String, entry: Option<&EntryKind>) -> String {
    if let Some(entry) = entry {
        label.push_str(" · ");
        label.push_str(entry.label());
    }
    label
}

impl WireRange {
    fn label(&self) -> String {
        let lines = match (self.old_start, self.old_count) {
            (Some(start), Some(1)) => format!("base line {start}"),
            (Some(start), Some(count)) if count > 1 => {
                format!("base lines {start}–{}", start.saturating_add(count - 1))
            }
            _ => "no base lines".into(),
        };
        format!("{} · {} · {lines}", self.path, self.status)
    }
}

fn validate_head(head: &Head) -> Result<()> {
    let valid = if head.requested_ref == "WORKTREE" {
        head.commit.is_none()
    } else {
        head.commit.as_ref().is_some_and(|commit| {
            matches!(commit.len(), 40 | 64) && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    };
    ensure!(
        valid,
        "CLI returned an invalid resolved impact preview head"
    );
    Ok(())
}
