//! Revision pickers and changed-method rows drive the saved comparison.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Editor {
    Entry,
    Base,
    PickBase,
    CloseBase,
    After,
    PickAfter,
    CloseAfter,
    Bound,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Selecting {
    Row,
    Pair,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Purpose {
    Cases,
    Inspect,
    Reloaded,
    Fresh,
}
impl Driver {
    pub(super) fn editor_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Editor,
    ) -> Result<()> {
        match step {
            Editor::Entry => self.click(runtime, "compare-flows", Phase::Editing(Editor::Base)),
            Editor::Base => {
                self.click(runtime, "comparison-base", Phase::Editing(Editor::PickBase))
            }
            Editor::PickBase => {
                ensure!(
                    runtime.find("comparison-revision-worktree").is_none(),
                    "Base picker offered a Worktree endpoint"
                );
                self.click(
                    runtime,
                    &format!("comparison-revision-{}", self.config.initial.base_commit),
                    Phase::Editing(Editor::CloseBase),
                );
            }
            Editor::CloseBase if commit_message_drawn(runtime, "fixture-before") => self.click(
                runtime,
                "comparison-revisions-done",
                Phase::Editing(Editor::After),
            ),
            Editor::After => self.click(
                runtime,
                "comparison-head",
                Phase::Editing(Editor::PickAfter),
            ),
            Editor::PickAfter if runtime.find("comparison-revision-worktree").is_some() => self
                .click(
                    runtime,
                    &format!("comparison-revision-{}", self.config.initial.head_commit),
                    Phase::Editing(Editor::CloseAfter),
                ),
            Editor::CloseAfter if commit_message_drawn(runtime, "fixture-after") => self.click(
                runtime,
                "comparison-revisions-done",
                Phase::Editing(Editor::Bound),
            ),
            Editor::Bound if runtime.state.flow_comparison.catalog.is_some() => {
                observation::bound(&runtime.state, &self.config.initial)?;
                self.report.check("commits_selected_by_controls");
                self.report.check("commit_messages_drawn_for_both_sides");
                self.phase = Phase::FlowFirst(super::flow_first::Step::outline());
            }
            _ => (),
        }
        Ok(())
    }
    pub(super) fn select_case(&mut self, index: usize, purpose: Purpose) {
        self.selected_case = index;
        self.purpose = purpose;
        self.phase = Phase::Selection(Selecting::Row);
    }

    pub(super) fn selection_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Selecting,
    ) -> Result<()> {
        if step != Selecting::Row {
            return Ok(());
        }
        let symbol = &self.config.cases[self.selected_case].symbol;
        let changes = &runtime.state.flow_comparison.changes;
        if !changes.loaded {
            return Ok(());
        }
        let change = changes
            .rows
            .iter()
            .find(|row| row.symbol.as_ref() == Some(symbol))
            .context("Git change list omitted the expected changed method")?;
        if let Some(key) = flow_first::change_key(runtime, &change.id) {
            self.click(runtime, &key, Phase::Selection(Selecting::Pair));
        } else {
            self.input
                .scroll_pane(runtime, "comparison-change-rows", 160.0);
        }
        Ok(())
    }

    pub(super) fn selection_observed(
        &mut self,
        runtime: &Runtime<State>,
        step: Selecting,
    ) -> Result<()> {
        if step == Selecting::Pair {
            self.pair_observed(runtime)?;
        }
        Ok(())
    }

    fn pair_observed(&mut self, runtime: &Runtime<State>) -> Result<()> {
        let case = &self.config.cases[self.selected_case];
        if observation::pair(runtime, case)?.is_none() {
            return Ok(());
        }
        observation::bound(&runtime.state, self.expected_binding())?;
        observation::controls(runtime)?;
        observation::provenance(runtime)?;
        observation::availability_drawn(runtime, case)?;
        self.report.check("provenance_drawn");
        self.report.check(&format!("{:?}_pair_exact", case.kind));
        self.report.observations.push(serde_json::json!({"case":format!("{:?}",case.kind),"symbol":case.symbol,"head_symbol":case.head_symbol,"comparison_id":self.expected_binding().comparison_id}));
        match self.purpose {
            Purpose::Cases if case.kind == CaseKind::Legacy => self.external(
                Operation::Capture,
                Checkpoint::UnavailableFlow,
                Phase::Interaction(Interaction::Start),
            )?,
            Purpose::Cases => self.select_case(
                if self.selected_case == 0 {
                    2 // Unchanged methods are intentionally absent from the sidebar.
                } else {
                    self.selected_case + 1
                },
                Purpose::Cases,
            ),
            Purpose::Inspect => self.phase = Phase::Interaction(Interaction::BeforeSelect),
            Purpose::Reloaded => self.phase = Phase::Cancellation(Cancellation::Arm),
            Purpose::Fresh => {
                self.report.check("fresh_request_after_cancel");
                if self.config.worktree_line.is_some() {
                    self.external(
                        Operation::EditWorktree,
                        Checkpoint::WorktreeEdited,
                        Phase::Worktree(worktree::Step::OpenAfter),
                    )?;
                } else {
                    self.phase = Phase::Cancellation(Cancellation::Close);
                }
            }
        }
        Ok(())
    }
}

fn commit_message_drawn(runtime: &Runtime<State>, subject: &str) -> bool {
    runtime.elements().iter().any(|element| {
        !element.exiting
            && element.visible[2].min(element.visible[3]) > 0.0
            && element.path.contains("/comparison-revision-message")
            && element.text.as_ref().is_some_and(|message| {
                message.contains(subject) && message.contains("Compiler-backed comparison fixture.")
            })
    })
}
