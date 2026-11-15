//! Moved refs and an externally acknowledged owned Git gate exercise real reload/cancel paths.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mismatch {
    Move,
    OpenCatalog,
    Search,
    Wait,
    Reload,
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
    pub(super) fn mismatch_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Mismatch,
    ) -> Result<()> {
        match step {
            Mismatch::Move => self.external(
                Operation::MoveAfterRef,
                Checkpoint::AfterRefMoved,
                Phase::Mismatch(Mismatch::OpenCatalog),
            )?,
            Mismatch::OpenCatalog => self.click(
                runtime,
                "comparison-catalog-open",
                Phase::Mismatch(Mismatch::Search),
            ),
            Mismatch::Search => self.click(
                runtime,
                "comparison-search-submit",
                Phase::Mismatch(Mismatch::Wait),
            ),
            Mismatch::Reload => self.click(
                runtime,
                "comparison-reload",
                Phase::Mismatch(Mismatch::Loaded),
            ),
            _ => (),
        }
        Ok(())
    }
    pub(super) fn mismatch_observed(
        &mut self,
        runtime: &Runtime<State>,
        step: Mismatch,
    ) -> Result<()> {
        match step {
            Mismatch::Wait => {
                if runtime.state.flow_comparison.pending() {
                    return Ok(());
                }
                ensure!(
                    matches!(
                        runtime.state.flow_comparison.phase,
                        crate::flow_comparison::Phase::ReloadRequired(_)
                    ),
                    "Moved ref did not require explicit Reload"
                );
                ensure!(
                    runtime.state.flow_comparison.current.is_none()
                        && runtime.state.flow_comparison.catalog.is_none()
                        && runtime.state.flow_comparison.history.is_empty(),
                    "Mismatch retained actionable comparison content"
                );
                self.report.check("mismatch_cleared_actionable_content");
                self.phase = Phase::Mismatch(Mismatch::Reload);
            }
            Mismatch::Loaded if runtime.state.flow_comparison.catalog.is_some() => {
                observation::bound(&runtime.state, &self.config.reloaded)?;
                self.report.check("explicit_reload_rebound");
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
