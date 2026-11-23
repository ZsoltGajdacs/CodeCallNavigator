//! Historical flows own their captured project and endpoint binding, not a graph epoch.
mod call_changes;
mod changes;
mod controls;
mod identity;
mod impact;
mod lifecycle;
mod navigation;
mod pair;
mod pane_geometry;
mod publication;
mod requests;
mod snapshots;
mod source_navigation;
mod work;
use crate::engine_client::{
    QueryContext,
    flow_comparison::{BoundComparison, Catalog, ComparisonRefs},
};
pub(crate) use changes::{Changes, PreviewTarget, StepChange};
pub use pair::{Pair, ReadySide, Side, SideId};
use requests::{CatalogOutput, Clock, DetailOutput};
use rune::{Latest, Waker};
pub(crate) use snapshots::SELECT_BUILDS;
pub use snapshots::{Navigation as GraphNavigation, Side as SnapshotSide};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub symbol: String,
    pub head_symbol: Option<String>,
}
#[derive(Debug, Default)]
pub enum Phase {
    #[default]
    Closed,
    Loading,
    Bound,
    ReloadRequired(String),
    Failed(String),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Panel {
    #[default]
    Flows,
    Graph,
    Impact,
}
#[derive(Clone)]
struct Project {
    context: QueryContext,
    refs: ComparisonRefs,
    runtime: crate::runtime::Runtime,
}
#[derive(Default)]
pub struct Workspace {
    pub controls: controls::Controls,
    pub snapshots: snapshots::Snapshots,
    pub impact: impact::Impact,
    pub changes: changes::Changes,
    project: Option<Project>,
    identity: identity::Identity,
    binding: Option<BoundComparison>,
    pub phase: Phase,
    pub catalog: Option<Arc<Catalog>>,
    pub current: Option<Pair>,
    pub history: Vec<Pair>,
    pub detail_message: String,
    catalog_job: Latest<work::Tracked<CatalogOutput>>,
    detail_job: Latest<work::Tracked<DetailOutput>>,
    catalog_clock: Clock,
    detail_clock: Clock,
    workers: Arc<work::Workers>,
    visit: u64,
    presentation_revision: u64,
    pending_scroll: Option<source_navigation::ScrollRequest>,
    mounted_source: Option<String>,
    pub viewports: [[f32; 2]; 2],
}
impl Workspace {
    pub(crate) fn release_hidden(&mut self) {
        self.cancel_requests();
        self.impact.close();
        self.snapshots.reset();
        self.current = None;
        self.history.clear();
        self.catalog = None;
        self.pending_scroll = None;
        self.mounted_source = None;
    }

    pub(crate) fn view_revision(&self) -> u64 {
        self.presentation_revision
    }
    pub fn descriptor(&self) -> Option<&crate::engine_client::Descriptor> {
        self.project
            .as_ref()
            .map(|project| &project.context.descriptor)
    }
    #[cfg(test)]
    pub fn references(&self) -> Option<(&str, &str)> {
        self.project
            .as_ref()
            .map(|project| (project.refs.base.as_str(), project.refs.head.as_str()))
    }
    pub fn catalog_pending(&self) -> bool {
        self.catalog_job.is_pending()
    }
    pub fn detail_pending(&self) -> bool {
        self.detail_job.is_pending()
    }
    pub fn header(&self) -> Option<&crate::engine_client::flow_comparison::Header> {
        self.binding.as_ref().map(BoundComparison::header)
    }
    pub fn set_waker(&self, waker: Waker) {
        self.catalog_job.set_waker(waker.clone());
        self.detail_job.set_waker(waker.clone());
        self.snapshots.set_waker(waker.clone());
        self.impact.set_waker(waker.clone());
        self.changes.set_waker(waker.clone());
        self.workers.set_waker(waker);
    }
    pub fn pending(&self) -> bool {
        self.catalog_job.is_pending()
            || self.detail_job.is_pending()
            || self.snapshots.pending()
            || self.impact.pending()
            || self.changes.pending()
    }
    pub fn stopped(&self) -> bool {
        self.workers.stopped()
    }
    pub fn close(&mut self) {
        self.impact.close();
        self.controls.panel = Panel::Flows;
        self.snapshots.reset();
        self.close_flow_binding();
    }
    fn close_flow_binding(&mut self) {
        self.controls.visible = false;
        self.controls.catalog_open = false;
        self.cancel_requests();
        self.binding = None;
        if let Some(project) = self.project.take() {
            let retired = self.workers.track(project);
            std::thread::spawn(move || drop(retired));
        }
        self.catalog = None;
        self.current = None;
        self.history.clear();
        self.identity.reset();
        self.phase = Phase::Closed;
        self.detail_message.clear();
        self.pending_scroll = None;
        self.mounted_source = None;
    }
    fn cancel_requests(&mut self) {
        self.catalog_clock.cancel();
        self.detail_clock.cancel();
        self.catalog_job.cancel();
        self.detail_job.cancel();
        self.changes.clear();
    }
    fn open(&mut self, project: Project) {
        self.close_flow_binding();
        self.project = Some(project.clone());
        self.phase = Phase::Loading;
        self.controls.visible = true;
        self.controls.reset_binding();
        let cancel = self.catalog_clock.restart();
        let serial = self.catalog_clock.serial;
        let work = self.workers.track(requests::CatalogWork {
            serial,
            cancel,
            operation: requests::CatalogOperation::Open {
                project: Box::new(project),
                identity: self.identity.reporter(serial, self.workers.clone()),
            },
        });
        self.catalog_job
            .request(work, |work| work.map(requests::CatalogWork::run));
    }
    pub fn reload(&mut self) -> Result<(), String> {
        let refs = self
            .project
            .as_ref()
            .ok_or("No comparison project is open")?
            .refs
            .clone();
        self.load_refs(refs)
    }
    fn load_refs(&mut self, refs: ComparisonRefs) -> Result<(), String> {
        let mut project = self
            .project
            .clone()
            .ok_or("No comparison project is open")?;
        project.context = project.context_for(self.identity.captured());
        project.refs = refs;
        self.open(project);
        Ok(())
    }
    pub fn page(
        &mut self,
        page: crate::engine_client::flow_comparison::CatalogPage,
    ) -> Result<(), String> {
        let bound = self
            .binding
            .clone()
            .ok_or("Reload the comparison before browsing")?;
        let physical_project = self
            .identity
            .captured()
            .ok_or("Reload the comparison before browsing")?
            .to_owned();
        let cancel = self.catalog_clock.restart();
        let serial = self.catalog_clock.serial;
        let work = self.workers.track(requests::CatalogWork {
            serial,
            cancel,
            operation: requests::CatalogOperation::Page {
                bound: Box::new(bound),
                page,
                physical_project,
            },
        });
        self.catalog_job
            .request(work, |work| work.map(requests::CatalogWork::run));
        Ok(())
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        self.close();
    }
}
#[cfg(test)]
mod cleanup_tests;
#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod identity_tests;
#[cfg(test)]
mod publication_tests;
#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
pub(crate) mod tests;
