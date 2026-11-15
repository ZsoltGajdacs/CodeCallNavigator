//! Result invariants only: no eligibility, target lookup or source-text inference.
use super::*;
use crate::flow::SourceSpan;
use anyhow::{Result, ensure};
use std::collections::HashSet;

pub(super) fn validate(result: &ExpandedFlow) -> Result<()> {
    ensure!(result.version == 1, "Unsupported expanded flow version");
    ensure!(
        !result.root_symbol.is_empty(),
        "Expanded flow has no root symbol"
    );
    result.sources.validate()?;
    let root = result
        .occurrences
        .first()
        .ok_or_else(|| anyhow::anyhow!("Expanded flow has no root occurrence"))?;
    ensure!(
        root.symbol == result.root_symbol && result.chart.symbol == result.root_symbol,
        "Root identity mismatch"
    );
    ensure!(
        root.parent.is_none() && root.path.0.is_empty() && root.outcome == OccurrenceOutcome::Root,
        "Invalid root occurrence"
    );
    ensure!(root.source.is_some(), "Root occurrence has no source");
    ensure!(root.span == result.chart.span, "Root chart range mismatch");
    let snapshot = result
        .sources
        .get(root.source.unwrap())
        .ok_or_else(|| anyhow::anyhow!("Unknown root source"))?;
    ensure!(
        result.chart.file == snapshot.owner.file,
        "Root chart source mismatch"
    );
    let mut paths = HashSet::new();
    for (index, occurrence) in result.occurrences.iter().enumerate() {
        ensure!(
            occurrence.id == OccurrenceId(index),
            "Occurrence IDs must be unique result-local indices"
        );
        ensure!(
            paths.insert(&occurrence.path),
            "Duplicate complete occurrence address"
        );
        occurrence_source(result, occurrence)?;
        parent_path(result, occurrence)?;
    }
    super::chart_validation::validate(result)?;
    super::blocked::validate(result)
}
fn occurrence_source(result: &ExpandedFlow, occurrence: &FlowOccurrence) -> Result<()> {
    ensure!(!occurrence.symbol.is_empty(), "Occurrence has no symbol");
    if let Some(owner) = &occurrence.owner {
        owner.validate()?;
    }
    if let Some(id) = occurrence.source {
        let source = result
            .sources
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Unknown occurrence source"))?;
        ensure!(
            occurrence.owner.as_ref() == Some(&source.owner),
            "Occurrence source owner mismatch"
        );
        span(result, id, occurrence.span)?;
    } else {
        ensure!(
            matches!(occurrence.outcome, OccurrenceOutcome::Stopped { .. }),
            "Built occurrence has no source"
        );
        ensure!(
            occurrence.span == SourceSpan::default(),
            "Unbound occurrence has a located range"
        );
    }
    Ok(())
}
fn parent_path(result: &ExpandedFlow, occurrence: &FlowOccurrence) -> Result<()> {
    let Some(parent) = occurrence.parent else {
        ensure!(
            occurrence.id == OccurrenceId(0),
            "Non-root occurrence has no parent"
        );
        return Ok(());
    };
    ensure!(
        parent.0 < occurrence.id.0,
        "Occurrence parent must precede its child"
    );
    ensure!(
        occurrence.outcome != OccurrenceOutcome::Root,
        "Child occurrence claims root outcome"
    );
    let parent = &result.occurrences[parent.0];
    ensure!(
        !matches!(parent.outcome, OccurrenceOutcome::Stopped { .. }),
        "Rendered occurrence has a stopped parent; preserve the request as blocked metadata"
    );
    ensure!(
        occurrence.path.0.len() == parent.path.0.len() + 1,
        "Invalid occurrence path depth"
    );
    ensure!(
        occurrence.path.0.starts_with(&parent.path.0),
        "Occurrence path does not extend its parent"
    );
    let choice = occurrence.path.0.last().unwrap();
    choice.validate()?;
    ensure!(
        choice.target.symbol == occurrence.symbol,
        "Occurrence target identity mismatch"
    );
    parent_call(result, parent.id, choice, occurrence.boundaries.call)
}
/// Compare only displayed result facts. Compiler eligibility and membership in
/// the immutable input IR are separately enforced by assembly.
fn parent_call(
    result: &ExpandedFlow,
    parent: OccurrenceId,
    choice: &CallChoice,
    boundary: Option<usize>,
) -> Result<()> {
    let found = result
        .chart
        .steps
        .iter()
        .zip(&result.step_origins)
        .any(|(step, origin)| {
            origin.occurrence == parent
                && boundary.is_none_or(|id| id == step.id)
                && origin.calls.iter().zip(&step.calls).any(|(address, call)| {
                    address == &choice.site && call.targets.contains(&choice.target)
                })
        });
    ensure!(
        found,
        "Child choice does not match its displayed parent call"
    );
    Ok(())
}
pub(super) fn span(result: &ExpandedFlow, source: SourceId, span: SourceSpan) -> Result<()> {
    if span == SourceSpan::default() {
        return Ok(());
    }
    let source = result
        .sources
        .get(source)
        .ok_or_else(|| anyhow::anyhow!("Unknown source index"))?;
    ensure!(
        source.content.get(span.start..span.end).is_some(),
        "Expanded range does not match analyzed UTF-8 source"
    );
    Ok(())
}
