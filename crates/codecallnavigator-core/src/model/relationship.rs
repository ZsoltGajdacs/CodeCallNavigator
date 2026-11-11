//! Cross-language relationship meaning, derived without altering analyzer facts.
use super::Edge;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    Call,
    FunctionReference,
    FrameworkLifecycle,
    HttpNavigation,
    #[default]
    Other,
}

impl RelationshipKind {
    pub const ALL: [Self; 5] = [
        Self::Call,
        Self::FunctionReference,
        Self::FrameworkLifecycle,
        Self::HttpNavigation,
        Self::Other,
    ];
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TargetResolution {
    Direct,
    VirtualDispatch,
    Possible,
    #[default]
    Other,
}

impl TargetResolution {
    pub const ALL: [Self; 4] = [
        Self::Direct,
        Self::VirtualDispatch,
        Self::Possible,
        Self::Other,
    ];

    fn from_certainty(certainty: &str) -> Self {
        match certainty {
            "direct" | "proven-direct" => Self::Direct,
            "dispatch" => Self::VirtualDispatch,
            "possible" => Self::Possible,
            _ => Self::Other,
        }
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct RelationshipClassification {
    pub kind: RelationshipKind,
    pub resolution: TargetResolution,
}

impl Edge {
    /// Classification describes static evidence, never proof of runtime execution.
    pub fn classification(&self) -> RelationshipClassification {
        RelationshipClassification {
            kind: relationship_kind(self),
            resolution: TargetResolution::from_certainty(&self.certainty),
        }
    }
}

fn relationship_kind(edge: &Edge) -> RelationshipKind {
    if matches!(edge.kind.as_str(), "http" | "navigation") {
        RelationshipKind::HttpNavigation
    } else if matches!(
        edge.kind.as_str(),
        "methodgroup" | "reference" | "callback" | "_LM" | "react-event" | "react-prop"
    ) {
        RelationshipKind::FunctionReference
    } else if framework_relationship(edge) {
        RelationshipKind::FrameworkLifecycle
    } else if invocation_kind(&edge.kind) {
        RelationshipKind::Call
    } else {
        // In particular, bootstrap and inheritance links do not establish invocation.
        RelationshipKind::Other
    }
}

fn framework_relationship(edge: &Edge) -> bool {
    matches!(
        edge.kind.as_str(),
        "react-render"
            | "react-effect"
            | "react-cleanup"
            | "react-memo"
            | "react-initializer"
            | "react-reducer"
            | "spring-event"
            | "_RIR2"
            | "_CIC2"
            | "_TCID2"
            | "_TCWRID2"
            | "_TSR"
    ) || edge.provenance == "codecallnavigator.mediatr.v1"
        || edge
            .evidence
            .iter()
            .any(|item| item.semantics == "deferred" || item.semantics == "async-boundary")
}

fn invocation_kind(kind: &str) -> bool {
    matches!(
        kind,
        "call"
            | "new"
            | "construct"
            | "ctorinit"
            | "get"
            | "set"
            | "event"
            | "accessor"
            | "STA"
            | "SPE"
            | "VIR"
            | "INT"
            | "DYN"
            | "_ACT_I"
            | "_ACT_C"
            | "_SPR_ACT_I"
            | "_SPR_ACT_C"
            | "_RIR1"
            | "_CIC1"
            | "_TCID1"
            | "_TCWRID1"
            | "_CCS_SPE"
    )
}
