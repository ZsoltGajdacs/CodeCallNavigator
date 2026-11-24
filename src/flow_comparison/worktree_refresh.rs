//! Tracks a Worktree snapshot preparation through shared analysis and comparison binding.
use super::{
    Project, Workspace,
    requests::{CatalogOperation, CatalogWork},
};
use crate::{app::State, engine_client::flow_comparison::ComparisonRefs, jobs::AnalysisPurpose};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RefreshTicket(u64);

#[derive(Default)]
pub(crate) struct Refresh {
    serial: u64,
    active: Option<Intent>,
    pub(crate) message: String,
}

struct Intent {
    project: PathBuf,
    refs: ComparisonRefs,
    stage: Stage,
}

enum Stage {
    Confirming,
    Analyzing,
    Opening,
}

impl Refresh {
    pub(crate) fn pending(&self) -> bool {
        self.active.is_some()
    }

    pub(crate) fn showing_progress(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|intent| matches!(intent.stage, Stage::Analyzing))
    }

    fn opening(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|intent| matches!(intent.stage, Stage::Opening))
    }

    pub(crate) fn matches(&self, ticket: RefreshTicket) -> bool {
        self.serial == ticket.0 && self.pending()
    }

    fn begin(&mut self, project: PathBuf, refs: ComparisonRefs) -> RefreshTicket {
        self.serial += 1;
        self.active = Some(Intent {
            project,
            refs,
            stage: Stage::Confirming,
        });
        self.message.clear();
        RefreshTicket(self.serial)
    }

    fn end(&mut self, message: String) {
        self.active = None;
        self.message = message;
    }
}

impl Workspace {
    pub(crate) fn worktree_snapshot(&self) -> Option<super::WorktreeSnapshot> {
        use crate::engine_client::flow_comparison::{EndpointKind, EndpointStatus};
        let binding = self.binding.as_ref()?;
        if binding.header().head.kind != EndpointKind::Worktree {
            return None;
        }
        match &self.catalog.as_ref()?.endpoints.after {
            EndpointStatus::Available => Some(super::WorktreeSnapshot::Saved(
                self.class_pair()
                    .and_then(|pair| pair.freshness)
                    .or(binding.freshness),
            )),
            EndpointStatus::Unavailable(failure) if failure.reason == "graph_missing" => {
                Some(super::WorktreeSnapshot::Missing)
            }
            EndpointStatus::Unavailable(_) => None,
        }
    }

    pub(super) fn require_idle_worktree_refresh(&self) -> Result<(), String> {
        if self.worktree_refresh.pending() {
            Err("Wait for Worktree refresh to finish".into())
        } else {
            Ok(())
        }
    }

    pub(super) fn invalidate_worktree_refresh(&mut self) {
        if self.worktree_refresh.opening() {
            self.catalog_clock.cancel();
            self.catalog_job.cancel();
        }
        self.worktree_refresh.end(String::new());
    }

    pub(super) fn finish_worktree_refresh(&mut self, ticket: RefreshTicket, message: String) {
        if self.worktree_refresh.matches(ticket) {
            self.worktree_refresh.end(message);
        }
    }
}

impl State {
    pub(crate) fn can_refresh_comparison_worktree(&self) -> bool {
        let workspace = &self.flow_comparison;
        workspace.revisions.worktree()
            && workspace.revisions.base().is_some()
            && !workspace.worktree_refresh.pending()
            && !workspace.pending()
            && self.can_review_analysis_builds()
    }

    pub(crate) fn refresh_comparison_worktree(&mut self) {
        if !self.can_refresh_comparison_worktree() {
            return;
        }
        let Some(project) = self.flow_comparison.revisions.physical_project() else {
            return;
        };
        let refs = self.flow_comparison.control_refs();
        let ticket = self
            .flow_comparison
            .worktree_refresh
            .begin(project.to_owned(), refs);
        self.review_analysis_builds_for(AnalysisPurpose::CompareWorktree(ticket));
    }

    pub(crate) fn start_comparison_worktree_analysis(&mut self, ticket: RefreshTicket) -> bool {
        let workspace = &mut self.flow_comparison;
        if !workspace.worktree_refresh.matches(ticket) {
            return false;
        }
        workspace.previews.close();
        workspace
            .worktree_refresh
            .active
            .as_mut()
            .expect("matched refresh")
            .stage = Stage::Analyzing;
        true
    }

    pub(crate) fn comparison_worktree_analysis_failed(
        &mut self,
        ticket: RefreshTicket,
        message: String,
    ) {
        self.flow_comparison
            .finish_worktree_refresh(ticket, message);
    }

    pub(crate) fn cancel_comparison_worktree_refresh(&mut self) {
        if self.flow_comparison.worktree_refresh.showing_progress() {
            self.cancel_analysis();
        }
        self.flow_comparison.invalidate_worktree_refresh();
        self.flow_comparison.worktree_refresh.message = "Snapshot preparation cancelled".into();
    }

    pub(crate) fn comparison_worktree_published(
        &mut self,
        ticket: RefreshTicket,
        project: Option<&Path>,
        generation: String,
    ) {
        let workspace = &self.flow_comparison;
        let Some(intent) = workspace
            .worktree_refresh
            .active
            .as_ref()
            .filter(|_| workspace.worktree_refresh.matches(ticket))
        else {
            return;
        };
        if Some(intent.project.as_path()) != project || intent.refs != workspace.control_refs() {
            self.flow_comparison
                .finish_worktree_refresh(ticket, "Worktree snapshot prepared.".into());
            return;
        }
        let refs = intent.refs.clone();
        let (runtime, context) = match self.comparison_context() {
            Ok(context) => context,
            Err(error) => {
                self.flow_comparison.finish_worktree_refresh(ticket, error);
                return;
            }
        };
        let workspace = &mut self.flow_comparison;
        workspace
            .worktree_refresh
            .active
            .as_mut()
            .expect("matched refresh")
            .stage = Stage::Opening;
        workspace.worktree_refresh.message = "Opening the prepared Worktree snapshot…".into();
        let cancel = workspace.catalog_clock.restart();
        let serial = workspace.catalog_clock.serial;
        let request = workspace.workers.track(CatalogWork {
            serial,
            cancel,
            operation: CatalogOperation::Refresh {
                project: Box::new(Project {
                    context,
                    refs,
                    runtime,
                }),
                ticket,
                generation,
            },
        });
        workspace
            .catalog_job
            .request(request, |work| work.map(CatalogWork::run));
    }
}

impl Workspace {
    pub(super) fn adopt_refreshed_comparison(
        &mut self,
        project: Project,
        bound: super::BoundComparison,
        page: super::Catalog,
    ) {
        self.cancel_requests();
        self.project = Some(project);
        self.current = None;
        self.history.clear();
        self.pending_scroll = None;
        self.mounted_source = None;
        self.detail_message.clear();
        self.controls.reset_binding();
        self.binding = Some(bound);
        self.catalog = Some(std::sync::Arc::new(page));
        self.phase = super::Phase::Bound;
        self.open_change_list();
    }
}
