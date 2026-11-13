//! UI observation lifetime bound to the loaded CLI descriptor, never the folder editor.
use crate::{
    app::State,
    engine_client::{Descriptor, QueryContext},
};
use codecallnavigator::{artifact::verification::InputStatus, freshness::FreshnessObservation};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

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
    binding: Option<Descriptor>,
    worker: Option<Worker>,
    retired: Vec<Worker>,
}

#[derive(Default)]
struct Pending {
    observation: Option<FreshnessObservation>,
    terminal: Option<Status>,
}
struct Worker {
    cancel: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    pending: Arc<Mutex<Pending>>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

impl Worker {
    fn start(context: QueryContext, wake: Option<rune::Waker>) -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let pending = Arc::new(Mutex::new(Pending::default()));
        let worker = Self {
            cancel: cancel.clone(),
            finished: finished.clone(),
            pending: pending.clone(),
        };
        std::thread::spawn(move || {
            let observed = |observation| {
                if let Ok(mut pending) = pending.lock() {
                    pending.observation = Some(observation);
                }
                if let Some(wake) = &wake {
                    wake();
                }
            };
            let result = context.observe(&cancel, &observed);
            if !cancel.load(Ordering::Acquire) {
                let terminal = match result {
                    Ok(false) => Some(Status::Unsupported),
                    Err(error) => Some(Status::Failed(format!("{error:#}"))),
                    Ok(true) => None,
                };
                if let Ok(mut pending) = pending.lock() {
                    pending.terminal = terminal;
                }
            }
            finished.store(true, Ordering::Release);
            if let Some(wake) = &wake {
                wake();
            }
        });
        worker
    }
}

impl Workspace {
    fn start(&mut self, context: QueryContext, wake: Option<rune::Waker>) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            self.retired.push(worker);
        }
        self.retired
            .retain(|worker| !worker.finished.load(Ordering::Acquire));
        if self.binding.as_ref() != Some(&context.descriptor) {
            self.last = None;
        }
        self.binding = Some(context.descriptor.clone());
        self.status = Status::Checking;
        self.worker = Some(Worker::start(context, wake));
    }

    fn sync(&mut self, context: Option<QueryContext>, wake: Option<rune::Waker>) {
        if let Some(context) =
            context.filter(|context| self.binding.as_ref() != Some(&context.descriptor))
        {
            self.start(context, wake);
        }
    }

    pub fn stop(&mut self) {
        if let Some(worker) = &self.worker {
            worker.cancel.store(true, Ordering::Release);
        }
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
        let Some(worker) = &self.worker else {
            return false;
        };
        let pending = worker
            .pending
            .lock()
            .map(|mut value| std::mem::take(&mut *value))
            .unwrap_or_else(|_| Pending {
                terminal: Some(Status::Failed("Freshness worker failed".into())),
                ..Default::default()
            });
        let changed = pending.observation.is_some() || pending.terminal.is_some();
        if let Some(observation) = pending.observation {
            self.accept(observation);
        }
        if let Some(terminal) = pending.terminal {
            self.status = terminal;
        }
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
    pub fn retry_freshness(&mut self) {
        if let Some(context) = self.query_context.clone() {
            self.freshness.start(context, self.jobs.waker.clone());
        }
    }
}

pub fn poll(state: &mut State) -> bool {
    if state.closing {
        state.freshness.stop();
        return false;
    }
    if !state.uses_fixture_queries() {
        state
            .freshness
            .sync(state.query_context.clone(), state.jobs.waker.clone());
    }
    state.freshness.take()
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
