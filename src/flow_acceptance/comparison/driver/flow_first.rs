//! Prove the default outline and unified diff through the native presentation.
mod class_flow;
mod popup;
use super::*;
use crate::flow_comparison::SideId;
pub(super) use popup::PopupObservation;
use popup::{Kind as PopupKind, Probe as PopupStep};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Outline,
    OpenFile,
    FileDiff,
    RestoreOutline,
    HoverOutlineClass,
    OutlineClassHovered,
    FocusOutlineClass,
    SelectMethod,
    Hover,
    Hovered,
    Focus,
    Class,
    ClassDrawn,
    HoverClass,
    ClassHovered,
    Popup(PopupKind, PopupStep),
    FocusClassNeighbor,
    ReturnToMethod,
    ReturnedToMethod,
}

impl Driver {
    pub(super) fn flow_first_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Step,
    ) -> Result<()> {
        match step {
            Step::OpenFile => self.open_changed_file(runtime),
            Step::RestoreOutline => self.restore_outline(runtime),
            Step::SelectMethod => self.open_method_change(runtime, Step::Hover),
            Step::Hover => self.hover_call(runtime),
            Step::Popup(kind, step) => self.popup_action(runtime, kind, step),
            Step::Focus => self.focus_call(runtime),
            Step::ReturnToMethod => self.open_method_change(runtime, Step::ReturnedToMethod),
            _ => self.class_flow_action(runtime, step),
        }
    }

    fn open_changed_file(&mut self, runtime: &mut Runtime<State>) -> Result<()> {
        use crate::engine_client::flow_comparison::ChangeKind;
        let rows = &runtime.state.flow_comparison.changes.rows;
        let file = rows
            .iter()
            .find(|row| row.symbol.as_deref() == Some(self.config.cases[0].symbol.as_str()))
            .and_then(|method| {
                rows.iter()
                    .find(|row| row.kind == ChangeKind::File && row.file == method.file)
            })
            .context("Git outline omitted the changed method's file row")?;
        if let Some(key) = change_key(runtime, &file.id) {
            self.click(runtime, &key, Phase::FlowFirst(Step::FileDiff));
        } else {
            self.input
                .scroll_pane(runtime, "comparison-change-rows", 0.0);
        }
        Ok(())
    }

    fn restore_outline(&mut self, runtime: &mut Runtime<State>) -> Result<()> {
        if runtime.state.flow_comparison.previews.active.is_some() {
            runtime.key(rune::KeyEvent {
                key: rune::Key::Escape,
                mods: rune::Mods::default(),
            });
            return Ok(());
        }
        if self.outline_class_key(runtime)?.is_some() && self.method_change_key(runtime)?.is_some()
        {
            self.phase = Phase::FlowFirst(Step::HoverOutlineClass);
        } else {
            self.input
                .scroll_pane(runtime, "comparison-change-rows", 160.0);
        }
        Ok(())
    }

    fn method_change_key(&self, runtime: &Runtime<State>) -> Result<Option<String>> {
        let symbol = self.config.cases[0].symbol.as_str();
        let change = runtime
            .state
            .flow_comparison
            .changes
            .rows
            .iter()
            .find(|row| row.symbol.as_deref() == Some(symbol))
            .context("The changed fixture method is absent from the outline")?;
        Ok(change_key(runtime, &change.id))
    }

    fn open_method_change(&mut self, runtime: &mut Runtime<State>, next: Step) -> Result<()> {
        if let Some(key) = self.method_change_key(runtime)? {
            self.click(runtime, &key, Phase::FlowFirst(next));
        }
        Ok(())
    }

    fn hover_call(&mut self, runtime: &mut Runtime<State>) -> Result<()> {
        if observation::pair(runtime, &self.config.cases[0])?.is_none() {
            return Ok(());
        }
        require_pane_labels(&runtime.elements(), "+ Changed", "Changed(")?;
        let key = self.call_key(runtime)?;
        if self.input.hover(runtime, &key) {
            self.phase = Phase::FlowFirst(Step::Hovered);
        }
        Ok(())
    }

    fn focus_call(&mut self, runtime: &mut Runtime<State>) -> Result<()> {
        self.input.hover(runtime, "comparison-base");
        if !focused(runtime, &self.call_key(runtime)?) {
            runtime.key(rune::KeyEvent {
                key: rune::Key::Tab,
                mods: rune::Mods::default(),
            });
        }
        Ok(())
    }

    pub(super) fn call_key(&self, runtime: &Runtime<State>) -> Result<String> {
        let after = observation::session(&runtime.state, SideId::After)?;
        let call =
            crate::flow_acceptance::observation::target(after, &self.config.cases[6].symbol)?;
        Ok(observation::address(SideId::After).key(&format!("step-{}", call.step)))
    }

    pub(super) fn flow_first_observed(
        &mut self,
        runtime: &Runtime<State>,
        step: Step,
    ) -> Result<()> {
        match step {
            Step::Outline | Step::FileDiff => self.file_outline_observed(runtime, step)?,
            Step::Hovered if preview_drawn(runtime, "Leaf(2)") => {
                self.report.check("unified_diff_drawn_on_hover");
                self.phase = PopupKind::Function.phase(PopupStep::Enter);
            }
            Step::Popup(kind, step) => self.popup_observed(runtime, kind, step)?,
            Step::Focus
                if focused(runtime, &self.call_key(runtime)?)
                    && preview_drawn(runtime, "Leaf(2)") =>
            {
                self.report.check("unified_diff_drawn_on_keyboard_focus");
                self.phase = Phase::FlowFirst(Step::Class);
            }
            Step::ReturnedToMethod
                if observation::pair(runtime, &self.config.cases[0])?.is_some() =>
            {
                self.report.check("method_flow_restored_from_outline");
                self.select_case(0, Purpose::Cases);
            }
            _ => self.class_flow_observed(runtime, step)?,
        }
        Ok(())
    }

    fn file_outline_observed(&mut self, runtime: &Runtime<State>, step: Step) -> Result<()> {
        let comparison = &runtime.state.flow_comparison;
        match step {
            Step::Outline if comparison.changes.loaded && comparison.current.is_some() => {
                verify_outline(runtime)?;
                self.report.check("changes_outline_drawn_and_auto_selected");
                self.phase = Phase::FlowFirst(Step::OpenFile);
            }
            Step::FileDiff if preview_drawn(runtime, "Leaf(2)") => {
                self.report.check("git_file_diff_drawn_from_outline");
                self.phase = Phase::FlowFirst(Step::RestoreOutline);
            }
            _ => {}
        }
        Ok(())
    }
}

