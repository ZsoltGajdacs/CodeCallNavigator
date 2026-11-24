//! Publish only matching request serials and bindings, clearing stale actionable results.
use super::{
    Phase, Workspace,
    requests::{CatalogAnswer, DetailSelection, PreparedDetail, StringError},
};
impl Workspace {
    pub fn poll(&mut self) -> bool {
        let identity = self.identity.poll(self.catalog_clock.serial);
        if identity == Some(super::identity::Installation::Different) {
            self.close();
            return true;
        }
        let catalog = self.publish_catalog();
        let detail = self.publish_detail();
        identity.is_some()
            | catalog
            | detail
            | self.revisions.poll()
            | self.publish_snapshot()
            | self.publish_changes()
            | self.publish_previews()
    }
    fn publish_snapshot(&mut self) -> bool {
        use super::snapshots::Poll;
        let result = self.snapshots.poll();
        if result == Poll::Rebind {
            self.rebind_after_snapshot();
        }
        result != Poll::Idle
    }
    fn rebind_after_snapshot(&mut self) {
        let Some(project) = &self.project else {
            return;
        };
        let Some(side) = self.snapshots.side else {
            return;
        };
        let mut refs = project.refs.clone();
        let selected = self.control_refs();
        if side.reference(&refs) != side.reference(&selected) {
            return;
        }
        match side {
            super::SnapshotSide::Base => refs.base_snapshot = selected.base_snapshot,
            super::SnapshotSide::Head => refs.head_snapshot = selected.head_snapshot,
        }
        if let Err(error) = self.load_refs(refs) {
            self.controls.message = error;
        }
    }
    fn publish_catalog(&mut self) -> bool {
        let Some(output) = self.catalog_job.take() else {
            return false;
        };
        output.map(|output| self.accept_catalog(output)).value
    }
    fn accept_catalog(&mut self, output: super::requests::CatalogOutput) -> bool {
        if output.serial != self.catalog_clock.serial {
            return false;
        }
        if let Some(ticket) = output.refresh {
            return self.publish_worktree_refresh(ticket, output);
        }
        if self.identity.completed(output.physical_project.as_deref())
            == super::identity::Installation::Different
        {
            // Discard every outcome while Project still pins the client, before retirement.
            drop(output);
            self.close();
            return true;
        }
        match output.result {
            Ok(CatalogAnswer::Opened(answer)) => {
                let super::requests::Opened { bound, page } = *answer;
                self.binding = Some(bound);
                self.catalog = Some(std::sync::Arc::new(page));
                self.phase = Phase::Bound;
                self.graph_catalog_ready();
            }
            Ok(CatalogAnswer::Refreshed { .. }) => unreachable!("refresh requires ticket"),
            Err(error) => {
                self.failed(StringError::Query(error));
                self.resolve_pending_class();
            }
        }
        true
    }
    fn publish_worktree_refresh(
        &mut self,
        ticket: super::RefreshTicket,
        output: super::requests::CatalogOutput,
    ) -> bool {
        if !self.worktree_refresh.matches(ticket) {
            return false;
        }
        let message = match output.result {
            Ok(CatalogAnswer::Refreshed { project, opened }) => {
                if project.refs != self.control_refs() {
                    self.finish_worktree_refresh(ticket, "Worktree snapshot prepared.".into());
                    return true;
                }
                if self.identity.completed(output.physical_project.as_deref())
                    == super::identity::Installation::Different
                {
                    self.close();
                    return true;
                }
                self.finish_worktree_refresh(ticket, String::new());
                self.snapshots.worktree = None;
                self.adopt_refreshed_comparison(*project, opened.bound, opened.page);
                return true;
            }
            Err(error) => error.to_string(),
            Ok(_) => unreachable!("refresh returned a normal catalog"),
        };
        self.finish_worktree_refresh(ticket, message);
        true
    }
    fn publish_detail(&mut self) -> bool {
        let Some(output) = self.detail_job.take() else {
            return false;
        };
        output.map(|output| self.accept_detail(output)).value
    }
    fn accept_detail(&mut self, output: super::requests::DetailOutput) -> bool {
        let current_request = output.serial == self.detail_clock.serial
            && self.detail_selection.as_ref() == Some(&output.selection);
        let current_binding = self
            .header()
            .is_some_and(|header| header.comparison_id == output.binding);
        if !current_request || !current_binding {
            return false;
        }
        match (output.selection, output.result) {
            (DetailSelection::Function(selection), Ok(PreparedDetail::Function(prepared))) => {
                self.visit += 1;
                self.presentation_revision += 1;
                let namespace = format!("comparison:{}:{}", output.binding, self.visit);
                self.current = Some(prepared.install(selection, &namespace, self.viewports));
                self.detail_message.clear();
                self.arm_source_restore();
                self.prefetch_selected_source();
            }
            (DetailSelection::Class { .. }, Ok(PreparedDetail::Class(prepared))) => {
                let mut pair = *prepared;
                for side in [super::SideId::Before, super::SideId::After] {
                    pair.fit(side, self.viewports[side.index()]);
                }
                self.controls.maximum_class_depth = pair.maximum_depth;
                self.class_pair = Some(pair);
                self.detail_message.clear();
                self.presentation_revision += 1;
            }
            (DetailSelection::Class { .. }, Err(error)) if !error.mismatch() => {
                self.detail_message = error.message();
            }
            (_, Err(error)) => self.failed(error),
            _ => unreachable!("prepared detail must match its selection"),
        }
        true
    }
    pub(super) fn failed(&mut self, error: StringError) {
        if self.changes.uses_legacy_graph() {
            self.cancel_requests();
        } else {
            self.cancel_graph_requests();
        }
        self.binding = None;
        self.catalog = None;
        self.current = None;
        self.history.clear();
        self.detail_message.clear();
        self.pending_scroll = None;
        self.phase = if error.mismatch() {
            Phase::ReloadRequired(error.message())
        } else {
            Phase::Failed(error.message())
        };
    }
}
#[cfg(test)]
#[path = "publication_race_tests.rs"]
mod race_tests;
