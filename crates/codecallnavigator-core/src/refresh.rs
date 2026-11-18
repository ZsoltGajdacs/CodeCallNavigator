//! Opt-in automatic-refresh activity is volatile and separate from input validity.
use crate::{artifact, discovery::BuildTarget, query::Error};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const VERSION: u32 = 1;
pub const DEBOUNCE_MILLISECONDS: u64 = 300;
pub const SESSION_VERSION: u32 = 1;
pub const OBSERVE_SESSION_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserveSession {
    pub version: u32,
    pub session: SessionRequest,
}

/// Start a session with a fixed selection, including projects without a graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRequest {
    pub version: u32,
    pub project: PathBuf,
    pub roots: Vec<(String, PathBuf)>,
    pub targets: Vec<BuildTarget>,
    pub infer_linked_roots: bool,
    pub expected_generation: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Activity {
    pub version: u32,
    /// Cumulative analysis attempt starts; survives notification coalescing.
    pub runs_started: u64,
    #[serde(flatten)]
    pub phase: Phase,
}

impl Activity {
    pub fn new(phase: Phase) -> Self {
        Self {
            version: VERSION,
            runs_started: 0,
            phase,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Phase {
    Watching,
    Debouncing {
        input_revision: Option<String>,
        remaining_milliseconds: u64,
    },
    Running {
        run: u64,
        input_revision: Option<String>,
        debounce_milliseconds: f64,
        wait_milliseconds: f64,
    },
    Busy {
        run: Option<u64>,
        input_revision: Option<String>,
        retry_milliseconds: u64,
        wait_milliseconds: f64,
    },
    Published {
        run: u64,
        input_revision: Option<String>,
        generation: String,
        roots: Vec<(String, PathBuf)>,
        analysis_milliseconds: f64,
        metrics: serde_json::Value,
    },
    Failed {
        run: u64,
        input_revision: Option<String>,
        error: Error,
        analysis_milliseconds: f64,
    },
    Cancelled {
        run: Option<u64>,
        input_revision: Option<String>,
        reason: CancelReason,
        analysis_milliseconds: f64,
    },
    Unavailable {
        error: Error,
    },
}

impl Phase {
    pub fn validate_publication(&self) -> Result<()> {
        let Self::Published {
            generation,
            roots,
            metrics,
            ..
        } = self
        else {
            return Ok(());
        };
        ensure!(
            generation.len() == 64
                && generation
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
            "Invalid analysis publication digest"
        );
        ensure!(metrics.is_object(), "Invalid analysis publication metrics");
        ensure!(
            roots.iter().any(|(name, _)| name == "repo")
                && roots.iter().all(|(_, path)| path.is_absolute()),
            "Analysis publication omitted absolute project roots"
        );
        artifact::validate_roots(roots)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelReason {
    Superseded,
    UserCancelled,
    Shutdown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Control {
    CancelRefresh { version: u32 },
}
