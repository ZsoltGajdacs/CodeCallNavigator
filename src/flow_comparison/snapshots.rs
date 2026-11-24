//! Snapshot jobs share comparison lifetime, but can run before a flow pair exists.
use super::{Panel, work};
use crate::{
    app::State,
    engine_client::{
        self, QueryContext,
        flow_comparison::{
            ComparisonRefs, GraphCursor, GraphPage, SnapshotPreparation, SnapshotStatus,
        },
    },
};
use codecallnavigator::{artifact::local_snapshots::SnapshotRecord, protocol::AnalysisFailureCode};
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
            Self::Base => "Base",
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
    pub panel_open: bool,
    pub message: String,
    pub(crate) preparation_failure: Option<engine_client::AnalysisFailure>,
    pub graph: Option<GraphPage>,
    pub selected_change: Option<usize>,
    lookups: [Option<Lookup>; 2],
    graph_refs: Option<ComparisonRefs>,
    previous: Vec<usize>,
    project: Option<PathBuf>,
    cancel: Arc<AtomicBool>,
    updates: Option<mpsc::Receiver<String>>,
    job: Latest<work::Tracked<anyhow::Result<Answer>>>,
}

enum Operation {
    Lookup {
        side: Side,
        revision: String,
    },
    Prepare {
        side: Side,
        selection: SnapshotPreparation,
    },
    Graph {
        refs: ComparisonRefs,
        cursor: GraphCursor,
        navigation: Navigation,
    },
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
    Graph {
        refs: ComparisonRefs,
        page: Box<GraphPage>,
        navigation: Navigation,
    },
}

#[derive(Clone, Copy)]
pub enum Navigation {
    First,
    Reload,
    Previous,
    Next,
}

impl State {
    pub fn lookup_comparison_snapshot(&mut self, side: Side) {
        self.flow_residency.activity();
        let revision = side
            .reference(&self.flow_comparison.control_refs())
            .to_owned();
        self.start_snapshot_operation(Operation::Lookup { side, revision });
    }

    pub fn prepare_comparison_snapshot(&mut self, side: Side) {
        self.flow_residency.activity();
        let Some(analysis) = self.selected_analysis() else {
            self.flow_comparison
                .snapshots
                .set_status(SELECT_BUILDS.into());
            return;
        };
        let revision = side
            .reference(&self.flow_comparison.control_refs())
            .to_owned();
        self.start_snapshot_operation(Operation::Prepare {
            side,
            selection: SnapshotPreparation { revision, analysis },
        });
    }

    pub fn compare_snapshot_graphs(&mut self, navigation: Navigation) {
        self.flow_residency.activity();
        let snapshot = &self.flow_comparison.snapshots;
        let (refs, cursor) = match navigation {
            Navigation::First => (self.flow_comparison.control_refs(), GraphCursor::default()),
            Navigation::Reload => {
                let Some(refs) = snapshot.graph_refs.clone() else {
                    return;
                };
                (refs, GraphCursor::default())
            }
            Navigation::Previous | Navigation::Next => {
                let Some(refs) = snapshot.graph_refs.clone() else {
                    return;
                };
                let Some(page) = &snapshot.graph else { return };
                let offset = match navigation {
                    Navigation::Previous => snapshot.previous.last().copied(),
                    Navigation::Next => page.next_offset,
                    Navigation::First | Navigation::Reload => unreachable!(),
                };
                let Some(offset) = offset else { return };
                (
                    refs,
                    GraphCursor {
                        offset,
                        binding: Some(page.header.comparison_id.clone()),
                    },
                )
            }
        };
        self.flow_comparison.controls.catalog_open = false;
        self.flow_comparison.previews.close();
        self.flow_comparison.controls.panel = Panel::Graph;
        self.flow_comparison.impact.clear();
        self.start_snapshot_operation(Operation::Graph {
            refs,
            cursor,
            navigation,
        });
    }

