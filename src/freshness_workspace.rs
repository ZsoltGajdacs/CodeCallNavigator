//! UI observation lifetime bound to the loaded CLI descriptor, never the folder editor.
mod worker;
use crate::{
    app::State,
    engine_client::{Descriptor, QueryContext},
};
use codecallnavigator::{artifact::verification::InputStatus, freshness::FreshnessObservation};
use std::sync::atomic::Ordering;
use worker::Worker;

#[derive(Default)]
pub enum Status {
    #[default]
    Idle,
    Checking,
    Observed(InputStatus),
    Unsupported,
    Failed(String),
}

#[derive(Default)]
pub struct Workspace {
    pub status: Status,
    pub last: Option<FreshnessObservation>,
    pub automatic: crate::automatic_refresh::Authorization,
    pub cleanup_warning: Option<String>,
    automatic_worker: bool,
    manual_pending: Option<PendingManual>,
    manual_running: bool,
    binding: Option<Descriptor>,
    worker: Option<Worker>,
    retired: Vec<Worker>,
}

struct PendingManual {
    selection: crate::engine_client::Analysis,
    binding: crate::workspace::Binding,
}

#[derive(Default)]
struct Pending {
    observation: Option<FreshnessObservation>,
    terminal: Option<Status>,
    activity: Option<codecallnavigator::refresh::Activity>,
    cleanup_warning: Option<String>,
}

impl Pending {
    fn has_update(&self) -> bool {
        self.cleanup_warning.is_some()
            || self.observation.is_some()
            || self.terminal.is_some()
            || self.activity.is_some()
    }
}

