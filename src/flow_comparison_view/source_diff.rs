//! Floating unified source previews render the engine's typed line changes.
use crate::{
    app::State,
    engine_client::flow_comparison::{Hunk, LineKind, SourceDiff, SourceStatus},
    flow_comparison::{PreviewTarget, Workspace},
    source_popup::{Content, Heading, Owner, Panel},
    theme,
};
use codecallnavigator::artifact::comparison_changes::ClassSource;
use rune::{Align, Cx, El, GrowthSpec, Side, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit};
use std::sync::Arc;

pub fn layer(state: &State, cx: &Cx<State>, k: &Kit) -> Option<El> {
    let workspace = &state.flow_comparison;
    let preview = workspace.previews.active.as_ref()?;
    let (title, content) = preview_content(workspace, &preview.target, k)?;
    if matches!(preview.target, PreviewTarget::Row(_)) {
        return Some(file_diff_panel(&preview.anchor, &title, content, k));
    }
    let popup = &workspace.previews.interaction;
    let bounds = if popup.fixed() {
        popup.fixed_bounds()
    } else {
        popup.bounds_next_to(cx.window_layout(&preview.anchor)?)
    };
    let panel = Panel {
        owner: Owner::Compare,
        bounds,
        interaction: popup,
    };
    Some(
        panel.view(
            Content {
                heading: Heading {
                    title: if matches!(preview.target, PreviewTarget::Class(..)) {
                        "Saved source"
                    } else {
                        "Source diff"
                    },
                    lead: None,
                },
                description: &title,
                body: col()
                    .key(format!("preview-content-{}", preview.anchor))
                    .w_full()
                    .grow(1.0)
                    .min_h(0.0)
                    .child(content),
                provenance: "Compared revisions",
            },
            k,
        ),
    )
}

fn preview_content(workspace: &Workspace, target: &PreviewTarget, k: &Kit) -> Option<(String, El)> {
    if let PreviewTarget::Class(side, id) = target {
        return Some(class_content(workspace.class_preview(*side, id), *side, k));
    }
    let id = match target {
        PreviewTarget::Row(id) | PreviewTarget::ClassEdit(id) => id.as_str(),
        _ => workspace.selected_change_id()?,
    };
    Some(match workspace.previews.preview_diff(id) {
        None => ("Source diff".into(), notice("Loading source…", k)),
        Some(Err(error)) => ("Source diff".into(), notice(error, k)),
        Some(Ok(diff)) => (diff.file.clone(), edit_content(workspace, diff, target, k)),
    })
}

fn file_diff_panel(anchor: &str, title: &str, body: El, k: &Kit) -> El {
    let header = row()
        .h(28.0)
        .w_full()
        .px(8.0)
        .center_y()
        .gap(8.0)
        .child(
            text(title)
                .font(k.ty.small_medium)
                .color(k.c.text_base)
                .min_w(0.0)
                .grow(1.0)
                .truncate(),
        )
        .child(
            Button::new("Close")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut State| s.flow_comparison.previews.dismiss())
                .build(k),
        );
    col()
        .key("comparison-unified-source")
        .anchored_fit(anchor, Side::Bottom, Align::Start, 8.0)
        .w(680.0)
        .min_h(0.0)
        .radius(8.0)
        .border(1.0, k.c.std_border)
        .fill(k.c.surface_raised)
        .role("group")
        .label("Unified source diff")
        .child(header)
        .child(body)
}

fn edit_content(workspace: &Workspace, diff: &SourceDiff, target: &PreviewTarget, k: &Kit) -> El {
    if diff.status == SourceStatus::Unavailable {
        return notice(&unavailable_message(diff.reason.as_deref()), k);
    }
    let (hunks, empty_message) = match target {
        PreviewTarget::Step(side, index) => (
            workspace
                .step_hunk(diff, *side, *index)
                .map(std::slice::from_ref)
                .unwrap_or_default(),
            "No source change at this step",
        ),
        PreviewTarget::Row(_) | PreviewTarget::ClassEdit(_) => {
            (diff.hunks.as_slice(), "No source changes in this selection")
        }
        PreviewTarget::Class(..) => unreachable!("class source has its own content"),
    };
    if hunks.is_empty() {
        notice(empty_message, k)
    } else {
        diff_lines(hunks, matches!(target, PreviewTarget::Row(_)), k)
    }
}

fn unavailable_message(reason: Option<&str>) -> std::borrow::Cow<'_, str> {
    match reason {
        Some("binary_content") => "This file contains binary data.".into(),
        Some("source_too_large") => "This file exceeds the source preview size limit.".into(),
        Some("source_capture_budget_exceeded") => {
            "This file could not be previewed within the comparison’s source capture limit.".into()
        }
        Some(reason) => reason.replace('_', " ").into(),
        None => "Source is unavailable.".into(),
    }
}

fn class_content(
    result: Option<Result<&ClassSource, &str>>,
    side: crate::flow_comparison::SideId,
    k: &Kit,
) -> (String, El) {
    let message = match result {
        None => "Loading saved source…",
        Some(Err(error)) => error,
        Some(Ok(ClassSource::Unavailable { reason })) => reason,
        Some(Ok(ClassSource::Diff { file, hunks })) => {
            return (
                format!("{file} · Saved graph source"),
                diff_lines(hunks, false, k),
            );
        }
        Some(Ok(ClassSource::Excerpt { file, start, text })) => {
            let before = side == crate::flow_comparison::SideId::Before;
            let lines = text
                .lines()
                .enumerate()
                .map(|(index, text)| DisplayLine {
                    before: before.then_some(start + index),
                    after: (!before).then_some(start + index),
                    kind: LineKind::Context,
                    text: text.into(),
                })
                .collect();
            let excerpt = source_rows(lines, false, k);
            return (format!("{file}:{start} · Saved graph source"), excerpt);
        }
    };
    ("Saved source".into(), notice(message, k))
}

