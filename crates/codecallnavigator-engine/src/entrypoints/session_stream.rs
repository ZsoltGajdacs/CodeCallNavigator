//! One engine-owned session schedules attempts and retains validated preparation.
use super::{
    analysis::{self, Attempt},
    analysis_session::AnalysisSession,
    arguments,
    observation_transport::{Input, Output, bridge_stop},
    refresh_policy::{Key, Pending, Policy},
    session_inputs::Inputs,
};
use anyhow::{Result, ensure};
use codecallnavigator_core::{
    artifact::{CoordinatorLease, GenerationChanged, LockBusy},
    fingerprint::InputsChanged,
    freshness::{self, EventPayload, StreamEvent},
    protocol::Event,
    query::Error,
    refresh::{self, Activity, CancelReason, Phase, SessionRequest},
};
use codecallnavigator_engine::{analyzers::ConsumedRevision, process_ownership::CleanupFailure};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub(super) fn cli(arguments: arguments::Analysis) -> Result<i32> {
    let project = arguments.project.canonicalize()?;
    let targets = analysis::select_targets(&project, arguments.targets)?;
    let request = SessionRequest {
        version: refresh::SESSION_VERSION,
        project,
        roots: arguments.roots,
        targets,
        infer_linked_roots: false,
        expected_generation: None,
    };
    let stopped = Arc::new(AtomicBool::new(false));
    let cancel = Arc::new(AtomicBool::new(false));
    bridge_stop(stopped.clone(), cancel.clone());
    let output = Output::start(stopped.clone());
    let result = run(request, &stopped, &cancel, &mut Notice::new(&output));
    stopped.store(true, Ordering::Release);
    result?;
    Ok(0)
}

pub(super) fn serve() -> Result<i32> {
    let input = Input::<SessionRequest>::start(true);
    let output = Output::start(input.stopped.clone());
    let Some(request) = input.receive_initial() else {
        return Ok(0);
    };
    let mut notice = Notice::new(&output);
    let result = request
        .map_err(|error| anyhow::anyhow!(error.message))
        .and_then(|request| run(request, &input.stopped, &input.cancel_refresh, &mut notice));
    if let Err(error) = result {
        notice.failed(terminal_error(&error));
        return Ok(2);
    }
    if let Ok(Err(error)) = input.requests.try_recv() {
        notice.failed(error);
        return Ok(2);
    }
    Ok(0)
}

fn run(
    request: SessionRequest,
    stopped: &AtomicBool,
    cancel_attempt: &AtomicBool,
    notice: &mut Notice<'_>,
) -> Result<()> {
    ensure!(
        request.version == refresh::SESSION_VERSION,
        "Unsupported analysis session version"
    );
    let project = request.project.canonicalize()?;
    let Some(_lease) = wait_for_lease(&project, stopped, notice)? else {
        return Ok(());
    };
    let roots = analysis::roots(&project, request.roots)?;
    let inputs = match &request.expected_generation {
        Some(expected) => Inputs::saved(&project, &roots, Some(expected), &request.targets)?.0,
        None => Inputs::bootstrap(&project, &request.targets)?,
    };
    let session =
        AnalysisSession::new(project, roots, request.targets, request.infer_linked_roots)?;
    let controls = Controls {
        stopped,
        cancel_attempt,
    };
    let mut monitor = Monitor::new(request.expected_generation, inputs, session, controls);
    let result = monitor.run(notice);
    let cleanup = monitor.session.shutdown();
    cleanup?;
    result
}

fn wait_for_lease(
    project: &Path,
    stopped: &AtomicBool,
    notice: &mut Notice<'_>,
) -> Result<Option<CoordinatorLease>> {
    let mut delay = 1;
    while !stopped.load(Ordering::Acquire) {
        match CoordinatorLease::acquire(project) {
            Ok(lease) => return Ok(Some(lease)),
            Err(error) if error.is::<LockBusy>() => {
                notice.activity(
                    0,
                    Phase::Unavailable {
                        error: Error::new("coordinator_busy", error.to_string()),
                    },
                );
            }
            Err(error) => return Err(error),
        }
        let deadline = Instant::now() + Duration::from_secs(delay);
        delay = (delay * 2).min(5);
        while !stopped.load(Ordering::Acquire) && Instant::now() < deadline {
            notice.flush();
            thread::sleep(Duration::from_millis(20));
        }
    }
    Ok(None)
}

