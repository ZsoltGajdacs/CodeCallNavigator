//! Only supplied CLI facts and the comparison's endpoint-local presentation are observed.
use super::config::{Binding, Case, ExpectedSide};
use crate::{
    app::State,
    flow_comparison::{Pair, Side, SideId},
    flow_view::address::Address,
    flow_workspace::Session,
};
use anyhow::{Result, ensure};
use rune::Runtime;
pub(super) fn bound(state: &State, expected: &Binding) -> Result<bool> {
    let Some(header) = state.flow_comparison.header() else {
        return Ok(false);
    };
    ensure!(
        header.comparison_id == expected.comparison_id,
        "Native comparison has another binding"
    );
    ensure!(
        header.base.commit.as_ref() == Some(&expected.base_commit)
            && header.head.commit.as_ref() == Some(&expected.head_commit),
        "Native resolved commits differ from CLI preparation"
    );
    ensure!(
        header.base.digest == expected.base_digest && header.head.digest == expected.head_digest,
        "Native artifact digests differ from CLI preparation"
    );
    Ok(true)
}
pub(super) fn pair<'a>(runtime: &'a Runtime<State>, case: &Case) -> Result<Option<&'a Pair>> {
    let Some(pair) = &runtime.state.flow_comparison.current else {
        return Ok(None);
    };
    if pair.selection.symbol != case.symbol || pair.selection.head_symbol != case.head_symbol {
        return Ok(None);
    }
    exact_side(&pair.before, &case.before)?;
    exact_side(&pair.after, &case.after)?;
    ensure!(
        (pair.correspondence.kind
            == crate::engine_client::flow_comparison::CorrespondenceKind::Manual)
            == case.head_symbol.is_some(),
        "Manual correspondence status changed"
    );
    Ok(Some(pair))
}
fn exact_side(actual: &Side, expected: &ExpectedSide) -> Result<()> {
    match (actual, expected) {
        (
            Side::Ready(actual),
            ExpectedSide::Ready {
                symbol,
                source,
                chart,
            },
        ) => {
            ensure!(
                actual.node.symbol == *symbol,
                "Ready node belongs to another endpoint identity"
            );
            let ready = actual
                .session
                .ready
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Ready endpoint has no rendered chart"))?;
            ensure!(
                ready.source.text == *source,
                "Native source differs from the exact saved snapshot"
            );
            ensure!(
                serde_json::to_value(&ready.chart)? == serde_json::to_value(chart)?,
                "Native chart differs from the endpoint CLI chart"
            );
        }
        (Side::Missing, ExpectedSide::Missing) => (),
        (Side::Unavailable(actual), ExpectedSide::Unavailable { reason }) => ensure!(
            actual.failure.reason == *reason,
            "Unavailable reason differs from CLI facts"
        ),
        _ => anyhow::bail!("Native endpoint availability differs from CLI preparation"),
    }
    Ok(())
}
pub(super) fn address(side: SideId) -> Address {
    match side {
        SideId::Before => Address::Before,
        SideId::After => Address::After,
    }
}
pub(super) fn session(state: &State, side: SideId) -> Result<&Session> {
    let pair = state
        .flow_comparison
        .current
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No native pair"))?;
    match pair.side(side) {
        Side::Ready(ready) => Ok(&ready.session),
        _ => anyhow::bail!("Selected endpoint has no ready chart"),
    }
}
pub(super) use super::source_observation::{highlighted, source_drawn};
pub(super) use super::surface::{availability_drawn, controls, provenance};
