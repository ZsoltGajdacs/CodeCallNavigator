//! Bounded monitor delivery and owned background cleanup, independent of UI state.
use super::{Pending, Status};
use crate::engine_client::{Notice, ObservationOptions, QueryContext};
use codecallnavigator::refresh::{ObserveSession, Phase};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

pub(super) enum Mode {
    Passive,
    Saved,
    Selected(ObserveSession),
}

impl Mode {
    fn selection(&self) -> Option<&ObserveSession> {
        match self {
            Self::Selected(selection) => Some(selection),
            _ => None,
        }
    }
}

pub(super) struct Worker {
    pub automatic: bool,
    pub cancel: Arc<AtomicBool>,
    pub finished: Arc<AtomicBool>,
    pub pending: Arc<Mutex<Pending>>,
    pub controls: mpsc::SyncSender<codecallnavigator::refresh::Control>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

impl Worker {
    pub fn start(
        context: QueryContext,
        wake: Option<rune::Waker>,
        mode: Mode,
        previous: Vec<Arc<AtomicBool>>,
    ) -> Self {
        let (controls, commands) = mpsc::sync_channel(1);
        let worker = Self {
            automatic: !matches!(mode, Mode::Passive),
            cancel: Arc::new(AtomicBool::new(false)),
            finished: Arc::new(AtomicBool::new(false)),
            pending: Arc::new(Mutex::new(Pending::default())),
            controls,
        };
        let task = Task {
            context,
            wake,
            mode,
            previous,
            commands,
            cancel: worker.cancel.clone(),
            finished: worker.finished.clone(),
            pending: worker.pending.clone(),
        };
        std::thread::spawn(move || task.run());
        worker
    }
}

struct Task {
    context: QueryContext,
    wake: Option<rune::Waker>,
    mode: Mode,
    previous: Vec<Arc<AtomicBool>>,
    commands: mpsc::Receiver<codecallnavigator::refresh::Control>,
    cancel: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    pending: Arc<Mutex<Pending>>,
}

impl Task {
    fn run(self) {
        self.wait_previous();
        let result = self.observe();
        self.report_cleanup(&result);
        if !self.cancel.load(Ordering::Acquire) {
            self.complete(result);
        }
        self.finished.store(true, Ordering::Release);
        self.wake();
    }

    fn wait_previous(&self) {
        while self
            .previous
            .iter()
            .any(|finished| !finished.load(Ordering::Acquire))
            && !self.cancel.load(Ordering::Acquire)
        {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    fn observe(&self) -> anyhow::Result<bool> {
        if self.cancel.load(Ordering::Acquire) {
            return Ok(true);
        }
        if matches!(self.mode, Mode::Passive) {
            return self.context.observe(&self.cancel, &|observation| {
                self.publish(Notice::Observation(observation))
            });
        }
        self.context.observe_automatic(
            &self.cancel,
            ObservationOptions {
                automatic: true,
                selection: self.mode.selection(),
                receive: &|notice| self.publish(notice),
                controls: Some(&self.commands),
            },
        )
    }

    fn publish(&self, notice: Notice) {
        if let Ok(mut pending) = self.pending.lock() {
            match notice {
                Notice::Observation(observation) => pending.observation = Some(observation),
                Notice::Activity(activity) => {
                    if self.mode.selection().is_some()
                        && matches!(activity.phase, Phase::Published { .. })
                    {
                        pending.publication = Some(activity.clone());
                    }
                    pending.activity = Some(activity);
                }
            }
        }
        self.wake();
    }

    fn report_cleanup(&self, result: &anyhow::Result<bool>) {
        let Some(error) = result
            .as_ref()
            .err()
            .filter(|error| error.is::<crate::engine_client::CleanupFailure>())
        else {
            return;
        };
        let warning = format!("{error:#}").chars().take(4096).collect::<String>();
        eprintln!("{warning}");
        if let Ok(mut pending) = self.pending.lock() {
            pending.cleanup_warning = Some(warning);
        }
    }

    fn complete(&self, result: anyhow::Result<bool>) {
        let terminal = match result {
            Ok(false) => Some(Status::Unsupported),
            Err(error) => Some(Status::Failed(format!("{error:#}"))),
            Ok(true) => None,
        };
        if let Ok(mut pending) = self.pending.lock() {
            pending.terminal = terminal;
        }
    }

    fn wake(&self) {
        if let Some(wake) = &self.wake {
            wake();
        }
    }
}
