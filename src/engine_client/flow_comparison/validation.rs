//! Validate reply attribution, not compiler semantics or comparison identity algorithms.
use super::model::*;
use anyhow::{Result, ensure};

mod catalog;
pub(super) use catalog::validate as catalog;

pub(super) fn detail(detail: &Detail, symbol: &str, head_symbol: &str) -> Result<()> {
    ensure!(
        detail.kind == "detail",
        "CLI returned a different comparison selection"
    );
    let correspondence = &detail.correspondence;
    ensure!(
        correspondence.symbol == symbol && correspondence.head_symbol == head_symbol,
        "CLI returned different comparison functions"
    );
    let kind = if symbol == head_symbol {
        CorrespondenceKind::Exact
    } else {
        CorrespondenceKind::Manual
    };
    ensure!(
        correspondence.kind == kind,
        "CLI returned different counterpart provenance"
    );
    side(&detail.before, symbol)?;
    side(&detail.after, head_symbol)
}
fn side(side: &DetailSide, symbol: &str) -> Result<()> {
    match side {
        DetailSide::Ready(ready) => ready_side(ready, symbol),
        DetailSide::Unavailable(side) => {
            if let Some(node) = &side.node {
                ensure!(
                    node.symbol == symbol,
                    "Unavailable node belongs to another function"
                );
            }
            Ok(())
        }
        DetailSide::Missing => Ok(()),
    }
}
fn ready_side(ready: &Ready, symbol: &str) -> Result<()> {
    ensure!(
        matches!(ready.membership, Presence::Present),
        "Ready flow has unknown membership"
    );
    ensure!(
        ready.node.symbol == symbol
            && ready.function.symbol == symbol
            && ready.chart.symbol == symbol
            && ready.source.symbol == symbol,
        "Flow/source belongs to another function"
    );
    ensure!(
        ready.source.line == ready.node.line,
        "Flow/source line ownership differs"
    );
    ensure!(
        ready.source.origin == "analyzed",
        "Historical source is not analyzed source"
    );
    let owner = &ready.node.owner;
    ensure!(
        ready.source.file == owner.file
            && ready.function.file == owner.file
            && ready.chart.file == owner.file,
        "Flow/source file ownership differs"
    );
    ensure!(
        ready.function.build == owner.build
            && ready.function.project == owner.project
            && ready.function.language == owner.language,
        "Flow function ownership differs"
    );
    Ok(())
}
