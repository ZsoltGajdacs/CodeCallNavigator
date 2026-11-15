//! Blocked entries describe requests, never synthesized callee source or execution.
use super::*;
use anyhow::{Context, Result, ensure};
use std::collections::HashSet;
pub(super) fn validate(result: &ExpandedFlow) -> Result<()> {
    let mut keys = HashSet::new();
    for blocked in &result.blocked_selections {
        ensure!(
            keys.insert((&blocked.caller, &blocked.choice.site)),
            "Duplicate blocked selection"
        );
        let ancestor = result
            .occurrences
            .get(blocked.ancestor.0)
            .context("Unknown blocked ancestor")?;
        ensure!(
            matches!(ancestor.outcome, OccurrenceOutcome::Stopped { .. }),
            "Blocked ancestor is not stopped"
        );
        ensure!(
            blocked.caller.0.starts_with(&ancestor.path.0),
            "Blocked selection is outside its ancestor path"
        );
        for choice in blocked
            .caller
            .0
            .iter()
            .chain(std::iter::once(&blocked.choice))
        {
            choice.validate()?;
        }
        let mut path = blocked.caller.clone();
        path.0.push(blocked.choice.clone());
        ensure!(
            !result
                .occurrences
                .iter()
                .any(|occurrence| occurrence.path == path),
            "Blocked selection is also rendered"
        );
    }
    Ok(())
}
