//! Editable comparison inputs never retarget an already captured revision pair.
use super::{Phase, SideId, Workspace};
use crate::app::State;
pub struct Controls {
    pub visible: bool,
    pub class_depth_open: Option<SideId>,
    pub(super) class_depth: u32,
    pub(super) maximum_class_depth: u32,
    pub message: String,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            visible: false,
            class_depth_open: None,
            class_depth: 1,
            maximum_class_depth: 1,
            message: String::new(),
        }
    }
}
impl State {
    pub(crate) fn cancel_comparison_action(&mut self) {
        if self.flow_comparison.worktree_refresh.pending() {
            self.cancel_comparison_worktree_refresh();
        } else {
            self.flow_comparison.cancel_control();
        }
    }

    pub fn show_flow_comparison(&mut self) {
        self.flow_residency.activity();
        self.close_source();
        self.flow_workspace.preserve_current_scroll();
        self.flow_comparison.controls.visible = true;
        self.start_comparison_history();
    }
    pub fn load_flow_comparison(&mut self) {
        self.flow_comparison.invalidate_worktree_refresh();
        self.flow_residency.activity();
        let refs = self.flow_comparison.control_refs();
        let result = if self.flow_comparison.descriptor().is_some() {
            self.flow_comparison.load_refs(refs)
        } else {
            self.open_selected_flow_comparison(refs)
        };
        if let Err(error) = result {
            self.flow_comparison.controls.message = error;
        }
    }
}
impl Controls {
    pub(super) fn reset_binding(&mut self) {
        self.message.clear();
    }
}
impl Workspace {
    pub fn cancel_control(&mut self) {
        if self.snapshots.pending() {
            self.snapshots.cancel();
            return;
        }
        self.cancel_requests();
        self.identity.reset();
        self.phase = Phase::Closed;
        self.controls.message = "Comparison loading cancelled. Use Find snapshot to reopen.".into();
    }
}
