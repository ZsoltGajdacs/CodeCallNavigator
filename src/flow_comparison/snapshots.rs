//! Snapshot jobs share comparison lifetime, but can run before a flow pair exists.
use super::work;
use crate::{
    app::State,
    engine_client::{
        self, QueryContext,
        flow_comparison::{ComparisonRefs, SnapshotPreparation, SnapshotStatus},
    },
};
use codecallnavigator::{
    artifact::{local_snapshots::SnapshotRecord, verification::InputStatus},
    protocol::AnalysisFailureCode,
    query::Query,
};
use rune::{Latest, Waker};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
};

#[cfg(test)]
mod tests;

pub(crate) const SELECT_BUILDS: &str = "Select builds in the main graph to prepare a snapshot.";
const PREPARATION_FAILED: &str = "Snapshot preparation failed";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Base,
    Head,
}

impl Side {
    pub fn label(self) -> &'static str {
        match self {
            Self::Base => "Before",
            Self::Head => "After",
        }
    }

    pub fn reference(self, refs: &ComparisonRefs) -> &str {
        match self {
            Self::Base => &refs.base,
            Self::Head => &refs.head,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Base => 0,
            Self::Head => 1,
        }
    }
}

pub struct Lookup {
    reference: String,
    pub status: SnapshotStatus,
    pub selected: Option<String>,
}

#[derive(Default)]
pub struct Snapshots {
    pub message: String,
    pub side: Option<Side>,
    pub worktree: Option<WorktreeSnapshot>,
    pub variant_picker: Option<Side>,
    pub(crate) preparation_failure: Option<engine_client::AnalysisFailure>,
    lookups: [Option<Lookup>; 2],
    project: Option<PathBuf>,
    cancel: Arc<AtomicBool>,
    updates: Option<mpsc::Receiver<String>>,
    job: Latest<work::Tracked<anyhow::Result<Answer>>>,
}

enum Operation {
    Lookup(String),
    Prepare(SnapshotPreparation),
    Worktree,
}

struct SnapshotRequest {
    side: Side,
    operation: Operation,
}

