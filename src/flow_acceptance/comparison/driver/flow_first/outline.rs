//! Drive outline row hover, focus, and activation through native input.
use super::{Driver, Phase, Runtime, State, Step, change_key, focused, preview_drawn};
use crate::{engine_client::flow_comparison::ChangeKind, flow_comparison::PreviewTarget};
use anyhow::{Context, Result, ensure};
use rune::{Key, KeyEvent, Mods};
use std::time::{Duration, Instant};

// Leave enough time for the delayed canvas source popup to appear if a row
// accidentally triggers it.
const ROW_HOVER_HOLD: Duration = Duration::from_millis(600);
const ROWS: &str = "comparison-change-rows";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::flow_acceptance::comparison::driver) enum Row {
    File,
    Class,
    Method,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::flow_acceptance::comparison::driver) enum OutlineStep {
    Loaded,
    Hover(Row),
    Hovered(Row),
    OpenFile,
    OpenDiff,
    FileDiff,
    Restore,
    FocusClass,
    PressEnter,
}

impl OutlineStep {
    fn phase(self) -> Phase {
        Phase::FlowFirst(Step::Outline(self))
    }
}

impl Driver {
    pub(super) fn outline_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: OutlineStep,
    ) -> Result<()> {
        match step {
            OutlineStep::Hover(row) => self.hover_outline_row(runtime, row)?,
            OutlineStep::OpenFile => {
                if let Some(key) = self.changed_file_key(runtime)? {
                    self.click(runtime, &key, OutlineStep::OpenDiff.phase());
                }
            }
            OutlineStep::OpenDiff => {
                require_closed_preview(runtime)?;
                if runtime.state.flow_comparison.class_pair().is_some() {
                    self.click(
                        runtime,
                        "comparison-view-source-diff",
                        OutlineStep::FileDiff.phase(),
                    );
                }
            }
            OutlineStep::Restore => self.restore_outline(runtime),
            OutlineStep::FocusClass => self.focus_outline_class(runtime)?,
            OutlineStep::PressEnter => {
                key_press(runtime, Key::Enter);
                self.phase = Phase::FlowFirst(Step::ClassByKeyboardDrawn);
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn outline_observed(
        &mut self,
        runtime: &Runtime<State>,
        step: OutlineStep,
    ) -> Result<()> {
        let comparison = &runtime.state.flow_comparison;
        match step {
            OutlineStep::Loaded if comparison.changes.loaded && comparison.current.is_some() => {
                verify_outline(runtime)?;
                self.report.check("changes_outline_drawn_and_auto_selected");
                self.phase = OutlineStep::Hover(Row::File).phase();
            }
            OutlineStep::Hovered(row) => self.outline_hovered(runtime, row)?,
            OutlineStep::FileDiff
                if comparison
                    .previews
                    .active
                    .as_ref()
                    .is_some_and(|preview| matches!(preview.target, PreviewTarget::Row(_)))
                    && preview_drawn(runtime, "Leaf(2)") =>
            {
                self.report.check("git_file_diff_drawn_from_outline");
                self.phase = OutlineStep::Restore.phase();
            }
            OutlineStep::FocusClass
                if self
                    .outline_class_key(runtime)?
                    .is_some_and(|key| focused(runtime, &key)) =>
            {
                require_closed_preview(runtime)?;
                self.report.check("outline_focus_has_no_source_preview");
                self.phase = OutlineStep::PressEnter.phase();
            }
            _ => {}
        }
        Ok(())
    }

    fn changed_file_key(&self, runtime: &Runtime<State>) -> Result<Option<String>> {
        let rows = &runtime.state.flow_comparison.changes.rows;
        let file = rows
            .iter()
            .find(|row| row.symbol.as_deref() == Some(self.config.cases[0].symbol.as_str()))
            .and_then(|method| {
                rows.iter()
                    .find(|row| row.kind == ChangeKind::File && row.file == method.file)
            })
            .context("Git outline omitted the changed method's file row")?;
        Ok(change_key(runtime, &file.id))
    }

    fn outline_row_key(&self, runtime: &Runtime<State>, row: Row) -> Result<Option<String>> {
        match row {
            Row::File => self.changed_file_key(runtime),
            Row::Class => self.outline_class_key(runtime),
            Row::Method => self.method_change_key(runtime),
        }
    }

    fn hover_outline_row(&mut self, runtime: &mut Runtime<State>, row: Row) -> Result<()> {
        if let Some(key) = self.outline_row_key(runtime, row)? {
            if self.input.hover(runtime, &key) {
                self.requested = Some(Instant::now());
                self.phase = OutlineStep::Hovered(row).phase();
            }
        } else {
            let offset = if row == Row::File { 0.0 } else { 160.0 };
            self.input.scroll_pane(runtime, ROWS, offset);
        }
        Ok(())
    }

    fn outline_hovered(&mut self, runtime: &Runtime<State>, row: Row) -> Result<()> {
        require_closed_preview(runtime)?;
        if self
            .requested
            .is_none_or(|start| start.elapsed() < ROW_HOVER_HOLD)
        {
            return Ok(());
        }
        let key = self
            .outline_row_key(runtime, row)?
            .context("Hovered row left the view")?;
        ensure!(
            runtime.elements().iter().any(|element| {
                element.key.as_deref() == Some(&key) && element.hover > 0.0 && !element.exiting
            }),
            "The pointer did not reach the {row:?} row"
        );
        let next = match row {
            Row::File => OutlineStep::OpenFile,
            Row::Class => OutlineStep::Hover(Row::Method),
            Row::Method => {
                self.report.check("outline_hover_has_no_source_preview");
                OutlineStep::FocusClass
            }
        };
        self.phase = next.phase();
        Ok(())
    }

    fn restore_outline(&mut self, runtime: &mut Runtime<State>) {
        if runtime.state.flow_comparison.previews.active.is_some() {
            key_press(runtime, Key::Escape);
        } else {
            self.phase = OutlineStep::Hover(Row::Class).phase();
        }
    }

    fn focus_outline_class(&mut self, runtime: &mut Runtime<State>) -> Result<()> {
        self.input.hover(runtime, "comparison-base");
        match self.outline_class_key(runtime)? {
            Some(key) if !focused(runtime, &key) => key_press(runtime, Key::Tab),
            None => {
                self.input.scroll_pane(runtime, ROWS, 160.0);
            }
            _ => {}
        }
        Ok(())
    }
}

pub(super) fn require_closed_preview(runtime: &Runtime<State>) -> Result<()> {
    ensure!(
        runtime.state.flow_comparison.previews.active.is_none()
            && !runtime.elements().iter().any(|element| {
                element.key.as_deref() == Some("comparison-unified-source")
                    && !element.exiting
                    && element.visible[2].min(element.visible[3]) > 0.0
            }),
        "The outline shows a source preview at focus {:?}: {:?}",
        runtime
            .elements()
            .into_iter()
            .find(|element| element.focused)
            .and_then(|element| element.key),
        runtime.state.flow_comparison.previews.active,
    );
    Ok(())
}

fn key_press(runtime: &mut Runtime<State>, key: Key) {
    runtime.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
}

fn verify_outline(runtime: &Runtime<State>) -> Result<()> {
    let comparison = &runtime.state.flow_comparison;
    ensure!(
        comparison.git_outline(),
        "Native outline did not use Git source comparison"
    );
    let elements = runtime.elements();
    ensure!(
        elements.iter().any(|el| {
            el.key.as_deref() == Some("comparison-changes")
                && !el.exiting
                && el.visible[2].min(el.visible[3]) > 0.0
        }),
        "The comparison outline did not draw"
    );
    let first = comparison
        .changes
        .rows
        .iter()
        .find(|row| row.flow_available && row.symbol.is_some());
    ensure!(
        first.and_then(|row| row.symbol.as_deref())
            == comparison
                .current
                .as_ref()
                .map(|pair| pair.selection.symbol.as_str()),
        "The first available changed flow was not selected"
    );
    Ok(())
}
