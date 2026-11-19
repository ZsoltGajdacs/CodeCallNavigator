//! Versioned process boundary; runtime observations never enter graph.json.
use crate::{
    discovery::{BuildIdentity, BuildTarget},
    model::FileFingerprint,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const VERSION: u32 = 2;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Discover {
        version: u32,
        project: PathBuf,
    },
    ClearCache {
        version: u32,
        project: PathBuf,
    },
    PrepareSnapshot {
        version: u32,
        project: PathBuf,
        revision: String,
        #[serde(default)]
        targets: Vec<PathBuf>,
    },
    Analyze {
        version: u32,
        project: PathBuf,
        roots: Vec<(String, PathBuf)>,
        targets: Vec<BuildTarget>,
        /// Desktop preserves compiler-declared sources outside the selected root.
        #[serde(default)]
        infer_linked_roots: bool,
        /// Opt in to typed failures while preserving legacy message-only events.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        failure_version: Option<u32>,
        /// Checked after taking graph.lock, before observing or compiling inputs.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expected_generation: Option<String>,
    },
}

impl Request {
    pub fn version(&self) -> u32 {
        match self {
            Self::Discover { version, .. }
            | Self::ClearCache { version, .. }
            | Self::PrepareSnapshot { version, .. }
            | Self::Analyze { version, .. } => *version,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    Hello {
        version: u32,
    },
    Progress {
        message: String,
    },
    Discovered {
        targets: Vec<BuildTarget>,
    },
    CacheCleared {
        removed_files: u32,
    },
    SnapshotPrepared {
        snapshot: crate::artifact::local_snapshots::SnapshotRecord,
    },
    Complete {
        roots: Vec<(String, PathBuf)>,
        digest: String,
        fingerprints: Vec<FileFingerprint>,
        engine_versions: String,
        metrics: serde_json::Value,
    },
    Failed {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<AnalysisFailureCode>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<TargetFailure>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisFailureCode {
    Busy,
    InputsChanged,
    GenerationChanged,
    Cancelled,
    Failed,
}

impl AnalysisFailureCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::InputsChanged => "inputs_changed",
            Self::GenerationChanged => "generation_changed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

/// Attribution is present only when an operational failure belongs to one build.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetFailure {
    pub target: BuildIdentity,
    pub phase: AnalysisPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisPhase {
    Preparation,
    Compilation,
    Extraction,
    Validation,
}