    fn start_snapshot_operation(&mut self, operation: Operation) {
        match self.library_comparison_context() {
            Ok(context) => self.flow_comparison.snapshots.start(
                context,
                operation,
                self.flow_comparison.workers.clone(),
            ),
            Err(error) => self.flow_comparison.snapshots.set_status(error),
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

    pub fn previous_available(&self) -> bool {
        !self.previous.is_empty()
    }

    pub fn can_reload_graph(&self) -> bool {
        self.graph_refs.is_some()
    }

    pub fn select_graph_change(&mut self, binding: &str, offset: usize, index: usize) {
        if self.graph.as_ref().is_some_and(|page| {
            page.header.comparison_id == binding
                && page.offset == offset
                && index < page.changes.len()
        }) {
            self.selected_change = Some(index);
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
        self.graph = None;
        self.selected_change = None;
        self.graph_refs = None;
        self.previous.clear();
        self.project = None;
        self.set_status(String::new());
        self.panel_open = false;
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

    fn start(&mut self, context: QueryContext, operation: Operation, workers: Arc<work::Workers>) {
        if self.pending() {
            return;
        }
        self.project_installed(Some(&context.descriptor.project));
        self.project = Some(context.descriptor.project.clone());
        self.cancel = Arc::new(AtomicBool::new(false));
        self.set_status(
            match &operation {
                Operation::Lookup { .. } => "Looking up local snapshots…",
                Operation::Prepare { .. } => "Preparing local snapshot…",
                Operation::Graph { .. } => "Comparing saved graphs…",
            }
            .into(),
        );
        if let Operation::Graph {
            navigation: Navigation::First | Navigation::Reload,
            refs,
            ..
        } = &operation
        {
            self.graph = None;
            self.selected_change = None;
            self.graph_refs = Some(refs.clone());
            self.previous.clear();
        }
        let (send, receive) = mpsc::channel();
        self.updates = Some(receive);
        let cancel = self.cancel.clone();
        let progress_workers = workers.clone();
        let task = workers.track(move || {
            execute(context, operation, &cancel, &|message| {
                if send.send(message).is_ok() {
                    progress_workers.wake();
                }
            })
        });
        self.job.request(task, |task| task.map(|run| run()));
    }

    pub(super) fn poll(&mut self) -> bool {
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
            return progress.is_some();
        };
        self.updates = None;
        output.map(|result| self.accept(result));
        true
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
            Ok(Answer::Graph {
                refs,
                page,
                navigation,
            }) => {
                match navigation {
                    Navigation::First | Navigation::Reload => self.previous.clear(),
                    Navigation::Previous => {
                        self.previous.pop();
                    }
                    Navigation::Next => {
                        if let Some(old) = &self.graph {
                            self.previous.push(old.offset);
                        }
                    }
                }
                self.graph = Some(*page);
                self.selected_change = None;
                self.graph_refs = Some(refs);
            }
            Err(error) => {
                if error
                    .downcast_ref::<engine_client::QueryFailure>()
                    .is_some_and(|error| error.0.code == "comparison_mismatch")
                {
                    self.graph = None;
                    self.selected_change = None;
                    self.previous.clear();
                }
                self.message = format!("{error:#}");
            }
        }
    }

    fn set_status(&mut self, message: String) {
        self.message = message;
        self.preparation_failure = None;
    }
}

fn execute(
    context: QueryContext,
    operation: Operation,
    cancel: &AtomicBool,
    progress: &dyn Fn(String),
) -> anyhow::Result<Answer> {
    match operation {
        Operation::Lookup { side, revision } => Ok(Answer::Lookup {
            status: context.snapshot_status(revision.clone(), cancel)?,
            side,
            reference: revision,
            prepared: None,
        }),
        Operation::Prepare { side, selection } => {
            let revision = selection.revision.clone();
            let record = match context.prepare_local_snapshot(selection, cancel, progress) {
                Ok(record) => record,
                Err(error) => return preparation_error(error),
            };
            let message = preparation_message(&record);
            match context.snapshot_status(revision.clone(), cancel) {
                Ok(status) if status.snapshots.contains(&record) => Ok(Answer::Lookup {
                    side,
                    reference: revision,
                    status,
                    prepared: Some(record),
                }),
                Ok(_) => Ok(Answer::PreparedWithoutLookup(format!(
                    "{message} Refresh the snapshot list to select a current variant."
                ))),
                Err(error) => Ok(Answer::PreparedWithoutLookup(format!(
                    "{message} Could not refresh the snapshot list: {error:#}"
                ))),
            }
        }
        Operation::Graph {
            refs,
            cursor,
            navigation,
        } => Ok(Answer::Graph {
            page: Box::new(context.comparison_graph_page(&refs, cursor, cancel)?),
            refs,
            navigation,
        }),
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
