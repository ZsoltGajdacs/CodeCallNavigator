//! Bounded expected facts are derived by the helper from the installed CLI, never the UI graph.
use anyhow::{Context, Result, ensure};
use codecallnavigator::flow::FlowChart;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc, time::Duration};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Configuration {
    pub version: u32,
    pub project: PathBuf,
    pub worktree_line: Option<String>,
    pub expected_viewport: [u32; 2],
    pub timeout_seconds: u64,
    pub initial: Binding,
    pub reloaded: Binding,
    pub cases: Vec<Arc<Case>>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Binding {
    pub comparison_id: String,
    pub base_commit: String,
    pub head_commit: String,
    pub base_digest: String,
    pub head_digest: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CaseKind {
    Changed,
    Unchanged,
    Added,
    Removed,
    Legacy,
    Manual,
    Target,
}
impl CaseKind {
    pub const ORDER: [Self; 7] = [
        Self::Changed,
        Self::Unchanged,
        Self::Added,
        Self::Removed,
        Self::Legacy,
        Self::Manual,
        Self::Target,
    ];
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Case {
    pub kind: CaseKind,
    pub symbol: String,
    pub head_symbol: Option<String>,
    pub before: ExpectedSide,
    pub after: ExpectedSide,
}
#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ExpectedSide {
    Ready {
        symbol: String,
        source: String,
        chart: Box<FlowChart>,
    },
    Missing,
    Unavailable {
        reason: String,
    },
}
impl Configuration {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut config: Self =
            serde_json::from_slice(bytes).context("Invalid comparison acceptance configuration")?;
        config.validate()?;
        config.project = config
            .project
            .canonicalize()
            .context("Cannot open comparison acceptance project")?;
        ensure!(
            config.project.is_dir(),
            "Comparison project must be a directory"
        );
        Ok(config)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            self.version == 2,
            "Unsupported comparison acceptance version"
        );
        ensure!(
            self.project.is_absolute(),
            "Comparison project must be absolute"
        );
        if let Some(line) = &self.worktree_line {
            ensure!(
                bounded(line, 1024),
                "Acceptance worktree source line must be explicit and bounded"
            );
        }
        ensure!(
            self.expected_viewport == [1050, 680],
            "Comparison acceptance requires the minimum 1050x680 viewport"
        );
        ensure!(
            (30..=300).contains(&self.timeout_seconds),
            "Comparison timeout must be 30..300 seconds"
        );
        self.initial.validate()?;
        self.reloaded.validate()?;
        ensure!(
            self.initial.base_commit == self.reloaded.base_commit
                && self.initial.head_commit != self.reloaded.head_commit
                && self.initial.comparison_id != self.reloaded.comparison_id,
            "Selecting the later commit must establish a new comparison binding"
        );
        ensure!(
            self.cases.iter().map(|case| case.kind).eq(CaseKind::ORDER),
            "Comparison requires all seven ordered workflow cases"
        );
        for case in &self.cases {
            case.validate()?;
            case.validate_role()?;
        }
        ensure!(
            self.cases[5].symbol == self.cases[0].symbol,
            "Manual case must preserve the changed before identity"
        );
        for side in [&self.cases[0].before, &self.cases[0].after] {
            ensure!(
                side.calls(&self.cases[6].symbol),
                "Target must be an actual recorded call in both changed charts"
            );
        }
        Ok(())
    }
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }
    pub fn viewport_matches(&self, actual: [f32; 2]) -> bool {
        actual
            .into_iter()
            .zip(self.expected_viewport)
            .all(|(a, b)| (a - b as f32).abs() <= 1.0)
    }
}
fn bounded(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.contains(['\0', '\n', '\r'])
}
fn hex(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
impl Binding {
    fn validate(&self) -> Result<()> {
        for digest in [&self.comparison_id, &self.base_digest, &self.head_digest] {
            ensure!(
                digest.len() == 64 && hex(digest),
                "Acceptance digest must be hexadecimal"
            );
        }
        for commit in [&self.base_commit, &self.head_commit] {
            ensure!(hex(commit), "Acceptance commit must be bounded hexadecimal");
        }
        Ok(())
    }
}
impl Case {
    fn validate(&self) -> Result<()> {
        ensure!(
            bounded(&self.symbol, 16384),
            "Acceptance symbol must be explicit and bounded"
        );
        if let Some(head) = &self.head_symbol {
            ensure!(
                bounded(head, 16384) && head != &self.symbol,
                "Manual counterpart must be explicit and distinct"
            );
        }
        ensure!(
            self.head_symbol.is_some() == (self.kind == CaseKind::Manual),
            "Only the manual case selects a counterpart"
        );
        self.before.validate(&self.symbol)?;
        self.after
            .validate(self.head_symbol.as_deref().unwrap_or(&self.symbol))?;
        Ok(())
    }
}
impl Case {
    fn validate_role(&self) -> Result<()> {
        let ready = matches!(
            (&self.before, &self.after),
            (ExpectedSide::Ready { .. }, ExpectedSide::Ready { .. })
        );
        let equality = self.before.equality(&self.after)?;
        let correct = match self.kind {
            CaseKind::Added => matches!(
                (&self.before, &self.after),
                (ExpectedSide::Missing, ExpectedSide::Ready { .. })
            ),
            CaseKind::Removed => matches!(
                (&self.before, &self.after),
                (ExpectedSide::Ready { .. }, ExpectedSide::Missing)
            ),
            CaseKind::Legacy => {
                matches!((&self.before,&self.after),(ExpectedSide::Unavailable{reason},ExpectedSide::Ready{..}) if reason=="missing_function")
            }
            CaseKind::Changed => matches!(
                equality,
                Some(Equality {
                    source: false,
                    chart: false
                })
            ),
            CaseKind::Unchanged => matches!(
                equality,
                Some(Equality {
                    source: true,
                    chart: true
                })
            ),
            CaseKind::Manual | CaseKind::Target => ready,
        };
        ensure!(
            correct,
            "Expected sides do not prove the named {:?} case",
            self.kind
        );
        Ok(())
    }
}
struct Equality {
    source: bool,
    chart: bool,
}
impl ExpectedSide {
    fn equality(&self, other: &Self) -> Result<Option<Equality>> {
        match (self, other) {
            (
                Self::Ready {
                    source: a,
                    chart: x,
                    ..
                },
                Self::Ready {
                    source: b,
                    chart: y,
                    ..
                },
            ) => Ok(Some(Equality {
                source: a == b,
                chart: serde_json::to_value(x)? == serde_json::to_value(y)?,
            })),
            _ => Ok(None),
        }
    }
    fn calls(&self, symbol: &str) -> bool {
        match self {
            Self::Ready { chart, .. } => chart
                .steps
                .iter()
                .flat_map(|step| &step.calls)
                .flat_map(|site| &site.targets)
                .any(|target| target.kind == "call" && target.symbol == symbol),
            _ => false,
        }
    }
    fn validate(&self, selected: &str) -> Result<()> {
        match self {
            Self::Ready {
                symbol,
                source,
                chart,
            } => {
                ensure!(
                    symbol == selected && chart.symbol == *symbol,
                    "Expected chart must belong to the exact selected ID"
                );
                ensure!(
                    !source.is_empty() && source.len() <= 512 * 1024,
                    "Expected source must be exact and bounded"
                );
                ensure!(
                    chart.steps.len() <= 10000 && chart.links.len() <= 30000,
                    "Expected chart exceeds native acceptance budget"
                );
            }
            Self::Unavailable { reason } => {
                ensure!(bounded(reason, 128), "Unavailable reason must be bounded")
            }
            Self::Missing => (),
        }
        Ok(())
    }
}
#[cfg(test)]
#[path = "config_tests.rs"]
pub(crate) mod tests;