struct Controls<'a> {
    stopped: &'a AtomicBool,
    cancel_attempt: &'a AtomicBool,
}

struct ActiveAttempt {
    pending: Pending,
    started: Instant,
}

impl ActiveAttempt {
    fn revision(&self) -> Option<String> {
        Some(self.pending.key.revision.clone())
    }
    fn milliseconds(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * 1000.0
    }
}

struct Monitor<'a> {
    session: AnalysisSession,
    controls: Controls<'a>,
    policy: Policy,
    inputs: Inputs,
    failed_inputs: Option<Inputs>,
    baseline: Option<String>,
    generation: Option<String>,
    runs: u64,
    retry_at: Instant,
    backoff: u64,
    last_failure: Option<(String, String)>,
}

impl<'a> Monitor<'a> {
    fn new(
        generation: Option<String>,
        inputs: Inputs,
        session: AnalysisSession,
        controls: Controls<'a>,
    ) -> Self {
        let baseline = generation.as_ref().map(|_| inputs.expected());
        Self {
            session,
            controls,
            policy: Policy::default(),
            inputs,
            failed_inputs: None,
            baseline,
            generation,
            runs: 0,
            retry_at: Instant::now(),
            backoff: 1000,
            last_failure: None,
        }
    }

    fn run(&mut self, notice: &mut Notice<'_>) -> Result<()> {
        notice.activity(self.runs, Phase::Watching);
        while !self.controls.stopped.load(Ordering::Acquire) {
            notice.flush();
            let started = Instant::now();
            let result = self.tick(notice);
            if result
                .as_ref()
                .is_err_and(|error| error.is::<CleanupFailure>())
            {
                return result;
            }
            if self.controls.stopped.load(Ordering::Acquire) {
                break;
            }
            result?;
            wait_for_probe(started, self.controls.stopped);
        }
        Ok(())
    }

    fn tick(&mut self, notice: &mut Notice<'_>) -> Result<()> {
        let current = self.observe_inputs()?;
        if self.cancel_pending(&current, notice) || self.controls.stopped.load(Ordering::Acquire) {
            return Ok(());
        }
        let Some(active) = self.begin_attempt(current, notice) else {
            return Ok(());
        };
        let result = self.session.attempt(
            self.generation.clone(),
            Attempt {
                cancel: self.controls.cancel_attempt,
                progress: &|message| eprintln!("{message}"),
            },
        );
        if self.controls.stopped.load(Ordering::Acquire) {
            return result.map(|_| ());
        }
        self.finish_attempt(result, active, notice)
    }

    fn observe_inputs(&mut self) -> Result<String> {
        let current = self.observed_inputs().current(self.controls.stopped)?;
        let key = Key {
            generation: self.generation.clone(),
            revision: current.clone(),
        };
        let changed = self.baseline.as_ref() != Some(&current);
        self.policy.observe(key, changed, Instant::now());
        Ok(current)
    }

    fn cancel_pending(&mut self, current: &str, notice: &mut Notice<'_>) -> bool {
        if !self.controls.cancel_attempt.swap(false, Ordering::AcqRel) {
            return false;
        }
        self.policy.cancel();
        notice.activity(
            self.runs,
            Phase::Cancelled {
                run: None,
                input_revision: Some(current.into()),
                reason: CancelReason::UserCancelled,
                analysis_milliseconds: 0.0,
            },
        );
        true
    }

