//! Prove the default outline and unified diff through the native presentation.
mod class_depth;
mod class_flow;
mod outline;
mod popup;
use super::*;
use crate::flow_comparison::SideId;
use class_depth::DepthStep;
use outline::OutlineStep;
pub(super) use popup::PopupObservation;
use popup::{Kind as PopupKind, Probe as PopupStep};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Outline(OutlineStep),
    Depth(DepthStep),
    ClassByKeyboardDrawn,
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

impl Step {
    pub(super) fn outline() -> Self {
        Self::Outline(OutlineStep::Loaded)
    }
}

impl Driver {
    pub(super) fn flow_first_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Step,
    ) -> Result<()> {
        match step {
            Step::Outline(step) => self.outline_action(runtime, step),
            Step::Depth(step) => self.depth_action(runtime, step),
            Step::SelectMethod => self.open_method_change(runtime, Step::Hover),
            Step::Hover => self.hover_call(runtime),
            Step::Popup(kind, step) => self.popup_action(runtime, kind, step),
            Step::Focus => self.focus_call(runtime),
            Step::ReturnToMethod => self.open_method_change(runtime, Step::ReturnedToMethod),
            _ => self.class_flow_action(runtime, step),
        }
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
            Step::Outline(step) => self.outline_observed(runtime, step)?,
            Step::Depth(step) => self.depth_observed(runtime, step)?,
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
