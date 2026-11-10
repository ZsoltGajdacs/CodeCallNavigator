//! Versioned process boundary; runtime observations never enter graph.json.
use crate::{discovery::BuildTarget, model::FileFingerprint};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Discover {
        version: u32,
        project: PathBuf,
    },
    Analyze {
        version: u32,
        project: PathBuf,
        roots: Vec<(String, PathBuf)>,
        targets: Vec<BuildTarget>,
        /// Desktop preserves compiler-declared sources outside the selected root.
        #[serde(default)]
        infer_linked_roots: bool,
    },
}

impl Request {
    pub fn version(&self) -> u32 {
        match self {
            Self::Discover { version, .. } | Self::Analyze { version, .. } => *version,
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
    Complete {
        roots: Vec<(String, PathBuf)>,
        digest: String,
        fingerprints: Vec<FileFingerprint>,
        engine_versions: String,
        metrics: serde_json::Value,
    },
    Failed {
        message: String,
    },
}
