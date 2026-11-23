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
        "comparison-load",
        "comparison-reload",
        "comparison-cancel",
        "comparison-back",
        "comparison-fit-before",
        "comparison-fit-after",
        "comparison-catalog-open",
        "comparison-choose-after",
        "comparison-reset-counterpart",
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
fn endpoint_provenance(elements: &[ElementInfo], name: &str, endpoint: &Endpoint) -> Result<()> {
    let commit = endpoint
        .commit
        .as_deref()
        .context("Expected committed endpoint provenance")?;
    let summary = format!(
        "{name}: {} · commit {}",
        endpoint.requested_ref,
        &commit[..commit.len().min(12)]
    );
    ensure!(
        visible_text(elements, &summary),
        "Requested ref/resolved commit is not drawn"
    );
    ensure!(
        visible_text(elements, &format!("Artifact {}", endpoint.digest)),
        "Artifact digest is not drawn"
    );
    ensure!(
        visible_text(elements, &endpoint.path),
        "Artifact origin is not drawn"
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
    endpoint_provenance(&elements, "Before", &header.base)?;
    endpoint_provenance(&elements, "After", &header.head)?;
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
