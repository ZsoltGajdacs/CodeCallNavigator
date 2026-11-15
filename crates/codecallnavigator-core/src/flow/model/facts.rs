//! Compiler-owned facts; absence is uncertainty, never evidence for synchronous expansion.
use serde::{Deserialize, Serialize};
/// For a callable this describes the whole body's execution, not caller activation.
/// Async/mixed bodies stay Unknown; compiler-proven iterator bodies are Deferred.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTiming {
    #[default]
    Unknown,
    Synchronous,
    Deferred,
}
impl ExecutionTiming {
    pub(super) fn is_unknown(&self) -> bool {
        *self == Self::Unknown
    }
}
/// Dispatch classification is independent of target certainty/count and survives pruning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationDispatch {
    #[default]
    Unknown,
    Direct,
    Virtual,
    Candidate,
}
impl InvocationDispatch {
    pub(super) fn is_unknown(&self) -> bool {
        *self == Self::Unknown
    }
}
/// Activation timing at this site and the callee's whole-body execution are independent.
/// Isolation is a positive compiler proof of one ordered final invocation, not a heuristic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InvocationFacts {
    #[serde(skip_serializing_if = "ExecutionTiming::is_unknown")]
    pub timing: ExecutionTiming,
    #[serde(skip_serializing_if = "InvocationDispatch::is_unknown")]
    pub dispatch: InvocationDispatch,
    #[serde(skip_serializing_if = "is_false")]
    pub isolated_ordered: bool,
}
fn is_false(value: &bool) -> bool {
    !*value
}
impl InvocationFacts {
    pub(super) fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}
