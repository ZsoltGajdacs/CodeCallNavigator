//! Opt-in automatic-refresh activity is volatile and separate from input validity.
use crate::query::Error;
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const DEBOUNCE_MILLISECONDS: u64 = 300;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Activity {
    pub version: u32,
    /// Cumulative actual analysis-child starts; survives notification coalescing.
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
