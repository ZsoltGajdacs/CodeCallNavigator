//! Exact-ID catalog selection and deliberate counterpart controls.
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
    Load,
    Catalog,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Selecting {
    Open,
    Edit,
    Submit,
    Wait,
    Record,
    Pair,
    ChooseManual,
    Reset,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Purpose {
    Cases,
    ManualBase,
    Manual,
    Reset,
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
                Phase::Editing(Editor::Load),
            ),
            Editor::Load => self.click(runtime, "comparison-load", Phase::Editing(Editor::Catalog)),
            Editor::Catalog if runtime.state.flow_comparison.catalog.is_some() => {
                observation::bound(&runtime.state, &self.config.initial)?;
                self.report.check("commits_selected_by_controls");
                self.report.check("commit_messages_drawn_for_both_sides");
                self.phase = Phase::FlowFirst(super::flow_first::Step::outline());
            }
            _ => (),
        }
        Ok(())
    }
    fn query_symbol(&self) -> &str {
        let case = &self.config.cases[self.selected_case];
        case.head_symbol.as_deref().unwrap_or(&case.symbol)
    }
    pub(super) fn select_case(&mut self, index: usize, purpose: Purpose) {
        self.selected_case = index;
        self.purpose = purpose;
        self.phase = Phase::Selection(Selecting::Open);
    }
    pub(super) fn selection_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Selecting,
    ) -> Result<()> {
        match step {
            Selecting::Open => {
                if runtime.state.flow_comparison.controls.catalog_open {
                    self.phase = Phase::Selection(Selecting::Edit);
                } else {
                    self.click(
                        runtime,
                        "comparison-catalog-open",
                        Phase::Selection(Selecting::Edit),
                    );
                }
            }
            Selecting::Edit => {
                let symbol = self.query_symbol().to_owned();
                if self.input.edit(runtime, "comparison-search", &symbol) {
                    self.phase = Phase::Selection(Selecting::Submit);
                }
            }
            Selecting::Submit => {
                ensure!(
                    runtime.state.flow_comparison.controls.search.text == self.query_symbol(),
                    "Actual catalog editor did not accept the exact ID"
                );
                self.click(
                    runtime,
                    "comparison-search-submit",
                    Phase::Selection(Selecting::Wait),
                );
            }
            Selecting::Record => {
                let key = format!("comparison-record-{}", self.query_symbol());
                self.click(runtime, &key, Phase::Selection(Selecting::Pair));
            }
            Selecting::ChooseManual => {
                self.selected_case = 5;
                self.purpose = Purpose::Manual;
                self.click(
                    runtime,
                    "comparison-choose-after",
                    Phase::Selection(Selecting::Edit),
                );
            }
            Selecting::Reset => {
                self.selected_case = 0;
                self.purpose = Purpose::Reset;
                self.click(
                    runtime,
                    "comparison-reset-counterpart",
                    Phase::Selection(Selecting::Pair),
                );
            }
            _ => (),
        }
        Ok(())
    }
    pub(super) fn selection_observed(
        &mut self,
        runtime: &Runtime<State>,
        step: Selecting,
    ) -> Result<()> {
        if step == Selecting::Wait {
            if runtime.state.flow_comparison.catalog_pending() {
                return Ok(());
            }
            let page = runtime
                .state
                .flow_comparison
                .catalog
                .as_ref()
                .context("Native catalog is unavailable")?;
            // Latest can settle between UI polling and Drawn observation. Wait
            // for the requested page to be published before checking its IDs.
            if page.text != self.query_symbol() || page.offset != 0 {
                return Ok(());
            }
            ensure!(
                page.records
                    .iter()
                    .any(|record| record.symbol == self.query_symbol()),
                "Exact catalog search did not return its expected ID"
            );
            self.phase = Phase::Selection(Selecting::Record);
        } else if step == Selecting::Pair {
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
                Phase::Interaction(Interaction::StartManual),
            )?,
            Purpose::Cases => self.select_case(self.selected_case + 1, Purpose::Cases),
            Purpose::ManualBase => self.phase = Phase::Selection(Selecting::ChooseManual),
            Purpose::Manual => {
                ensure!(
                    runtime.elements().iter().any(|el| el
                        .text
                        .as_deref()
                        .is_some_and(|s| s.contains("Manual counterpart"))),
                    "Manual status is not drawn"
                );
                self.external(
                    Operation::Capture,
                    Checkpoint::ManualPair,
                    Phase::Selection(Selecting::Reset),
                )?;
            }
            Purpose::Reset => self.phase = Phase::Interaction(Interaction::BeforeSelect),
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
