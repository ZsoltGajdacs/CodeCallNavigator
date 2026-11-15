//! The result is bound to supplied immutable inputs; query generation/revision stays outside it.
use super::{OccurrencePath, SiteAddress, SourceId, SourceOwner, SourceTable};
use crate::flow::{FlowChart, SourceSpan};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OccurrenceId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionLimit {
    Depth,
    Steps,
    Links,
    SourceBytes,
    Requests,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExpansionStop {
    Unselected,
    Ineligible,
    Unavailable,
    Cycle,
    Limit { limit: ExpansionLimit },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OccurrenceOutcome {
    Root,
    Expanded,
    Stopped {
        reason: ExpansionStop,
        message: String,
    },
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryRefs {
    pub call: Option<usize>,
    pub entry: Option<usize>,
    pub returns: Vec<usize>,
    pub throws: Vec<usize>,
    pub marker: Option<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowOccurrence {
    pub id: OccurrenceId,
    pub symbol: String,
    pub parent: Option<OccurrenceId>,
    pub path: OccurrencePath,
    pub source: Option<SourceId>,
    pub owner: Option<SourceOwner>,
    pub span: SourceSpan,
    pub outcome: OccurrenceOutcome,
    pub boundaries: BoundaryRefs,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginRole {
    Original,
    Call,
    Return,
    Throw,
    Marker,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepOrigin {
    pub occurrence: OccurrenceId,
    pub source: Option<SourceId>,
    pub role: OriginRole,
    /// One structural address for each call in the corresponding chart step.
    pub calls: Vec<SiteAddress>,
}
/// Unexecuted request metadata below a real, visibly stopped ancestor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedSelection {
    pub caller: OccurrencePath,
    pub choice: super::CallChoice,
    pub ancestor: OccurrenceId,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpandedFlow {
    pub version: u32,
    pub root_symbol: String,
    pub chart: FlowChart,
    pub occurrences: Vec<FlowOccurrence>,
    pub sources: SourceTable,
    pub step_origins: Vec<StepOrigin>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_selections: Vec<BlockedSelection>,
}
impl ExpandedFlow {
    pub fn validate(&self) -> anyhow::Result<()> {
        super::validation::validate(self)
    }
}