    fn begin_attempt(&mut self, current: String, notice: &mut Notice<'_>) -> Option<ActiveAttempt> {
        let now = Instant::now();
        if self.policy.due(now).is_none() {
            self.debouncing(current, notice);
            return None;
        }
        if now < self.retry_at {
            return None;
        }
        let pending = self.policy.take().expect("due session attempt");
        self.runs += 1;
        notice.activity(
            self.runs,
            Phase::Running {
                run: self.runs,
                input_revision: Some(current),
                debounce_milliseconds: refresh::DEBOUNCE_MILLISECONDS as f64,
                wait_milliseconds: pending.queue_milliseconds(now),
            },
        );
        Some(ActiveAttempt {
            pending,
            started: now,
        })
    }

    fn debouncing(&self, current: String, notice: &mut Notice<'_>) {
        if let Some(pending) = &self.policy.pending {
            notice.activity(
                self.runs,
                Phase::Debouncing {
                    input_revision: Some(current),
                    remaining_milliseconds: refresh::DEBOUNCE_MILLISECONDS
                        .saturating_sub(pending.changed.elapsed().as_millis() as u64),
                },
            );
        }
    }

    fn observed_inputs(&self) -> &Inputs {
        self.failed_inputs.as_ref().unwrap_or(&self.inputs)
    }

    fn finish_attempt(
        &mut self,
        result: Result<Event>,
        active: ActiveAttempt,
        notice: &mut Notice<'_>,
    ) -> Result<()> {
        let analysis_milliseconds = active.milliseconds();
        let input_revision = active.revision();
        match result {
            Ok(event) => self.published(event, active, notice),
            Err(error) if error.is::<LockBusy>() => {
                self.retry(active.pending);
                notice.activity(
                    self.runs,
                    Phase::Busy {
                        run: Some(self.runs),
                        input_revision,
                        retry_milliseconds: self.backoff,
                        wait_milliseconds: 0.0,
                    },
                );
            }
            Err(error) if error.is::<CleanupFailure>() => return Err(error),
            Err(error) if error.is::<GenerationChanged>() => self.follow_latest_saved(notice)?,
            Err(_) if self.controls.cancel_attempt.swap(false, Ordering::AcqRel) => {
                self.policy.suppress(active.pending.key);
                notice.activity(
                    self.runs,
                    Phase::Cancelled {
                        run: Some(self.runs),
                        input_revision,
                        reason: CancelReason::UserCancelled,
                        analysis_milliseconds,
                    },
                );
            }
            Err(error) => self.failed(error, active, notice)?,
        }
        Ok(())
    }

    fn follow_latest_saved(&mut self, notice: &mut Notice<'_>) -> Result<()> {
        let (inputs, generation) = self.session.latest_saved_inputs()?;
        self.inputs = inputs;
        self.baseline = Some(self.inputs.expected());
        self.generation = Some(generation);
        self.failed_inputs = None;
        self.policy = Policy::default();
        self.last_failure = None;
        self.clear_backoff();
        notice.activity(self.runs, Phase::Watching);
        Ok(())
    }

    fn published(&mut self, event: Event, active: ActiveAttempt, notice: &mut Notice<'_>) {
        let Event::Complete {
            digest,
            fingerprints,
            metrics,
            ..
        } = event
        else {
            unreachable!("locked publication returns a complete result");
        };
        let input_revision = active.revision();
        let analysis_milliseconds = active.milliseconds();
        self.inputs = Inputs::captured(fingerprints);
        self.failed_inputs = None;
        self.baseline = Some(self.inputs.expected());
        self.generation = Some(digest.clone());
        self.policy.suppress(active.pending.key);
        self.last_failure = None;
        self.clear_backoff();
        notice.activity(
            self.runs,
            Phase::Published {
                run: self.runs,
                input_revision,
                generation: digest,
                analysis_milliseconds,
                metrics,
            },
        );
    }

    fn failed(
        &mut self,
        error: anyhow::Error,
        active: ActiveAttempt,
        notice: &mut Notice<'_>,
    ) -> Result<()> {
        let analysis_milliseconds = active.milliseconds();
        let input_revision = active.revision();
        let changed = error.is::<InputsChanged>();
        self.settle_failure(&error, active.pending)?;
        notice.activity(
            self.runs,
            Phase::Failed {
                run: self.runs,
                input_revision,
                error: bounded_error(
                    if changed {
                        "inputs_changed"
                    } else {
                        "analysis_failed"
                    },
                    &format!("{error:#}"),
                ),
                analysis_milliseconds,
            },
        );
        Ok(())
    }