#[derive(Clone, Copy)]
pub(crate) enum WorktreeSnapshot {
    Missing,
    Saved(Option<InputStatus>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Poll {
    Idle,
    Updated,
    Rebind,
}

enum Answer {
    Lookup {
        side: Side,
        reference: String,
        status: SnapshotStatus,
        prepared: Option<SnapshotRecord>,
    },
    PreparedWithoutLookup(String),
    PreparationFailed(engine_client::AnalysisFailure),
    Worktree(anyhow::Result<WorktreeSnapshot>),
}

impl State {
    pub fn lookup_comparison_snapshot(&mut self, side: Side) {
        self.flow_residency.activity();
        let revision = side
            .reference(&self.flow_comparison.control_refs())
            .to_owned();
        if revision == "WORKTREE" {
            self.flow_comparison.worktree_refresh.message.clear();
            self.start_snapshot_operation(side, Operation::Worktree);
        } else {
            self.start_snapshot_operation(side, Operation::Lookup(revision));
        }
    }

    pub fn prepare_comparison_snapshot(&mut self, side: Side) {
        self.flow_residency.activity();
        let revision = side
            .reference(&self.flow_comparison.control_refs())
            .to_owned();
        if revision == "WORKTREE" {
            self.refresh_comparison_worktree();
            return;
        }
        let Some(analysis) = self.selected_analysis() else {
            self.flow_comparison.snapshots.side = Some(side);
            self.flow_comparison
                .snapshots
                .set_status(SELECT_BUILDS.into());
            return;
        };
        self.start_snapshot_operation(
            side,
            Operation::Prepare(SnapshotPreparation { revision, analysis }),
        );
    }

    fn start_snapshot_operation(&mut self, side: Side, operation: Operation) {
        if self.flow_comparison.snapshots.pending()
            || self.flow_comparison.worktree_refresh.pending()
        {
            return;
        }
        match self.library_comparison_context() {
            Ok(context) => self.flow_comparison.snapshots.start(
                context,
                SnapshotRequest { side, operation },
                self.flow_comparison.workers.clone(),
            ),
            Err(error) => {
                self.flow_comparison.snapshots.side = Some(side);
                self.flow_comparison.snapshots.set_status(error);
            }
        }
    }
}

impl Snapshots {
    pub fn lookup(&self, side: Side, reference: &str) -> Option<&Lookup> {
        self.lookups[side.index()]
            .as_ref()
            .filter(|lookup| lookup.reference == reference)
    }

    pub fn choose(&mut self, side: Side, id: Option<String>) {
        if let Some(lookup) = &mut self.lookups[side.index()] {
            lookup.selected = id;
        }
    }

    pub fn pending(&self) -> bool {
        self.job.is_pending()
    }

    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if self.pending() {
            self.message = "Cancelling snapshot request…".into();
        }
    }

    pub(super) fn set_waker(&self, waker: Waker) {
        self.job.set_waker(waker);
    }

    pub(super) fn reset(&mut self) {
        self.cancel();
        self.job.cancel();
        self.updates = None;
        self.lookups = [None, None];
        self.worktree = None;
        self.side = None;
        self.variant_picker = None;
        self.project = None;
        self.set_status(String::new());
    }

    pub(super) fn project_installed(&mut self, project: Option<&Path>) {
        if self
            .project
            .as_deref()
            .is_some_and(|captured| Some(captured) != project)
        {
            self.reset();
        }
    }

    pub(super) fn references(&self, base: String, head: String) -> ComparisonRefs {
        let base_snapshot = self
            .lookup(Side::Base, &base)
            .and_then(|lookup| lookup.selected.clone());
        let head_snapshot = self
            .lookup(Side::Head, &head)
            .and_then(|lookup| lookup.selected.clone());
        ComparisonRefs {
            base,
            head,
            base_snapshot,
            head_snapshot,
        }
    }

    fn start(
        &mut self,
        context: QueryContext,
        request: SnapshotRequest,
        workers: Arc<work::Workers>,
    ) {
        self.project_installed(Some(&context.descriptor.project));
        self.project = Some(context.descriptor.project.clone());
        self.side = Some(request.side);
        self.variant_picker = None;
        self.cancel = Arc::new(AtomicBool::new(false));
        self.set_status(
            match &request.operation {
                Operation::Lookup(_) => "Looking up local snapshots…",
                Operation::Prepare(_) => "Preparing local snapshot…",
                Operation::Worktree => "Finding Worktree snapshot…",
            }
            .into(),
        );
        let (send, receive) = mpsc::channel();
        self.updates = Some(receive);
        let cancel = self.cancel.clone();
        let progress_workers = workers.clone();
        let task = workers.track(move || {
            execute(context, request, &cancel, &|message| {
                if send.send(message).is_ok() {
                    progress_workers.wake();
                }
            })
        });
        self.job.request(task, |task| task.map(|run| run()));
    }

    pub(super) fn discard_pending(&mut self) {
        self.cancel();
        self.job.cancel();
        self.updates = None;
        self.side = None;
        self.variant_picker = None;
        self.set_status(String::new());
        self.worktree = None;
    }

    pub(super) fn poll(&mut self) -> Poll {
        let progress = self
            .updates
            .as_ref()
            .and_then(|updates| updates.try_iter().last());
        if let Some(message) = progress
            .as_ref()
            .filter(|_| !self.cancel.load(Ordering::Acquire))
        {
            self.message.clone_from(message);
        }
        let Some(output) = self.job.take() else {
            return if progress.is_some() {
                Poll::Updated
            } else {
                Poll::Idle
            };
        };
        self.updates = None;
        if self.cancel.load(Ordering::Acquire) {
            self.set_status("Snapshot request cancelled".into());
            return Poll::Updated;
        }
        let reopen = output
            .map(|result| {
                let reopen = matches!(&result, Ok(Answer::Worktree(_) | Answer::Lookup { .. }));
                self.accept(result);
                reopen
            })
            .value;
        if reopen { Poll::Rebind } else { Poll::Updated }
    }

    fn accept(&mut self, answer: anyhow::Result<Answer>) {
        self.set_status(String::new());
        match answer {
            Ok(Answer::Lookup {
                side,
                reference,
                status,
                prepared,
            }) => {
                let old = self
                    .lookup(side, &reference)
                    .and_then(|lookup| lookup.selected.clone());
                let selected = prepared
                    .as_ref()
                    .map(|record| {
                        self.message = preparation_message(record);
                        record.id.clone()
                    })
                    .or(old)
                    .filter(|id| status.snapshots.iter().any(|record| &record.id == id));
                if prepared.is_none() && !status.snapshots.is_empty() {
                    self.variant_picker = Some(side);
                }
                self.lookups[side.index()] = Some(Lookup {
                    reference,
                    status,
                    selected,
                });
            }
            Ok(Answer::PreparedWithoutLookup(message)) => self.message = message,
            Ok(Answer::PreparationFailed(failure)) => {
                self.message = PREPARATION_FAILED.into();
                self.preparation_failure = Some(failure);
            }
            Ok(Answer::Worktree(Ok(status))) => self.worktree = Some(status),
            Ok(Answer::Worktree(Err(error))) => {
                self.worktree = None;
                self.message = format!("{error:#}");
            }
            Err(error) => self.message = format!("{error:#}"),
        }
    }

    fn set_status(&mut self, message: String) {
        self.message = message;
        self.preparation_failure = None;
    }
}

fn execute(
    context: QueryContext,
    request: SnapshotRequest,
    cancel: &AtomicBool,
    progress: &dyn Fn(String),
) -> anyhow::Result<Answer> {
    let SnapshotRequest { side, operation } = request;
    match operation {
        Operation::Lookup(revision) => Ok(Answer::Lookup {
            status: context.snapshot_status(revision.clone(), cancel)?,
            side,
            reference: revision,
            prepared: None,
        }),
        Operation::Prepare(selection) => {
            let prepared = prepare(&context, selection, cancel, progress);
            finish_preparation(side, prepared)
        }
        Operation::Worktree => Ok(Answer::Worktree(find_worktree(&context, cancel))),
    }
}

fn prepare(
    context: &QueryContext,
    selection: SnapshotPreparation,
    cancel: &AtomicBool,
    progress: &dyn Fn(String),
) -> anyhow::Result<PreparedCommit> {
    let revision = selection.revision.clone();
    let record = context.prepare_local_snapshot(selection, cancel, progress)?;
    let status = context.snapshot_status(revision.clone(), cancel);
    Ok(PreparedCommit {
        revision,
        record,
        status,
    })
}

struct PreparedCommit {
    revision: String,
    record: SnapshotRecord,
    status: anyhow::Result<SnapshotStatus>,
}

fn finish_preparation(
    side: Side,
    prepared: anyhow::Result<PreparedCommit>,
) -> anyhow::Result<Answer> {
    let PreparedCommit {
        revision,
        record,
        status,
    } = match prepared {
        Ok(prepared) => prepared,
        Err(error) => return preparation_error(error),
    };
    let message = preparation_message(&record);
    match status {
        Ok(status) if status.snapshots.contains(&record) => Ok(Answer::Lookup {
            side,
            reference: revision,
            status,
            prepared: Some(record),
        }),
        Ok(_) => Ok(Answer::PreparedWithoutLookup(format!(
            "{message} Use Find snapshot to select a current variant."
        ))),
        Err(error) => Ok(Answer::PreparedWithoutLookup(format!(
            "{message} Could not refresh the snapshot list: {error:#}"
        ))),
    }
}

fn find_worktree(context: &QueryContext, cancel: &AtomicBool) -> anyhow::Result<WorktreeSnapshot> {
    match context.response(Query::Status, cancel) {
        Ok(response) => Ok(WorktreeSnapshot::Saved(
            crate::engine_client::flow_comparison::worktree_freshness(response.freshness),
        )),
        Err(error)
            if error
                .downcast_ref::<engine_client::QueryFailure>()
                .is_some_and(|failure| failure.0.code == "graph_missing") =>
        {
            Ok(WorktreeSnapshot::Missing)
        }
        Err(error) => Err(error),
    }
}

fn preparation_message(record: &SnapshotRecord) -> String {
    format!("Prepared snapshot for {}.", record.identity.commit)
}

fn preparation_error(error: anyhow::Error) -> anyhow::Result<Answer> {
    let code = error
        .downcast_ref::<engine_client::AnalysisFailure>()
        .and_then(|failure| failure.code);
    match code {
        Some(AnalysisFailureCode::Failed) => error.downcast().map(Answer::PreparationFailed),
        Some(AnalysisFailureCode::Cancelled) => {
            Err(error.context("Snapshot preparation cancelled"))
        }
        _ => Err(error.context(PREPARATION_FAILED)),
    }
}
