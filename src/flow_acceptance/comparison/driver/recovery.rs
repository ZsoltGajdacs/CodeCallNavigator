//! A moved tag and an acknowledged owned Git gate exercise real reload and cancellation.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RefMove {
    Move,
    Reload,
    Pinned,
    OpenAfter,
    PickAfter,
    CloseAfter,
    Load,
    Loaded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Cancellation {
    Arm,
    OpenCatalog,
    Search,
    WaitGate,
    CloseCatalog,
    Cancel,
    Cancelled,
    LoadFresh,
    Loaded,
    Close,
    Closed,
}
impl Driver {
    pub(super) fn ref_move_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: RefMove,
    ) -> Result<()> {
        match step {
            RefMove::Move => self.external(
                Operation::MoveAfterRef,
                Checkpoint::AfterRefMoved,
                Phase::RefMove(RefMove::Reload),
            )?,
            RefMove::Reload => self.click(
                runtime,
                "comparison-reload",
                Phase::RefMove(RefMove::Pinned),
            ),
            RefMove::OpenAfter => self.click(
                runtime,
                "comparison-head",
                Phase::RefMove(RefMove::PickAfter),
            ),
            RefMove::PickAfter => self.click(
                runtime,
                &format!("comparison-revision-{}", self.config.reloaded.head_commit),
                Phase::RefMove(RefMove::CloseAfter),
            ),
            RefMove::CloseAfter => self.click(
                runtime,
                "comparison-revisions-done",
                Phase::RefMove(RefMove::Load),
            ),
            RefMove::Load => {
                self.click(runtime, "comparison-load", Phase::RefMove(RefMove::Loaded))
            }
            _ => (),
        }
        Ok(())
    }
    pub(super) fn ref_move_observed(
        &mut self,
        runtime: &Runtime<State>,
        step: RefMove,
    ) -> Result<()> {
        match step {
            RefMove::Pinned if observation::bound(&runtime.state, &self.config.initial)? => {
                self.report.check("moved_tag_left_selected_commit_pinned");
                self.phase = Phase::RefMove(RefMove::OpenAfter);
            }
            RefMove::Loaded if observation::bound(&runtime.state, &self.config.reloaded)? => {
                self.report.check("selected_later_commit_rebound");
                self.select_case(0, Purpose::Reloaded);
            }
            _ => (),
        }
        Ok(())
    }
    pub(super) fn cancel_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Cancellation,
    ) -> Result<()> {
        match step {
            Cancellation::Arm => self.external(
                Operation::ArmGitGate,
                Checkpoint::GateArmed,
                Phase::Cancellation(Cancellation::OpenCatalog),
            )?,
            Cancellation::OpenCatalog => self.click(
                runtime,
                "comparison-catalog-open",
                Phase::Cancellation(Cancellation::Search),
            ),
            Cancellation::Search => self.click(
                runtime,
                "comparison-search-submit",
                Phase::Cancellation(Cancellation::WaitGate),
            ),
            Cancellation::CloseCatalog => self.click(
                runtime,
                "comparison-catalog-close",
                Phase::Cancellation(Cancellation::Cancel),
            ),
            Cancellation::Cancel => self.click(
                runtime,
                "comparison-cancel",
                Phase::Cancellation(Cancellation::Cancelled),
            ),
            Cancellation::LoadFresh => self.click(
                runtime,
                "comparison-load",
                Phase::Cancellation(Cancellation::Loaded),
            ),
            Cancellation::Close => self.click(
                runtime,
                "comparison-close",
                Phase::Cancellation(Cancellation::Closed),
            ),
            _ => (),
        }
        Ok(())
    }
    pub(super) fn cancel_observed(
        &mut self,
        runtime: &Runtime<State>,
        step: Cancellation,
    ) -> Result<()> {
        match step {
            Cancellation::WaitGate => {
                if runtime.state.flow_comparison.pending() {
                    self.external(
                        Operation::WaitGitGate,
                        Checkpoint::GateHeld,
                        Phase::Cancellation(Cancellation::CloseCatalog),
                    )?;
                }
            }
            Cancellation::Cancelled => {
                ensure!(
                    runtime.state.flow_comparison.current.is_none(),
                    "Cancel retained an actionable Pair"
                );
                if !runtime.state.flow_comparison.stopped() {
                    return Ok(());
                }
                self.report.check("cancel_waited_for_owned_query_cleanup");
                self.external(
                    Operation::ReleaseGitGate,
                    Checkpoint::GateReleased,
                    Phase::Cancellation(Cancellation::LoadFresh),
                )?;
            }
            Cancellation::Loaded => {
                if runtime.state.flow_comparison.catalog.is_some() {
                    observation::bound(&runtime.state, &self.config.reloaded)?;
                    self.select_case(0, Purpose::Fresh);
                }
            }
            Cancellation::Closed => {
                ensure!(
                    runtime.state.graph.is_none()
                        && !runtime.state.flow_comparison.controls.visible,
                    "Comparison did not return to its no-graph workspace"
                );
                self.report.check("no_graph_returned");
                self.finish(true)?;
            }
            _ => (),
        }
        Ok(())
    }
}