impl Workspace {
    fn start(&mut self, context: QueryContext, wake: Option<rune::Waker>) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            self.retired.push(worker);
        }
        self.retired_cleanup();
        if self.binding.as_ref() != Some(&context.descriptor) {
            self.last = None;
        }
        self.automatic.reconcile(Some(&context.descriptor));
        self.automatic_worker = self.automatic.enabled(&context.descriptor.project);
        self.automatic.activity = None;
        self.binding = Some(context.descriptor.clone());
        self.status = Status::Checking;
        let previous = self
            .retired
            .iter()
            .map(|worker| worker.finished.clone())
            .collect();
        self.worker = Some(Worker::start(
            context,
            wake,
            self.automatic_worker,
            previous,
        ));
    }

    fn sync(&mut self, context: Option<QueryContext>, wake: Option<rune::Waker>) {
        self.automatic
            .reconcile(context.as_ref().map(|context| &context.descriptor));
        if let Some(context) = context.filter(|context| {
            self.binding.as_ref().is_none_or(|bound| {
                bound.project != context.descriptor.project
                    || bound.roots != context.descriptor.roots
            }) || self.automatic_worker != self.automatic.enabled(&context.descriptor.project)
        }) {
            self.start(context, wake);
        }
    }

    pub fn installed(&mut self, descriptor: Option<&Descriptor>) {
        self.automatic.reconcile(descriptor);
        if descriptor.is_none_or(|next| {
            self.binding
                .as_ref()
                .is_some_and(|old| old.project != next.project || old.roots != next.roots)
        }) {
            self.stop();
        }
    }

    pub fn stop(&mut self) {
        if let Some(worker) = &self.worker {
            worker.cancel.store(true, Ordering::Release);
        }
    }
    pub fn cancel_pending_manual(&mut self) -> bool {
        let cancelled = self.manual_pending.take().is_some();
        if cancelled {
            self.binding = None;
        }
        cancelled
    }

    fn automatic_cleanup_pending(&self) -> bool {
        self.worker
            .iter()
            .chain(self.retired.iter())
            .any(|worker| worker.automatic && !worker.finished.load(Ordering::Acquire))
    }

    pub fn manual_pending(&self) -> bool {
        self.manual_pending.is_some()
    }

    pub fn stopped(&self) -> bool {
        self.worker
            .as_ref()
            .is_none_or(|worker| worker.finished.load(Ordering::Acquire))
            && self
                .retired
                .iter()
                .all(|worker| worker.finished.load(Ordering::Acquire))
    }

    fn take(&mut self) -> bool {
        let cleanup_changed = self.retired_cleanup();
        let Some(worker) = &self.worker else {
            return cleanup_changed;
        };
        let pending = worker
            .pending
            .lock()
            .map(|mut value| std::mem::take(&mut *value))
            .unwrap_or_else(|_| Pending {
                terminal: Some(Status::Failed("Freshness worker failed".into())),
                ..Default::default()
            });
        let changed = cleanup_changed || pending.has_update();
        if let Some(warning) = pending.cleanup_warning {
            self.cleanup_warning = Some(warning);
        }
        if let Some(observation) = pending.observation {
            self.accept(observation);
        }
        if let Some(activity) = pending.activity {
            self.automatic.activity = Some(activity);
        }
        if let Some(terminal) = pending.terminal {
            self.status = terminal;
        }
        changed
    }

    fn retired_cleanup(&mut self) -> bool {
        let mut changed = false;
        let latest = &mut self.cleanup_warning;
        self.retired.retain(|worker| {
            // Acquire completion before draining: a worker completing afterward
            // stays retained for the next poll, so its diagnostic cannot be lost.
            let finished = worker.finished.load(Ordering::Acquire);
            if let Ok(mut pending) = worker.pending.lock()
                && let Some(warning) = pending.cleanup_warning.take()
            {
                *latest = Some(warning);
                changed = true;
            }
            !finished
        });
        changed
    }

    pub(crate) fn accept(&mut self, observation: FreshnessObservation) {
        self.status = Status::Observed(observation.input_status);
        self.last = Some(observation);
    }

    #[cfg(test)]
    pub fn is_stale(&self) -> bool {
        self.last
            .as_ref()
            .is_some_and(|last| last.input_status == InputStatus::Stale)
    }
    pub fn retry_available(&self) -> bool {
        matches!(self.status, Status::Failed(_) | Status::Unsupported)
            || self.automatic.retry_available()
    }

    pub fn new_generation(&self, loaded: Option<&str>) -> Option<&str> {
        let loaded = loaded?;
        self.last
            .as_ref()
            .filter(|last| last.contract_valid && last.generation != loaded)
            .map(|last| last.generation.as_str())
    }

    pub fn message(&self, loaded: Option<&str>) -> Option<String> {
        match &self.status {
            Status::Idle => None,
            Status::Checking => Some("Checking saved-input freshness…".into()),
            Status::Unsupported => {
                Some("Continuous freshness unavailable · update the CLI or refresh manually".into())
            }
            Status::Failed(error) => Some(format!("Freshness check failed · {error}")),
            Status::Observed(_) if self.new_generation(loaded).is_some() => {
                Some("A newer saved graph is available".into())
            }
            Status::Observed(InputStatus::Match) => {
                Some("Recorded inputs match · observed scope only".into())
            }
            Status::Observed(InputStatus::Stale) => {
                Some("Source files changed · refresh to update this graph".into())
            }
            Status::Observed(InputStatus::Unverifiable) => Some(format!(
                "Saved inputs are unverifiable · {}",
                self.reason_context()
            )),
        }
    }

    pub fn attention(&self, loaded: Option<&str>) -> Option<String> {
        if matches!(self.status, Status::Observed(InputStatus::Match))
            && self.new_generation(loaded).is_none()
        {
            return None;
        }
        self.message(loaded)
    }

    fn reason_context(&self) -> String {
        self.last
            .as_ref()
            .and_then(|last| last.reasons.first())
            .map(|reason| match &reason.path {
                Some(path) => format!("{}: {path}", reason.code),
                None => reason.code.clone(),
            })
            .unwrap_or_else(|| "no complete input manifest".into())
    }
}

