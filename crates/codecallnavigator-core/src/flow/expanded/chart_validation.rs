//! Validate graph references and bind every visible step to its occurrence/source.
use super::*;
use anyhow::{Result, ensure};

pub(super) fn validate(result: &ExpandedFlow) -> Result<()> {
    let chart = &result.chart;
    ensure!(
        chart.entry < chart.steps.len(),
        "Invalid expanded chart entry"
    );
    ensure!(
        result.step_origins.len() == chart.steps.len(),
        "Expanded chart origins are incomplete"
    );
    super::step_validation::validate(result)?;
    for link in &chart.links {
        ensure!(
            link.from < chart.steps.len() && link.to < chart.steps.len(),
            "Invalid expanded chart link"
        );
    }
    for region in &chart.regions {
        ensure!(region.entry < chart.steps.len(), "Invalid region entry");
        ensure!(
            region.steps.iter().all(|step| *step < chart.steps.len()),
            "Invalid region step"
        );
    }
    for occurrence in &result.occurrences {
        boundaries(result, occurrence)?;
    }
    ensure!(
        result.occurrences[0].boundaries.entry == Some(chart.entry),
        "Root entry boundary mismatch"
    );
    Ok(())
}
fn reference(
    result: &ExpandedFlow,
    step: usize,
    owner: OccurrenceId,
    role: Option<OriginRole>,
) -> Result<()> {
    let origin = result
        .step_origins
        .get(step)
        .ok_or_else(|| anyhow::anyhow!("Unknown boundary step"))?;
    ensure!(
        origin.occurrence == owner,
        "Boundary step has a different occurrence owner"
    );
    if let Some(role) = role {
        ensure!(origin.role == role, "Boundary role mismatch");
    }
    Ok(())
}
fn boundaries(result: &ExpandedFlow, occurrence: &FlowOccurrence) -> Result<()> {
    let refs = &occurrence.boundaries;
    caller_boundaries(result, occurrence)?;
    completion_boundaries(result, occurrence)?;
    marker_reference(result, occurrence)?;
    stopped_marker(result, occurrence)?;
    if occurrence.outcome == OccurrenceOutcome::Expanded {
        ensure!(
            refs.entry.is_some() && refs.call.is_some(),
            "Expanded occurrence lacks boundaries"
        );
    }
    Ok(())
}

fn caller_boundaries(result: &ExpandedFlow, occurrence: &FlowOccurrence) -> Result<()> {
    let refs = &occurrence.boundaries;
    if let Some(entry) = refs.entry {
        reference(result, entry, occurrence.id, Some(OriginRole::Original))?;
    }
    if let Some(call) = refs.call {
        let parent = occurrence
            .parent
            .ok_or_else(|| anyhow::anyhow!("Root cannot have a caller boundary"))?;
        reference(result, call, parent, Some(OriginRole::Call))?;
    }
    Ok(())
}
fn completion_boundaries(result: &ExpandedFlow, occurrence: &FlowOccurrence) -> Result<()> {
    let refs = &occurrence.boundaries;
    for step in &refs.returns {
        reference(result, *step, occurrence.id, Some(OriginRole::Return))?;
    }
    for step in &refs.throws {
        reference(result, *step, occurrence.id, Some(OriginRole::Throw))?;
    }
    Ok(())
}
fn stopped_marker(result: &ExpandedFlow, occurrence: &FlowOccurrence) -> Result<()> {
    let OccurrenceOutcome::Stopped { message, .. } = &occurrence.outcome else {
        ensure!(
            occurrence.boundaries.marker.is_none(),
            "Built occurrence claims a stop marker"
        );
        return Ok(());
    };
    ensure!(
        !message.trim().is_empty(),
        "Stopped occurrence has no explanation"
    );
    let marker = occurrence
        .boundaries
        .marker
        .ok_or_else(|| anyhow::anyhow!("Stopped occurrence has no visible marker"))?;
    ensure!(
        !result.chart.steps[marker].label.trim().is_empty(),
        "Stop marker has no visible label"
    );
    ensure!(
        occurrence.boundaries.entry.is_none()
            && occurrence.boundaries.returns.is_empty()
            && occurrence.boundaries.throws.is_empty(),
        "Stopped occurrence claims an assembled body"
    );
    Ok(())
}

fn marker_reference(result: &ExpandedFlow, occurrence: &FlowOccurrence) -> Result<()> {
    if let Some(marker) = occurrence.boundaries.marker {
        let owner = result
            .step_origins
            .get(marker)
            .ok_or_else(|| anyhow::anyhow!("Unknown marker boundary"))?
            .occurrence;
        ensure!(
            owner == occurrence.id || Some(owner) == occurrence.parent,
            "Marker has an unrelated owner"
        );
        reference(result, marker, owner, Some(OriginRole::Marker))?;
    }
    Ok(())
}
