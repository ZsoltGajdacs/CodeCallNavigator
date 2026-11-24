//! Minimum native controls, visible endpoint provenance and availability notices.
use super::config::{Case, ExpectedSide};
use crate::{app::State, engine_client::flow_comparison::Endpoint};
use anyhow::{Context, Result, ensure};
use rune::{ElementInfo, Runtime};
pub(super) fn controls(runtime: &Runtime<State>) -> Result<()> {
    for key in [
        "comparison-close",
        "comparison-base",
        "comparison-head",
        "comparison-snapshot-base-find",
        "comparison-snapshot-after-find",
        "comparison-snapshot-base-prepare",
        "comparison-snapshot-after-prepare",
        "comparison-fit-before",
        "comparison-fit-after",
        "comparison-before-canvas",
        "comparison-after-canvas",
        "comparison-source-before",
        "comparison-source-after",
        "comparison-source-area",
    ] {
        let elements = runtime.elements();
        let el = elements
            .iter()
            .find(|el| el.key.as_deref() == Some(key) && !el.exiting)
            .ok_or_else(|| anyhow::anyhow!("Missing native control {key}"))?;
        control_bounds(el, runtime.viewport).with_context(|| {
            format!(
                "{key}: drawn {:?}, target {:?}, viewport {:?}",
                el.rect, el.target, runtime.viewport
            )
        })?;
    }
    Ok(())
}

fn inside_axis(origin: f32, size: f32, limit: f32) -> bool {
    origin >= -0.5 && origin + size <= limit + 0.5
}
fn control_bounds(el: &ElementInfo, viewport: (f32, f32)) -> Result<()> {
    ensure!(el.rect[2].min(el.rect[3]) > 0.0, "Empty native control");
    ensure!(
        inside_axis(el.rect[0], el.rect[2], viewport.0)
            && inside_axis(el.rect[1], el.rect[3], viewport.1),
        "Native control is outside minimum window"
    );
    ensure!(
        el.rect[2..]
            .iter()
            .zip(&el.visible[2..])
            .all(|(a, b)| (a - b).abs() < 0.5),
        "Native control is clipped"
    );
    Ok(())
}
fn visible_text(elements: &[ElementInfo], needle: &str) -> bool {
    elements
        .iter()
        .filter(|el| !el.exiting && el.visible[2].min(el.visible[3]) > 1.0)
        .any(|el| el.text.as_deref().is_some_and(|text| text.contains(needle)))
}
fn endpoint_provenance(
    elements: &[ElementInfo],
    picker: &str,
    status: &str,
    endpoint: &Endpoint,
) -> Result<()> {
    let commit = endpoint
        .commit
        .as_deref()
        .context("Expected a committed endpoint")?;
    let abbreviation = &commit[..commit.len().min(10)];
    ensure!(
        elements.iter().any(|el| {
            el.key.as_deref() == Some(picker)
                && !el.exiting
                && el.visible[2] > 1.0
                && el
                    .label
                    .as_deref()
                    .is_some_and(|label| label.contains(abbreviation))
        }),
        "Revision picker does not identify the bound commit"
    );
    let details = elements
        .iter()
        .find(|el| el.key.as_deref() == Some(status) && !el.exiting && el.visible[2] > 1.0)
        .and_then(|el| el.help.as_deref())
        .context("Snapshot status omits provenance details")?;
    ensure!(
        details.contains(&endpoint.digest) && details.contains(&endpoint.path),
        "Snapshot provenance details do not identify the bound artifact"
    );
    Ok(())
}

pub(super) fn provenance(runtime: &Runtime<State>) -> Result<()> {
    let header = runtime
        .state
        .flow_comparison
        .header()
        .context("No drawn comparison provenance")?;
    let elements = runtime.elements();
    endpoint_provenance(
        &elements,
        "comparison-base",
        "comparison-snapshot-base",
        &header.base,
    )?;
    endpoint_provenance(
        &elements,
        "comparison-head",
        "comparison-snapshot-after",
        &header.head,
    )?;
    let close = elements
        .iter()
        .find(|el| el.key.as_deref() == Some("comparison-close"))
        .context("No native comparison close control")?;
    ensure!(
        close.rect[0] >= rune::tooling::Chrome::DEFAULT.titlebar_inset(),
        "Comparison close overlaps native titlebar controls"
    );
    Ok(())
}
pub(super) fn availability_drawn(runtime: &Runtime<State>, case: &Case) -> Result<()> {
    for expected in [&case.before, &case.after] {
        let notice = match expected {
            ExpectedSide::Missing => Some("Missing in this revision"),
            ExpectedSide::Unavailable { reason } => Some(reason.as_str()),
            _ => None,
        };
        if let Some(notice) = notice {
            ensure!(
                visible_text(&runtime.elements(), notice),
                "Endpoint availability explanation is not drawn"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;
