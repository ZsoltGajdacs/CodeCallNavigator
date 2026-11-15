//! Explicit occurrence-path selections over immutable analyzed function/source inputs.
mod validation;
use super::{CallChoice, ExpansionLimits, OccurrencePath};
use crate::flow::FunctionFlowIR;
#[derive(Clone, Copy)]
pub struct FunctionSource<'a> {
    pub function: &'a FunctionFlowIR,
    pub content: &'a str,
}
#[derive(Clone, Debug)]
pub(crate) struct RequestedCall {
    pub caller: OccurrencePath,
    pub choice: CallChoice,
}
#[derive(Clone, Debug)]
pub struct ExpansionRequest {
    root: String,
    selections: Vec<RequestedCall>,
    limits: ExpansionLimits,
}
impl ExpansionRequest {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            selections: vec![],
            limits: ExpansionLimits::default(),
        }
    }
    pub fn select_root(self, choice: CallChoice) -> Self {
        self.select(OccurrencePath::default(), choice)
    }
    /// Select one exact call in a selected caller occurrence. Requests are bounded
    /// to 128 selections, depth 32 and 1 MiB of cumulative encoded identity data.
    pub fn select(mut self, caller: OccurrencePath, choice: CallChoice) -> Self {
        self.selections.push(RequestedCall { caller, choice });
        self
    }
    /// Budget fallbacks demote the active selected child, or the latest successful
    /// root graft for later caller work. This does not maximize the expanded subset.
    pub fn with_limits(mut self, limits: ExpansionLimits) -> Self {
        self.limits = limits;
        self
    }
    /// Validate execution limits and the bounded structural-selection envelope
    /// before adapters perform source lookup or decode selected compiler facts.
    pub fn validate(&self) -> anyhow::Result<()> {
        validation::validate(self)
    }
    pub(crate) fn limits(&self) -> ExpansionLimits {
        self.limits
    }
    pub(crate) fn root(&self) -> &str {
        &self.root
    }
    pub(crate) fn selections(&self) -> &[RequestedCall] {
        &self.selections
    }
}
