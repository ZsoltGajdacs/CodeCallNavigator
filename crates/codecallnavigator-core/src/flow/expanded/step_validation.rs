//! Per-step occurrence ownership, structural calls and exact analyzed-source ranges.
use super::*;
use crate::flow::{FlowStep, SourceSpan};
use anyhow::{Result, ensure};
use std::collections::HashSet;
pub(super) fn validate(result: &ExpandedFlow) -> Result<()> {
    let chart = &result.chart;
    let mut addresses = HashSet::new();
    for (index, step) in chart.steps.iter().enumerate() {
        ensure!(
            step.id == index,
            "Expanded step IDs must be unique chart-local indices"
        );
        let origin = &result.step_origins[index];
        stopped_owner(result, origin, step)?;
        ensure!(
            origin.calls.len() == step.calls.len(),
            "Step call origins are incomplete"
        );
        call_addresses(origin, &mut addresses)?;
        origin_range(result, origin, step.span)?;
        for call in &step.calls {
            origin_range(result, origin, call.span)?;
        }
    }
    Ok(())
}
fn stopped_owner(result: &ExpandedFlow, origin: &StepOrigin, step: &FlowStep) -> Result<()> {
    let occurrence = result
        .occurrences
        .get(origin.occurrence.0)
        .ok_or_else(|| anyhow::anyhow!("Unknown step occurrence"))?;
    if matches!(occurrence.outcome, OccurrenceOutcome::Stopped { .. }) {
        ensure!(
            origin.role == OriginRole::Marker && step.calls.is_empty(),
            "Stopped occurrence can own only a marker without calls"
        );
    }
    Ok(())
}
fn call_addresses<'a>(
    origin: &'a StepOrigin,
    addresses: &mut HashSet<(OccurrenceId, &'a SiteAddress)>,
) -> Result<()> {
    for address in &origin.calls {
        address.validate()?;
        ensure!(
            addresses.insert((origin.occurrence, address)),
            "Duplicate complete call address"
        );
    }
    Ok(())
}
fn origin_range(result: &ExpandedFlow, origin: &StepOrigin, range: SourceSpan) -> Result<()> {
    let occurrence = result
        .occurrences
        .get(origin.occurrence.0)
        .ok_or_else(|| anyhow::anyhow!("Unknown step occurrence"))?;
    ensure!(
        origin.source == occurrence.source,
        "Step origin source differs from its occurrence"
    );
    if let Some(source) = origin.source {
        super::validation::span(result, source, range)?;
        if range != SourceSpan::default() && occurrence.span != SourceSpan::default() {
            ensure!(
                range.start >= occurrence.span.start && range.end <= occurrence.span.end,
                "Step range is outside its callable source"
            );
        }
    } else {
        ensure!(
            range == SourceSpan::default(),
            "Unbound step has a located range"
        );
    }
    Ok(())
}