impl State {
    pub(crate) fn defer_manual_analysis(
        &mut self,
        selection: &crate::engine_client::Analysis,
    ) -> bool {
        if self.freshness.manual_running {
            return false;
        }
        let enabled = self.query_context.as_ref().is_some_and(|context| {
            self.freshness
                .automatic
                .enabled(&context.descriptor.project)
        });
        if !enabled && !self.freshness.automatic_cleanup_pending() {
            return false;
        }
        self.freshness.manual_pending = Some(PendingManual {
            selection: selection.clone(),
            binding: crate::workspace::Binding::capture(self),
        });
        self.freshness.stop();
        self.status = "Stopping automatic refresh before manual analysis…".into();
        true
    }

    pub fn toggle_automatic_refresh(&mut self) {
        let Some(context) = self.query_context.clone() else {
            return;
        };
        self.freshness.automatic.toggle(&context.descriptor);
        if self.analyzing {
            self.freshness.manual_running = true;
        }
        if self.freshness.manual_pending() || self.freshness.manual_running {
            self.freshness.stop();
            return;
        }
        self.freshness.start(context, self.jobs.waker.clone());
    }

    pub fn cancel_automatic_refresh(&mut self) {
        if let Some(worker) = &self.freshness.worker {
            let _ = worker
                .controls
                .try_send(codecallnavigator::refresh::Control::CancelRefresh {
                    version: codecallnavigator::refresh::VERSION,
                });
        }
    }

    pub fn retry_freshness(&mut self) {
        if self.freshness.manual_pending() || self.freshness.manual_running {
            return;
        }
        if let Some(context) = self.query_context.clone() {
            self.freshness.start(context, self.jobs.waker.clone());
        }
    }
}

pub fn poll(state: &mut State) -> bool {
    if state.closing {
        state.freshness.stop();
        return state.freshness.take();
    }
    if let Some(changed) = poll_manual(state) {
        return changed;
    }
    if !state.uses_fixture_queries() {
        state
            .freshness
            .sync(state.query_context.clone(), state.jobs.waker.clone());
    }
    let changed = state.freshness.take();
    automatic_adoption(state) || changed
}

fn poll_manual(state: &mut State) -> Option<bool> {
    if state.freshness.manual_pending.is_none() {
        return resume_after_manual(state);
    }
    if !state.freshness.stopped() {
        return Some(state.freshness.take());
    }
    let request = state
        .freshness
        .manual_pending
        .take()
        .expect("pending manual analysis");
    if request.binding.matches(state) {
        state.freshness.manual_running = true;
        state.start_selected_analysis(request.selection);
    } else {
        state.freshness.binding = None;
    }
    Some(true)
}

fn resume_after_manual(state: &mut State) -> Option<bool> {
    if !state.freshness.manual_running {
        return None;
    }
    if state.analyzing {
        return Some(state.freshness.take());
    }
    state.freshness.manual_running = false;
    state.freshness.binding = None;
    None
}

fn automatic_adoption(state: &mut State) -> bool {
    if state.project_job_busy() {
        return false;
    }
    let Some(context) = state.query_context.as_ref() else {
        return false;
    };
    let Some(generation) = state
        .freshness
        .new_generation(state.generation.as_deref())
        .map(str::to_owned)
    else {
        return false;
    };
    if !state
        .freshness
        .automatic
        .adoption_due(&context.descriptor, &generation)
    {
        return false;
    }
    state.reload_generation();
    true
}

