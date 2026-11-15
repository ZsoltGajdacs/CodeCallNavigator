//! Positive compiler facts authorize activation; display certainty never grants it.
use crate::flow::{
    parts::{Part, Parts},
    *,
};
pub(super) struct Stop {
    pub reason: ExpansionStop,
    pub message: &'static str,
}
impl Stop {
    pub fn new(reason: ExpansionStop, message: &'static str) -> Self {
        Self { reason, message }
    }
}
pub(super) fn activation(site: &CallSite, target: &CallTarget, deferred: bool) -> Option<Stop> {
    let actual_call = target.kind == "call" && target.certainty != "reference";
    if deferred || !actual_call {
        return Some(Stop::new(
            ExpansionStop::Ineligible,
            "Reference, non-call, or deferred activation",
        ));
    }
    if !proven_activation(site) {
        return Some(Stop::new(
            ExpansionStop::Ineligible,
            "Compiler facts do not prove isolated synchronous activation",
        ));
    }
    None
}
fn proven_activation(site: &CallSite) -> bool {
    site.invocation.timing == ExecutionTiming::Synchronous
        && site.invocation.isolated_ordered
        && site.invocation.dispatch != InvocationDispatch::Unknown
}
pub(super) fn body(input: Option<FunctionSource<'_>>) -> Option<Stop> {
    let Some(input) = input else {
        return Some(Stop::new(
            ExpansionStop::Unavailable,
            "Saved callee body is unavailable",
        ));
    };
    if input.function.execution != ExecutionTiming::Synchronous {
        return Some(Stop::new(
            ExpansionStop::Ineligible,
            "Callee whole-body execution is not synchronous",
        ));
    }
    if unsupported(input.function) {
        return Some(Stop::new(
            ExpansionStop::Ineligible,
            "Callee contains unsupported control flow",
        ));
    }
    None
}
fn unsupported(function: &FunctionFlowIR) -> bool {
    Parts::new(function).any(|part| {
        matches!(
            part,
            Part::Statement(Statement {
                kind: StatementKind::Unsupported { .. },
                ..
            })
        )
    })
}
