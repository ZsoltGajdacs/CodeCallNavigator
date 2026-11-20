//! Normalized compiler evidence passed from native adapters to publication.
use crate::{discovery::BuildTarget, model::FileFingerprint};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceRole {
    Revision,
    Generated,
    Dependency,
    Watch,
}

#[derive(Clone, Debug)]
pub struct EvidenceInput {
    pub fingerprint: FileFingerprint,
    pub role: EvidenceRole,
}

impl EvidenceInput {
    pub fn new(fingerprint: FileFingerprint, role: EvidenceRole) -> Self {
        Self { fingerprint, role }
    }
}

pub struct AnalyzerDescriptor {
    pub backend: &'static str,
    pub version: String,
    pub mode: &'static str,
}

pub struct TargetEvidence {
    pub target: BuildTarget,
    pub analyzer: AnalyzerDescriptor,
    pub inputs: Vec<EvidenceInput>,
    pub attribution_limit: Option<String>,
}
