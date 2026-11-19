//! The impact preview owns a read-only CLI request for the entered revisions.
use super::{Panel, Workspace, work};
use crate::{
    app::State,
    engine_client::{
        QueryContext,
        flow_comparison::{ImpactPreview, ImpactRefs, installed_impact_preview},
    },
    runtime::Runtime,
};
use rune::{Latest, Waker};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
pub struct Impact {
    pub report: Option<ImpactPreview>,
    pub message: String,
    pub supported: Option<bool>,
    capability_engine: Option<PathBuf>,
    capability_job: Latest<work::Tracked<anyhow::Result<bool>>>,
    capability_cancel: Arc<AtomicBool>,
    job: Latest<work::Tracked<anyhow::Result<ImpactPreview>>>,
    cancel: Arc<AtomicBool>,
    project: Option<PathBuf>,
    captured: Option<ImpactRefs>,
}

impl State {
    pub(crate) fn check_impact_preview_support(&mut self) {
        if let Some(runtime) = &self.runtime {
            self.flow_comparison
                .impact
                .check_support(runtime.clone(), &self.flow_comparison.workers);
        }
    }

    pub fn preview_changed_symbol_impact(&mut self) {
        let workspace = &self.flow_comparison;
        if !workspace.can_preview_impact() {
            return;
        }
        self.flow_residency.activity();
        let context = self.library_comparison_context();
        let refs = workspace.control_refs().into();
        let workspace = &mut self.flow_comparison;
        workspace.controls.catalog_open = false;
        workspace.controls.panel = Panel::Impact;
        match context {
            Ok(context) => workspace.impact.start(context, refs, &workspace.workers),
            Err(error) => workspace.impact.message = error,
        }
    }
}

impl Workspace {
    pub fn can_preview_impact(&self) -> bool {
        self.impact.supported == Some(true) && !self.impact.pending() && !self.snapshots.pending()
    }

    pub(super) fn poll_impact(&mut self) -> bool {
        let refs = self
            .impact
            .captured
            .as_ref()
            .map(|_| self.control_refs().into());
        self.impact.poll(refs.as_ref())
    }
}

impl Impact {
    fn check_support(&mut self, runtime: Arc<Runtime>, workers: &Arc<work::Workers>) {
        if self.capability_engine.as_ref() == Some(&runtime.engine) && self.checking_support() {
            return;
        }
        self.capability_cancel.store(true, Ordering::Release);
        self.capability_cancel = Arc::new(AtomicBool::new(false));
        self.capability_engine = Some(runtime.engine.clone());
        self.supported = None;
        let request = workers.track((runtime, self.capability_cancel.clone()));
        self.capability_job.request(request, |request| {
            request.map(|(runtime, cancel)| installed_impact_preview(&runtime, &cancel))
        });
    }

    pub fn checking_support(&self) -> bool {
        self.capability_job.is_pending()
    }

    pub fn pending(&self) -> bool {
        self.job.is_pending()
    }

    pub(super) fn set_waker(&self, waker: Waker) {
        self.capability_job.set_waker(waker.clone());
        self.job.set_waker(waker);
    }

    fn start(&mut self, context: QueryContext, refs: ImpactRefs, workers: &Arc<work::Workers>) {
        self.clear();
        self.cancel = Arc::new(AtomicBool::new(false));
        self.project = Some(context.descriptor.project.clone());
        self.captured = Some(refs.clone());
        self.message = "Previewing changed-symbol impact…".into();
        let request = workers.track((context, refs, self.cancel.clone()));
        self.job.request(request, |request| {
            request.map(|(context, refs, cancel)| context.impact_preview(refs, &cancel))
        });
    }

    pub(super) fn poll(&mut self, references: Option<&ImpactRefs>) -> bool {
        let mut changed = false;
        if let Some(response) = self.capability_job.take() {
            self.supported = Some(response.value.unwrap_or(false));
            changed = true;
        }
        if self.captured.as_ref() != references {
            self.clear();
            self.message = "Preview again for the edited revisions or base snapshot".into();
            return true;
        }
        if let Some(response) = self.job.take() {
            match response.value {
                Ok(report) => {
                    self.report = Some(report);
                    self.message.clear();
                }
                Err(error) => self.message = format!("{error:#}"),
            }
            return true;
        }
        changed
    }

    pub(super) fn project_installed(&mut self, project: Option<&Path>) {
        if self
            .project
            .as_deref()
            .is_some_and(|previous| Some(previous) != project)
        {
            self.clear();
        }
    }

    pub(super) fn cancel(&mut self) {
        self.clear();
        self.message = "Impact preview cancelled".into();
    }

    pub(super) fn clear(&mut self) {
        self.cancel.store(true, Ordering::Release);
        self.job.cancel();
        self.report = None;
        self.project = None;
        self.captured = None;
        self.message.clear();
    }

    pub(super) fn close(&mut self) {
        if self.checking_support() {
            self.capability_cancel.store(true, Ordering::Release);
            self.capability_job.cancel();
        }
        self.clear();
    }
}