fn notice(value: &str, k: &Kit) -> El {
    text(value)
        .w_full()
        .padding(8.0)
        .font(k.ty.small)
        .color(k.c.text_muted)
        .wrap()
}

struct DisplayLine {
    before: Option<usize>,
    after: Option<usize>,
    kind: LineKind,
    text: String,
}

fn diff_lines(hunks: &[Hunk], file_row: bool, k: &Kit) -> El {
    let mut lines = Vec::new();
    for hunk in hunks {
        let mut before = hunk.before.start;
        let mut after = hunk.after.start;
        lines.push(DisplayLine {
            before: None,
            after: None,
            kind: LineKind::Context,
            text: format!(
                "@@ -{},{} +{},{} @@",
                before, hunk.before.count, after, hunk.after.count
            ),
        });
        for line in &hunk.lines {
            let previous = (line.kind != LineKind::Insert).then_some(before);
            let next = (line.kind != LineKind::Delete).then_some(after);
            before += usize::from(previous.is_some());
            after += usize::from(next.is_some());
            lines.push(DisplayLine {
                before: previous,
                after: next,
                kind: line.kind,
                text: line.text.trim_end_matches(['\r', '\n']).to_owned(),
            });
        }
    }
    source_rows(lines, file_row, k)
}

fn source_rows(lines: Vec<DisplayLine>, file_row: bool, k: &Kit) -> El {
    let widest = lines
        .iter()
        .map(|line| line.text.chars().count())
        .max()
        .unwrap_or_default();
    let viewport_height = (lines.len() as f32 * 20.0).min(260.0);
    let lines = Arc::new(lines);
    let kit = k.clone();
    let viewport = col().key("comparison-source-scroll").w_full().min_h(0.0);
    let viewport = if file_row {
        viewport.h(viewport_height)
    } else {
        viewport.grow(1.0)
    };
    viewport.scroll_xy().selection_group(true).child(
        col()
            .w((widest as f32 * 8.0 + 92.0).max(400.0))
            .growth(GrowthSpec::default())
            .virtual_rows(
                lines.len(),
                20.0,
                |_: &State, i| format!("comparison-unified-row-{i}").into(),
                move |_: &State, i| render_line(&lines[i], &kit),
            ),
    )
}

fn render_line(line: &DisplayLine, k: &Kit) -> El {
    let (prefix, fg, bg) = match line.kind {
        LineKind::Context => (" ", k.c.text_base, rune::Color::TRANSPARENT),
        LineKind::Delete => ("−", k.c.danger.fg, k.c.danger.fg.alpha(0.12)),
        LineKind::Insert => ("+", k.c.success.fg, k.c.success.fg.alpha(0.12)),
    };
    let gutter = |line: Option<usize>| {
        text(line.map(|n| format!("{n:>5}")).unwrap_or_default())
            .w(40.0)
            .font(theme::mono(11.0, 400.0).lh(20.0))
            .color(k.c.text_muted)
    };
    row()
        .h(20.0)
        .w_full()
        .fill(bg)
        .child(gutter(line.before))
        .child(gutter(line.after))
        .child(
            text(prefix)
                .w(14.0)
                .font(theme::mono(11.0, 500.0).lh(20.0))
                .color(fg),
        )
        .child(
            text(&line.text)
                .font(theme::mono(12.0, 400.0).lh(20.0))
                .color(fg)
                .selectable(true)
                .selection_color(k.selection()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    #[test]
    fn unavailable_diff_explains_capture_limit_in_preview() {
        let mut runtime = Runtime::new(
            State::new(),
            |state, _| {
                let diff: SourceDiff = serde_json::from_value(serde_json::json!({
                    "kind": "source_diff",
                    "id": "file",
                    "status": "unavailable",
                    "file": "App.java",
                    "hunks": [],
                    "reason": "source_capture_budget_exceeded"
                }))
                .unwrap();
                edit_content(
                    &state.flow_comparison,
                    &diff,
                    &PreviewTarget::Row(diff.id.clone()),
                    &theme::kit(true),
                )
            },
            (520.0, 280.0),
            Env::new(true, 1.0),
        );
        runtime.frame(0.0);
        runtime.settle_all();
        runtime.frame(0.0);
        assert!(runtime.elements().iter().any(|element| {
            element
                .text
                .as_deref()
                .is_some_and(|text| text.contains("comparison’s source capture limit"))
        }));
    }

    #[test]
    fn unified_diff_mounts_virtual_rows_and_draws_changed_source() {
        let mut runtime = Runtime::new(
            State::new(),
            |_, _| {
                let hunk: Hunk = serde_json::from_value(serde_json::json!({
                    "before": {"start": 2, "count": 1},
                    "after": {"start": 2, "count": 1},
                    "lines": [
                        {"kind": "delete", "text": "CheckOld();\n"},
                        {"kind": "insert", "text": "CheckNew();\n"}
                    ]
                }))
                .unwrap();
                diff_lines(&[hunk], true, &theme::kit(true))
            },
            (520.0, 280.0),
            Env::new(true, 1.0),
        );
        runtime.frame(0.0);
        runtime.settle_all();
        runtime.frame(0.0);
        assert!(runtime.elements().iter().any(|element| {
            element.text.as_deref() == Some("CheckNew();")
                && element.visible[2].min(element.visible[3]) > 0.0
        }));
    }
}