#[cfg(test)]
pub(crate) fn fixture(status: InputStatus, generation: &str) -> FreshnessObservation {
    FreshnessObservation {
        version: 1,
        generation: generation.into(),
        input_revision: Some("0".repeat(64)),
        contract_valid: true,
        input_status: status,
        reasons: Vec::new(),
        reasons_total: 0,
        omitted_reasons: 0,
        coverage: codecallnavigator::freshness::Coverage {
            kind: codecallnavigator::freshness::CoverageKind::ObservedInputs,
            complete: true,
            certifies_build: false,
        },
        mode: Default::default(),
        verification_milliseconds: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_client::Analysis;

    fn opted_state() -> State {
        let mut state = State::fixture();
        let descriptor = Descriptor {
            project: "/loaded".into(),
            ..Default::default()
        };
        let runtime = crate::runtime::Runtime {
            engine: "/unused".into(),
            resources: "/".into(),
            dot: "/unused".into(),
        };
        state.query_context = Some(QueryContext::new(&runtime, descriptor.clone()));
        state.freshness.automatic.toggle(&descriptor);
        state
    }

    #[test]
    fn deferred_manual_request_is_frozen_and_folder_edit_cancels_it() {
        let mut state = opted_state();
        let selected = Analysis {
            project: "/requested".into(),
            targets: vec![],
        };
        assert!(state.defer_manual_analysis(&selected));
        state.root = rune::TextEdit::new("/another-pending-folder");
        assert_eq!(
            state
                .freshness
                .manual_pending
                .as_ref()
                .unwrap()
                .selection
                .project,
            selected.project
        );
        state.root_edited();
        assert!(!state.freshness.manual_pending());
        assert!(!state.analyzing);
        assert!(
            state
                .freshness
                .automatic
                .enabled(std::path::Path::new("/loaded"))
        );
    }

    #[test]
    fn opting_out_while_manual_is_queued_does_not_start_a_blocking_monitor() {
        let mut state = opted_state();
        assert!(state.defer_manual_analysis(&Analysis {
            project: "/requested".into(),
            targets: vec![]
        }));
        state.toggle_automatic_refresh();
        state.retry_freshness();
        assert!(state.freshness.manual_pending());
        assert!(state.freshness.worker.is_none());
        assert!(state.freshness.stopped());
        assert!(
            !state
                .freshness
                .automatic
                .enabled(std::path::Path::new("/loaded"))
        );
    }

    #[test]
    fn manual_waits_for_owned_auto_cleanup_after_opting_out() {
        use std::sync::{Arc, Mutex, atomic::AtomicBool, mpsc};
        let mut state = State::fixture();
        let (controls, _commands) = mpsc::sync_channel(1);
        state.freshness.retired.push(Worker {
            automatic: true,
            cancel: Arc::new(AtomicBool::new(true)),
            finished: Arc::new(AtomicBool::new(false)),
            pending: Arc::new(Mutex::new(Pending::default())),
            controls,
        });
        assert!(state.defer_manual_analysis(&Analysis {
            project: "/requested".into(),
            targets: vec![]
        }));
        assert!(state.freshness.manual_pending());
        assert!(!state.analyzing);
    }

    #[test]
    fn retired_cancelled_monitor_keeps_its_cleanup_uncertainty_visible() {
        use std::sync::{Arc, Mutex, atomic::AtomicBool, mpsc};
        let mut workspace = Workspace::default();
        let (controls, _commands) = mpsc::sync_channel(1);
        workspace.retired.push(Worker {
            automatic: true,
            cancel: Arc::new(AtomicBool::new(true)),
            finished: Arc::new(AtomicBool::new(true)),
            pending: Arc::new(Mutex::new(Pending {
                cleanup_warning: Some("cleanup inventory unavailable".into()),
                ..Default::default()
            })),
            controls,
        });
        assert!(workspace.take());
        assert!(workspace.retired.is_empty());
        assert_eq!(
            workspace.cleanup_warning.as_deref(),
            Some("cleanup inventory unavailable")
        );
    }

    #[test]
    fn obsolete_deferred_manual_request_cannot_start_after_workspace_cutover() {
        let mut state = opted_state();
        assert!(state.defer_manual_analysis(&Analysis {
            project: "/requested".into(),
            targets: vec![]
        }));
        state.epoch += 1;
        assert_eq!(poll_manual(&mut state), Some(true));
        assert!(!state.analyzing);
        assert!(!state.freshness.manual_pending());
    }
}
