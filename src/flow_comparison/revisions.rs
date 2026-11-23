//! Compare selection and history belong to the active checkout, independent of saved pairs.
use super::{SnapshotSide, Workspace, work};
use crate::{app::State, engine_client::flow_comparison::ComparisonRefs};
use codecallnavigator::artifact::revision::{CommitHistory, CommitPage, HistoryCommit};
use rune::{Latest, Waker};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
enum After {
    #[default]
    Worktree,
    Commit(String),
}

#[derive(Default)]
pub(crate) struct Revisions {
    base: Option<String>,
    after: After,
    pub(crate) picker: Option<SnapshotSide>,
    pub(crate) rows: Vec<HistoryCommit>,
    pub(crate) message: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) more: Option<usize>,
    project: Option<PathBuf>,
    history: Option<Arc<CommitHistory>>,
    page: Latest<work::Tracked<anyhow::Result<Page>>>,
    detail: Latest<work::Tracked<anyhow::Result<String>>>,
    cancel_page: Arc<AtomicBool>,
    cancel_detail: Arc<AtomicBool>,
}

struct Page {
    history: Arc<CommitHistory>,
    page: CommitPage,
}

impl Revisions {
    pub(super) fn physical_project(&self) -> Option<&std::path::Path> {
        self.history.as_ref().map(|history| history.project())
    }

    pub(crate) fn base(&self) -> Option<&str> {
        self.base.as_deref()
    }

    pub(crate) fn after(&self) -> &str {
        match &self.after {
            After::Worktree => "WORKTREE",
            After::Commit(id) => id,
        }
    }

    pub(crate) fn worktree(&self) -> bool {
        matches!(self.after, After::Worktree)
    }

    pub(crate) fn loading(&self) -> bool {
        self.page.is_pending()
    }

    pub(crate) fn reading_message(&self) -> bool {
        self.detail.is_pending()
    }

    pub(super) fn set_waker(&self, waker: Waker) {
        self.page.set_waker(waker.clone());
        self.detail.set_waker(waker);
    }

    fn select(&mut self, side: SnapshotSide, id: &str) {
        match side {
            SnapshotSide::Base => self.base = Some(id.to_owned()),
            SnapshotSide::Head if id == "WORKTREE" => self.after = After::Worktree,
            SnapshotSide::Head => self.after = After::Commit(id.to_owned()),
        }
    }

    #[cfg(test)]
    pub(crate) fn set_for_test(&mut self, side: SnapshotSide, id: &str) {
        self.select(side, id);
    }

    pub(super) fn reset(&mut self) {
        self.cancel_page.store(true, Ordering::Release);
        self.page.cancel();
        self.close_picker();
        self.project = None;
        self.history = None;
        self.rows.clear();
        self.error = None;
        self.more = None;
        self.base = None;
        self.after = After::Worktree;
    }

    pub(crate) fn close_picker(&mut self) {
        self.cancel_detail.store(true, Ordering::Release);
        self.detail.cancel();
        self.picker = None;
        self.message = None;
    }

    fn all_commits_loaded(&self) -> bool {
        self.history.is_some() && self.more.is_none()
    }

    fn request(&mut self, workers: &Arc<work::Workers>) {
        if self.loading() || self.all_commits_loaded() {
            return;
        }
        let Some(project) = self.project.clone() else {
            self.error = Some("Choose a Git project to browse commits.".into());
            return;
        };
        self.error = None;
        self.cancel_page = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel_page.clone();
        let history = self.history.clone();
        let offset = self.more.unwrap_or_default();
        self.page
            .request(workers.track((project, history)), move |request| {
                request.map(|(project, history)| {
                    let history = match history {
                        Some(history) => history,
                        None => Arc::new(CommitHistory::open(&project, &cancel)?),
                    };
                    let page = history.page(offset, &cancel)?;
                    Ok(Page { history, page })
                })
            });
    }

    fn request_message(&mut self, id: &str, workers: &Arc<work::Workers>) {
        self.cancel_detail.store(true, Ordering::Release);
        self.detail.cancel();
        self.message = None;
        let Some(history) = self.history.clone().filter(|_| id != "WORKTREE") else {
            return;
        };
        self.cancel_detail = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel_detail.clone();
        self.detail
            .request(workers.track(id.to_owned()), move |id| {
                id.map(|id| history.message(&id, &cancel))
            });
    }

    pub(super) fn poll(&mut self) -> bool {
        let page = self.page.take().map(|result| {
            result.map(|result| match result {
                Ok(Page { history, page }) => {
                    if self.history.is_none() && self.base.is_none() {
                        self.base = history.head().map(str::to_owned);
                    }
                    self.history = Some(history);
                    self.rows.extend(page.rows);
                    self.more = page.next_offset;
                }
                Err(error) => self.error = Some(format!("{error:#}")),
            })
        });
        let detail = self.detail.take().map(|result| {
            result.map(|result| {
                self.message =
                    Some(result.unwrap_or_else(|error| {
                        format!("Could not read commit message: {error:#}")
                    }));
            })
        });
        page.is_some() || detail.is_some()
    }
}

impl State {
    pub(super) fn start_comparison_history(&mut self) {
        let path = self
            .query_context
            .as_ref()
            .map(|context| context.descriptor.project.clone())
            .or_else(|| self.project_open.folder.as_deref().map(PathBuf::from));
        let workspace = &mut self.flow_comparison;
        if workspace.revisions.project != path {
            workspace.revisions.reset();
            workspace.revisions.project = path;
        }
        workspace.more_comparison_commits();
    }
}

impl Workspace {
    pub fn control_refs(&self) -> ComparisonRefs {
        self.snapshots.references(
            self.revisions.base().unwrap_or_default().to_owned(),
            self.revisions.after().to_owned(),
        )
    }

    pub(crate) fn more_comparison_commits(&mut self) {
        self.revisions.request(&self.workers);
    }

    pub(crate) fn show_revision_picker(&mut self, side: SnapshotSide) {
        self.controls.catalog_open = false;
        self.changes.preview = None;
        self.revisions.picker = Some(side);
        let id = match side {
            SnapshotSide::Base => self.revisions.base(),
            SnapshotSide::Head => Some(self.revisions.after()),
        }
        .map(str::to_owned);
        if let Some(id) = id {
            self.revisions.request_message(&id, &self.workers);
        }
    }

    pub(crate) fn choose_snapshot_variant(&mut self, side: SnapshotSide, id: Option<String>) {
        let previous = self.control_refs();
        self.snapshots.choose(side, id);
        if self.control_refs() != previous {
            self.invalidate_worktree_refresh();
        }
    }

    pub(crate) fn choose_revision(&mut self, side: SnapshotSide, id: &str) {
        if self.revisions.picker != Some(side) {
            return;
        }
        let before = self.control_refs();
        self.revisions.select(side, id);
        if self.control_refs() != before {
            self.invalidate_worktree_refresh();
        }
        self.revisions.request_message(id, &self.workers);
    }
}