fn require_pane_labels(
    elements: &[rune::runtime::ElementInfo],
    label: &str,
    help: &str,
) -> Result<()> {
    for side in ["Before", "After"] {
        let key = format!("comparison-pane-name-{side}");
        ensure!(
            elements.iter().any(|element| {
                element.key.as_deref() == Some(&key)
                    && element.text.as_deref() == Some(label)
                    && element
                        .help
                        .as_deref()
                        .is_some_and(|value| value.contains(help))
                    && !element.exiting
                    && element.visible[2].min(element.visible[3]) > 0.0
            }),
            "{side} heading must show {label} with help containing {help}"
        );
    }
    Ok(())
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
    ensure!(
        !comparison.controls.catalog_open,
        "Comparison opened the signature picker"
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

fn focused(runtime: &Runtime<State>, key: &str) -> bool {
    runtime.elements().iter().any(|element| {
        element.focused && element.focus_visible && element.key.as_deref() == Some(key)
    })
}

pub(super) fn change_key(runtime: &Runtime<State>, id: &str) -> Option<String> {
    runtime.elements().into_iter().find_map(|element| {
        let key = element.key?;
        (key.starts_with("comparison-change-")
            && key
                .strip_suffix(id)
                .is_some_and(|prefix| prefix.ends_with('-'))
            && !element.exiting
            && element.visible[2].min(element.visible[3]) > 0.0)
            .then_some(key)
    })
}

pub(super) fn preview_drawn(runtime: &Runtime<State>, changed_source: &str) -> bool {
    runtime.elements().iter().any(|element| {
        !element.exiting
            && element.visible[2].min(element.visible[3]) > 0.0
            && element.path.contains("/comparison-unified-row-")
            && element
                .text
                .as_deref()
                .is_some_and(|text| text.contains(changed_source))
    })
}
