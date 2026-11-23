//! Compare the compiler's newly published worktree graph through the actual refresh controls.
use super::*;
use crate::engine_client::flow_comparison::EndpointKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    OpenAfter,
    Pick,
    Done,
    ReviewCancel,
    Cancel,
    Cancelled,
    Review,
    Confirm,
    Published,
    SelectChanged,
    Hover,
    Preview,
    Close,
    Closed,
}

impl Driver {
    pub(super) fn worktree_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Step,
    ) -> Result<()> {
        let (key, next) = match step {
            Step::OpenAfter => ("comparison-head", Step::Pick),
            Step::Pick => ("comparison-revision-worktree", Step::Done),
            Step::Done => ("comparison-revisions-done", Step::ReviewCancel),
            Step::ReviewCancel if runtime.state.can_refresh_comparison_worktree() => {
                ("comparison-refresh-worktree", Step::Cancel)
            }
            Step::Cancel => ("cancel-analysis-confirmation", Step::Cancelled),
            Step::Review if runtime.state.can_refresh_comparison_worktree() => {
                ("comparison-refresh-worktree", Step::Confirm)
            }
            Step::Confirm if runtime.state.can_confirm_analysis() => {
                ("confirm-analysis", Step::Published)
            }
            Step::SelectChanged => return self.select_worktree_change(runtime),
            Step::Hover => {
                if !runtime
                    .state
                    .flow_comparison
                    .current
                    .as_ref()
                    .is_some_and(|pair| pair.selection.symbol == self.config.cases[0].symbol)
                {
                    return Ok(());
                }
                let key = self.call_key(runtime)?;
                if self.input.hover(runtime, &key) {
                    self.phase = Phase::Worktree(Step::Preview);
                }
                return Ok(());
            }
            Step::Close => ("comparison-close", Step::Closed),
            _ => return Ok(()),
        };
        self.click(runtime, key, Phase::Worktree(next));
        Ok(())
    }

    fn select_worktree_change(&mut self, runtime: &mut Runtime<State>) -> Result<()> {
        let comparison = &runtime.state.flow_comparison;
        if !comparison.changes.loaded {
            return Ok(());
        }
        let symbol = &self.config.cases[0].symbol;
        let change = comparison
            .changes
            .rows
            .iter()
            .find(|row| row.symbol.as_ref() == Some(symbol))
            .context("The uncommitted worktree edit did not appear in the change outline")?;
        if let Some(key) = flow_first::change_key(runtime, &change.id) {
            self.click(runtime, &key, Phase::Worktree(Step::Hover));
        }
        Ok(())
    }

    pub(super) fn worktree_observed(&mut self, runtime: &Runtime<State>, step: Step) -> Result<()> {
        let comparison = &runtime.state.flow_comparison;
        match step {
            Step::Cancelled => {
                ensure!(
                    observation::bound(&runtime.state, &self.config.reloaded)?
                        && observation::pair(runtime, &self.config.cases[0])?.is_some()
                        && runtime.state.graph.is_none(),
                    "Cancelling worktree build confirmation did not preserve the committed comparison"
                );
                self.report
                    .check("worktree_confirmation_cancel_preserved_pair");
                self.phase = Phase::Worktree(Step::Review);
            }
            Step::Published => return self.worktree_published(runtime),
            Step::Preview
                if self
                    .config
                    .worktree_line
                    .as_ref()
                    .is_some_and(|line| flow_first::preview_drawn(runtime, line)) =>
            {
                self.report
                    .check("worktree_refresh_rendered_uncommitted_diff");
                self.phase = Phase::Worktree(Step::Close);
            }
            Step::Closed => {
                ensure!(
                    runtime.state.graph.is_some() && !comparison.controls.visible,
                    "Leaving Compare did not return to the refreshed graph"
                );
                self.report.check("worktree_graph_returned");
                self.finish(true)?;
            }
            _ => {}
        }
        Ok(())
    }
    fn worktree_published(&mut self, runtime: &Runtime<State>) -> Result<()> {
        let comparison = &runtime.state.flow_comparison;
        let Some(header) = comparison
            .header()
            .filter(|header| header.head.kind == EndpointKind::Worktree)
        else {
            ensure!(
                comparison.worktree_refresh.pending()
                    || comparison.worktree_refresh.message.is_empty(),
                "Worktree refresh failed: {}",
                comparison.worktree_refresh.message
            );
            return Ok(());
        };
        ensure!(
            header.base.commit.as_deref() == Some(self.config.initial.base_commit.as_str())
                && runtime.state.generation.as_deref() == Some(header.head.digest.as_str()),
            "Worktree comparison did not bind to the selected base and published graph"
        );
        self.phase = Phase::Worktree(Step::SelectChanged);
        Ok(())
    }
}
