//! Drawn virtual source rows and highlight evidence belong to their endpoint session.
use super::observation::{address, session};
use crate::{app::State, flow_comparison::SideId};
use anyhow::{Context, Result, ensure};
use rune::Runtime;
pub(super) fn source_drawn(runtime: &Runtime<State>, side: SideId) -> Result<bool> {
    let session = session(&runtime.state, side)?;
    let elements = runtime.elements();
    let Some(viewport) = elements
        .iter()
        .find(|el| !el.exiting && el.key.as_deref() == Some(&session.scroll_key))
    else {
        return Ok(false);
    };
    let ready = session.ready.as_ref().unwrap();
    let keys = address(side).keys(session);
    for (index, _) in ready.source.lines.iter().enumerate() {
        if let Some(row) = elements.iter().find(|el| {
            !el.exiting
                && el.key.as_deref() == Some(keys.line(index).as_str())
                && el.visible[3] > 1.0
        }) {
            let offset = viewport.rect[1] + index as f32 * 20.0 - row.rect[1];
            ensure!(
                (offset - session.scroll.get()[1]).abs() < 1.0,
                "Stored source offset differs from drawn rows"
            );
            let text = elements
                .iter()
                .find(|el| el.key.as_deref() == Some(keys.text(index).as_str()))
                .context("Drawn source row has no endpoint text")?;
            ensure!(
                text.text.as_deref()
                    == Some(
                        ready.source.text[ready.source.lines[index].range.clone()]
                            .trim_end_matches('\r')
                    ),
                "Drawn source row differs from its cached source"
            );
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) fn highlighted(runtime: &Runtime<State>, side: SideId) -> Result<bool> {
    let session = session(&runtime.state, side)?;
    let Some(span) = session.selected_source_span() else {
        return Ok(false);
    };
    let ready = session.ready.as_ref().unwrap();
    let keys = address(side).keys(session);
    let elements = runtime.elements();
    Ok(ready
        .source
        .lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.range.start < span.end && line.range.end >= span.start)
        .any(|(index, _)| highlighted_row(&elements, &keys.line(index))))
}
fn highlighted_row(elements: &[rune::ElementInfo], key: &str) -> bool {
    let Some(row) = elements
        .iter()
        .find(|el| !el.exiting && el.key.as_deref() == Some(key))
    else {
        return false;
    };
    row.fill.a > 0.01 && row.visible[3] > 1.0
}