    fn settle_failure(&mut self, error: &anyhow::Error, pending: Pending) -> Result<()> {
        let changed = error.is::<InputsChanged>();
        let external_change =
            self.observed_inputs().current(self.controls.stopped)? != pending.key.revision;
        let expanded = self
            .inputs
            .with_candidates(self.session.bootstrap_inputs()?);
        let mut settled = pending.key;
        let consumed = error.downcast_ref::<ConsumedRevision>().map_or_else(
            || settled.revision.clone(),
            |revision| revision.as_str().into(),
        );
        settled.revision = expanded.current(self.controls.stopped)?;
        self.failed_inputs = Some(expanded);
        let pair = (consumed, settled.revision.clone());
        let inputs_changed = changed || external_change;
        let repeated_failure = self.last_failure.as_ref() == Some(&pair);
        if inputs_changed && !repeated_failure {
            self.schedule_catchup(settled);
        } else {
            self.policy.suppress(settled);
        }
        self.last_failure = Some(pair);
        Ok(())
    }

    fn retry(&mut self, pending: Pending) {
        self.policy.retry(pending);
        self.retry_at = Instant::now() + Duration::from_millis(self.backoff);
        self.backoff = (self.backoff * 2).min(5000);
    }

    fn clear_backoff(&mut self) {
        self.backoff = 1000;
        self.retry_at = Instant::now();
    }

    fn schedule_catchup(&mut self, key: Key) {
        self.policy.reschedule(key, Instant::now());
        self.retry_at = Instant::now() + Duration::from_millis(self.backoff);
        self.backoff = (self.backoff * 2).min(5000);
    }
}

fn wait_for_probe(started: Instant, stopped: &AtomicBool) {
    let deadline = started + Duration::from_millis(freshness::RECONCILE_MILLISECONDS);
    while !stopped.load(Ordering::Acquire) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
}

struct Notice<'a> {
    output: &'a Output,
    sequence: u64,
    pending: Option<StreamEvent>,
    publication: Option<StreamEvent>,
}

impl<'a> Notice<'a> {
    fn new(output: &'a Output) -> Self {
        Self {
            output,
            sequence: 0,
            pending: None,
            publication: None,
        }
    }

    fn activity(&mut self, runs_started: u64, phase: Phase) {
        let published = matches!(phase, Phase::Published { .. });
        let event = StreamEvent {
            sequence: 0,
            version: freshness::VERSION,
            payload: EventPayload::Activity {
                activity: Activity {
                    version: refresh::VERSION,
                    runs_started,
                    phase,
                },
            },
        };
        if published {
            self.pending = None;
            self.publication = Some(event);
        } else {
            self.pending = Some(event);
        }
        self.flush();
    }

    fn flush(&mut self) {
        for slot in [&mut self.publication, &mut self.pending] {
            if let Some(event) = slot.as_mut() {
                event.sequence = self.sequence + 1;
                if !self.output.offer(event.clone()) {
                    break;
                }
                self.sequence += 1;
                *slot = None;
            }
        }
    }

    fn failed(&mut self, error: Error) {
        self.flush();
        self.output.finish(StreamEvent {
            sequence: self.sequence + 1,
            version: freshness::VERSION,
            payload: EventPayload::Failed { error },
        });
    }
}

fn terminal_error(error: &anyhow::Error) -> Error {
    let code = if error.is::<CleanupFailure>() {
        "cleanup_unconfirmed"
    } else {
        "analysis_session_failed"
    };
    bounded_error(code, &format!("{error:#}"))
}

fn bounded_error(code: &str, message: &str) -> Error {
    let mut chars = message.chars();
    let mut bounded = chars.by_ref().take(4096).collect::<String>();
    if chars.next().is_some() {
        bounded.push_str(" … [truncated]");
    }
    Error::new(code, bounded)
}
