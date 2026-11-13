//! Versioned saved-input observations, independent of graph presentation.
use crate::{
    artifact::verification::{InputStatus, Reason},
    query::Error,
};
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const RECONCILE_MILLISECONDS: u64 = 250;
pub const HEARTBEAT_MILLISECONDS: u64 = 5_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    pub kind: CoverageKind,
    /// Every recorded scope was readable/traversable in this observation pass.
    pub complete: bool,
    /// Observations do not certify unrecorded build environment or editor buffers.
    pub certifies_build: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageKind {
    ObservedInputs,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mode {
    pub kind: ModeKind,
    pub limited: bool,
    pub reconcile_milliseconds: u64,
}

impl Default for Mode {
    fn default() -> Self {
        Self {
            kind: ModeKind::VerifiedPolling,
            limited: true,
            reconcile_milliseconds: RECONCILE_MILLISECONDS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeKind {
    VerifiedPolling,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FreshnessObservation {
    pub version: u32,
    pub generation: String,
    pub input_revision: Option<String>,
    pub contract_valid: bool,
    pub input_status: InputStatus,
    pub reasons: Vec<Reason>,
    pub reasons_total: usize,
    pub omitted_reasons: usize,
    pub coverage: Coverage,
    pub mode: Mode,
    pub verification_milliseconds: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamEvent {
    pub version: u32,
    pub sequence: u64,
    #[serde(flatten)]
    pub payload: EventPayload,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventPayload {
    Activity { activity: crate::refresh::Activity },
    Initial { observation: FreshnessObservation },
    Change { observation: FreshnessObservation },
    Heartbeat { observation: FreshnessObservation },
    Failed { error: Error },
}
