//! Reject inconsistent v1 catalog attribution; membership remains CLI-owned.
use super::super::model::*;
use anyhow::{Result, ensure};

pub(in crate::engine_client::flow_comparison) fn validate(
    page: &Catalog,
    text: &str,
    offset: usize,
    limit: usize,
) -> Result<()> {
    ensure!(
        page.kind == "catalog",
        "CLI returned a different comparison selection"
    );
    ensure!(
        page.text == text && page.offset == offset && page.limit == limit,
        "CLI returned a different catalog page"
    );
    ensure!(
        page.returned == page.records.len() && page.returned <= limit,
        "Invalid CLI catalog record count"
    );
    ensure!(
        page.returned <= page.total.saturating_sub(offset),
        "CLI catalog records exceed the reported total"
    );
    let next = offset
        .checked_add(page.returned)
        .ok_or_else(|| anyhow::anyhow!("Invalid CLI catalog offset"))?;
    ensure!(
        page.more == (next < page.total),
        "Inconsistent CLI catalog continuation"
    );
    if page.more {
        ensure!(
            page.returned > 0 && page.next_offset == Some(next),
            "CLI catalog cursor does not progress"
        );
    } else {
        ensure!(page.next_offset.is_none(), "Unexpected CLI catalog cursor");
    }
    records(page)
}
fn records(page: &Catalog) -> Result<()> {
    let complete = matches!(page.endpoints.before, EndpointStatus::Available)
        && matches!(page.endpoints.after, EndpointStatus::Available);
    ensure!(
        page.complete == complete,
        "CLI catalog completeness contradicts endpoint availability"
    );
    let mut previous = None;
    for record in &page.records {
        if let Some(previous) = previous {
            ensure!(
                previous < record.symbol.as_str(),
                "CLI catalog identities are duplicated or out of order"
            );
        }
        previous = Some(record.symbol.as_str());
        side(&record.before, &page.endpoints.before, &record.symbol)?;
        side(&record.after, &page.endpoints.after, &record.symbol)?;
        membership(record)?;
    }
    Ok(())
}
fn side(side: &CatalogSide, endpoint: &EndpointStatus, symbol: &str) -> Result<()> {
    let available = matches!(endpoint, EndpointStatus::Available);
    let unknown = matches!(side, CatalogSide::Unknown(_));
    ensure!(
        available != unknown,
        "CLI catalog side contradicts endpoint availability"
    );
    if let CatalogSide::Present(present) = side
        && let Some(node) = &present.node
    {
        ensure!(
            node.symbol == symbol,
            "CLI catalog node belongs to another identity"
        );
    }
    Ok(())
}
fn membership(record: &Record) -> Result<()> {
    use CatalogSide::{Missing, Present, Unknown};
    let sides = (&record.before, &record.after);
    ensure!(
        matches!(record.before, Present(_)) || matches!(record.after, Present(_)),
        "CLI catalog identity has no known present endpoint"
    );
    let consistent = match record.membership {
        Membership::Both => matches!(sides, (Present(_), Present(_))),
        Membership::Added => matches!(sides, (Missing, Present(_))),
        Membership::Removed => matches!(sides, (Present(_), Missing)),
        Membership::Unknown => matches!(sides, (Unknown(_), _) | (_, Unknown(_))),
    };
    ensure!(consistent, "CLI catalog membership contradicts its sides");
    Ok(())
}
