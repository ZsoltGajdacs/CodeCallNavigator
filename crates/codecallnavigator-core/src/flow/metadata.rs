//! Validate positive compiler claims against their actual structural owner.
use super::{
    CallSite, ExecutionTiming, Expression, FunctionFlowIR, InvocationDispatch,
    parts::{Owner, Part, Parts},
};
use anyhow::{Result, ensure};
pub(crate) fn validate_metadata(function: &FunctionFlowIR) -> Result<()> {
    for part in Parts::new(function) {
        validate_part(part)?;
    }
    Ok(())
}
pub(super) fn validate_part(part: Part<'_>) -> Result<()> {
    if let Part::Call { site, owner } = part {
        if site.invocation.isolated_ordered {
            validate_isolation(site, owner)?;
        }
        if site.invocation.dispatch == InvocationDispatch::Direct {
            validate_direct(site)?;
        }
    }
    Ok(())
}
fn validate_isolation(site: &CallSite, owner: Owner<'_>) -> Result<()> {
    ensure!(
        site.invocation.timing == ExecutionTiming::Synchronous,
        "Isolated invocation requires synchronous activation"
    );
    let Owner::Value(Expression::Value {
        span,
        calls,
        regions,
        ..
    }) = owner
    else {
        anyhow::bail!("Isolated invocation must belong to a Value expression");
    };
    ensure!(
        calls.len() == 1 && regions.is_empty(),
        "Isolated invocation requires one own call and no expression regions"
    );
    ensure!(
        span.start < span.end && *span == site.span,
        "Isolated invocation requires meaningful equal call/value spans"
    );
    Ok(())
}
fn validate_direct(site: &CallSite) -> Result<()> {
    let mut targets = site.targets.iter().filter(|target| target.kind == "call");
    if let Some(first) = targets.next() {
        ensure!(
            targets.all(|target| target.symbol == first.symbol),
            "Direct invocation has multiple distinct native call targets"
        );
    }
    Ok(())
}
