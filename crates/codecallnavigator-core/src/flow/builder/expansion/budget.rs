//! Typed failures retain attribution before an owned staged run is discarded.
use crate::flow::{ExpansionLimit, ExpansionLimits};
use anyhow::Result;
use std::fmt;
#[derive(Clone, Copy, Debug)]
pub(super) struct BudgetExceeded {
    pub limit: ExpansionLimit,
    pub candidate: Option<usize>,
}
impl fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Expanded flow exceeds {:?} allowance", self.limit)
    }
}
impl std::error::Error for BudgetExceeded {}
#[derive(Clone, Copy)]
pub(in crate::flow::builder) struct Allowance {
    pub limits: ExpansionLimits,
    pub candidate: Option<usize>,
}
impl Allowance {
    pub fn root(limits: ExpansionLimits) -> Self {
        Self {
            limits,
            candidate: None,
        }
    }
    pub fn child(limits: ExpansionLimits, request: usize) -> Self {
        Self {
            limits,
            candidate: Some(request),
        }
    }
    pub fn check(self, limit: ExpansionLimit, next: u64) -> Result<()> {
        let maximum = match limit {
            ExpansionLimit::Depth => self.limits.depth as u64,
            ExpansionLimit::Requests => self.limits.requests as u64,
            ExpansionLimit::Steps => self.limits.steps as u64,
            ExpansionLimit::Links => self.limits.links as u64,
            ExpansionLimit::SourceBytes => self.limits.source_bytes,
        };
        if next > maximum {
            return Err(BudgetExceeded {
                limit,
                candidate: self.candidate,
            }
            .into());
        }
        Ok(())
    }
}
pub(super) fn attribute(error: anyhow::Error, candidate: usize) -> anyhow::Error {
    let Some(failure) = error.downcast_ref::<BudgetExceeded>().copied() else {
        return error;
    };
    if failure.candidate.is_some() {
        return error;
    }
    BudgetExceeded {
        candidate: Some(candidate),
        ..failure
    }
    .into()
}
