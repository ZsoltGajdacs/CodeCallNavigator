//! Assertions over a frame that the native window actually drew.
use super::config::Ordinary as Configuration;
use crate::{app::State, flow_workspace::Session, source::SourceFile};
use anyhow::{Result, ensure};
use codecallnavigator::flow::SourceSpan;
use rune::{ElementInfo, Runtime};
use std::ops::Range;

#[derive(Clone, Copy)]
pub(super) struct CallSelection {
    pub step: usize,
    pub call: usize,
    pub target: usize,
}

pub(super) struct TargetDetails {
    pub signature: String,
    pub certainty: String,
    pub provenance: String,
}

pub(super) fn boot(state: &State, configuration: &Configuration) -> Result<bool> {
    let Some(context) = &state.query_context else {
        return Ok(false);
    };
    ensure!(
        !state.uses_fixture_queries(),
        "Acceptance boot used a fixture backend"
    );
    ensure!(
        context.descriptor.project == configuration.project,
        "Acceptance loaded another CLI project"
    );
    ensure!(
        context.descriptor.generation == state.generation && state.generation.is_some(),
        "Acceptance graph is not generation-bound"
    );
    ensure!(
        state.nav.selected.as_deref() == Some(&configuration.root_symbol),
        "Saved inspector selection is not the configured root method"
    );
    Ok(state.view_is_current() && state.flow_supported("csharp"))
}

pub(super) fn session<'a>(state: &'a State, symbol: &str) -> Result<Option<&'a Session>> {
    let Some(session) = &state.flow_workspace.current else {
        return Ok(None);
    };
    if session.symbol != symbol {
        return Ok(None);
    }
    if session.ready.is_some() {
        return Ok(Some(session));
    }
    ensure!(
        session.message.is_empty() || session.message.starts_with("Preparing"),
        "Flow could not open: {}",
        session.message
    );
    Ok(None)
}

pub(super) fn target(session: &Session, symbol: &str) -> Result<CallSelection> {
    let chart = &session
        .ready
        .as_ref()
        .expect("observed ready session")
        .chart;
    chart
        .steps
        .iter()
        .enumerate()
        .find_map(|(step, value)| {
            value.calls.iter().enumerate().find_map(|(call, site)| {
                site.targets
                    .iter()
                    .position(|candidate| candidate.symbol == symbol)
                    .map(|target| CallSelection { step, call, target })
            })
        })
        .ok_or_else(|| {
            anyhow::anyhow!("Root chart has no call site for the exact configured target")
        })
}

pub(super) fn highlighted(
    runtime: &Runtime<State>,
    session: &Session,
    step: usize,
) -> Result<bool> {
    let ready = session.ready.as_ref().expect("observed ready session");
    let span = ready.chart.steps[step].span;
    ensure!(
        span != SourceSpan::default(),
        "Configured call step has no source location"
    );
    let mut visible = false;
    for element in runtime.elements() {
        let Some(range) = source_row_range(&ready.source, &element)? else {
            continue;
        };
        if element.fill.a > 0.1 {
            ensure!(
                range.start < span.end && range.end > span.start,
                "An unrelated source row received the call-step highlight"
            );
            visible |= element.visible[3] > 1.0;
        }
    }
    Ok(visible)
}

fn source_row_range(source: &SourceFile, element: &ElementInfo) -> Result<Option<Range<usize>>> {
    let Some(index) = element
        .key
        .as_deref()
        .and_then(|key| key.strip_prefix("flow-source-line-"))
        .and_then(|index| index.parse::<usize>().ok())
    else {
        return Ok(None);
    };
    let line = source
        .lines
        .get(index)
        .ok_or_else(|| anyhow::anyhow!("Displayed source row {index} is outside the snapshot"))?;
    let end = source
        .lines
        .get(index + 1)
        .map_or(line.range.end, |next| next.range.start);
    Ok(Some(line.range.start..end))
}

pub(super) fn unlocated_has_no_highlight(runtime: &Runtime<State>) -> bool {
    runtime
        .elements()
        .iter()
        .filter(|element| {
            element
                .key
                .as_deref()
                .is_some_and(|key| key.starts_with("flow-source-line-"))
        })
        .all(|element| element.fill.a < 0.01)
}

pub(super) fn unlocated_explained(runtime: &Runtime<State>) -> bool {
    runtime.elements().iter().any(|element| {
        element.key.as_deref() == Some("flow-source-unlocated")
            && element.visible[2] > 1.0
            && element.visible[3] > 1.0
            && element.text.as_deref().is_some_and(|text| !text.is_empty())
    })
}

pub(super) fn target_details(
    runtime: &Runtime<State>,
    session: &Session,
    selection: CallSelection,
) -> Result<Option<TargetDetails>> {
    let elements = runtime.elements();
    let key = format!("flow-target-{}-{}", selection.call, selection.target);
    let Some(button) = elements
        .iter()
        .find(|element| element.key.as_deref() == Some(&key))
    else {
        return Ok(None);
    };
    if button.visible[2] < 1.0 || button.visible[3] < 1.0 {
        return Ok(None);
    }
    let target = &session
        .ready
        .as_ref()
        .expect("observed ready session")
        .chart
        .steps[selection.step]
        .calls[selection.call]
        .targets[selection.target];
    let graph = runtime.state.graph.as_ref().expect("observed CLI graph");
    let index = graph
        .node_index(&target.symbol)
        .ok_or_else(|| anyhow::anyhow!("Displayed target has no graph identity"))?;
    let signature = &graph.data.nodes[index as usize].signature;
    let details = TargetDetails {
        signature: signature.clone(),
        certainty: target.certainty.clone(),
        provenance: target.provenance.clone(),
    };
    assert_target_presentation(&elements, button, &target.symbol, &details)?;
    Ok(Some(details))
}

fn assert_target_presentation(
    elements: &[ElementInfo],
    button: &ElementInfo,
    symbol: &str,
    details: &TargetDetails,
) -> Result<()> {
    let label = format!("{} · {}", details.signature, details.certainty);
    let displayed = button
        .label
        .as_deref()
        .is_some_and(|text| text.contains(&label))
        || elements.iter().any(|element| {
            element.path.starts_with(&format!("{}/", button.path))
                && element
                    .text
                    .as_deref()
                    .is_some_and(|text| text.contains(&label))
        });
    ensure!(
        displayed,
        "Target button omitted its exact overload signature or certainty"
    );
    ensure!(
        button.help.as_deref() == Some(&format!("{symbol}\n{}", details.provenance)),
        "Target button omitted compiler identity/provenance help"
    );
    Ok(())
}

pub(super) fn controls_visible(runtime: &Runtime<State>) -> Result<()> {
    let elements = runtime.elements();
    for key in [
        "flow-close",
        "flow-back",
        "flow-fit",
        "flow-canvas",
        "flow-source-pane",
    ] {
        let element = elements
            .iter()
            .find(|element| element.key.as_deref() == Some(key))
            .ok_or_else(|| anyhow::anyhow!("Missing flow control {key}"))?;
        assert_control_bounds(element, key, runtime.viewport.0)?;
    }
    Ok(())
}

fn assert_control_bounds(element: &ElementInfo, key: &str, width: f32) -> Result<()> {
    ensure!(
        element.visible[2] > 20.0 && element.visible[3] > 15.0,
        "Flow control {key} is clipped"
    );
    ensure!(
        element.rect[0] >= -1.0 && element.rect[0] + element.rect[2] <= width + 1.0,
        "Flow control {key} lies outside the window"
    );
    Ok(())
}
