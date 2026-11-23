//! Prove the default outline and unified diff through the native presentation.
use super::*;
use crate::flow_comparison::SideId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Outline,
    SelectMethod,
    Hover,
    Hovered,
    Focus,
    Class,
    ClassPreview,
}

impl Driver {
    pub(super) fn flow_first_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Step,
    ) -> Result<()> {
        match step {
            Step::SelectMethod => self.open_change(runtime, false),
            Step::Class => self.open_change(runtime, true),
            Step::Hover => self.hover_call(runtime),
            Step::Focus => self.focus_call(runtime),
            _ => Ok(()),
        }
    }

    fn open_change(&mut self, runtime: &mut Runtime<State>, class: bool) -> Result<()> {
        let symbol = self.config.cases[0].symbol.as_str();
        let change = runtime
            .state
            .flow_comparison
            .changes
            .rows
            .iter()
            .find(|row| row.symbol.as_deref() == Some(symbol))
            .context("The changed fixture method is absent from the outline")?;
        let (id, next) = if class {
            (
                change
                    .class_id
                    .as_deref()
                    .context("The fixture has no owning class row")?,
                Step::ClassPreview,
            )
        } else {
            (change.id.as_str(), Step::Hover)
        };
        if let Some(key) = change_key(runtime, id) {
            self.click(runtime, &key, Phase::FlowFirst(next));
        }
        Ok(())
    }

    fn hover_call(&mut self, runtime: &mut Runtime<State>) -> Result<()> {
        if observation::pair(runtime, &self.config.cases[0])?.is_none() {
            return Ok(());
        }
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
        let comparison = &runtime.state.flow_comparison;
        match step {
            Step::Outline if comparison.changes.loaded && comparison.current.is_some() => {
                verify_outline(runtime)?;
                self.report.check("changes_outline_drawn_and_auto_selected");
                self.phase = Phase::FlowFirst(Step::SelectMethod);
            }
            Step::Hovered if preview_drawn(runtime, "Leaf(2)") => {
                self.report.check("unified_diff_drawn_on_hover");
                self.phase = Phase::FlowFirst(Step::Focus);
            }
            Step::Focus
                if focused(runtime, &self.call_key(runtime)?)
                    && preview_drawn(runtime, "Leaf(2)") =>
            {
                self.report.check("unified_diff_drawn_on_keyboard_focus");
                self.phase = Phase::FlowFirst(Step::Class);
            }
            Step::ClassPreview if preview_drawn(runtime, "Leaf(2)") => {
                ensure!(
                    comparison.changes.preview.as_ref().is_some_and(|preview| {
                        matches!(
                            preview.target,
                            crate::flow_comparison::PreviewTarget::Row(_)
                        )
                    }),
                    "The visible diff is not a class preview"
                );
                ensure!(
                    observation::pair(runtime, &self.config.cases[0])?.is_some(),
                    "Opening a class diff replaced the selected flow"
                );
                self.report.check("class_preview_preserved_flow");
                self.select_case(0, Purpose::Cases);
            }
            _ => {}
        }
        Ok(())
    }
}

fn verify_outline(runtime: &Runtime<State>) -> Result<()> {
    let comparison = &runtime.state.flow_comparison;
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
        .find(|row| row.flow_available);
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
