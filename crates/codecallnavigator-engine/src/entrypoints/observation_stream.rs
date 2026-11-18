//! Sequential verified polling, independent of analysis and notification delivery.
use super::{
    observation_transport::{Input, Output},
    query,
    query_executor::Executor,
    refresh_coordinator::Coordinator,
};
use codecallnavigator_core::{
    freshness::{self, EventPayload, FreshnessObservation, StreamEvent},
    query::{Error, Query, Request},
    refresh::{self, ObserveSession, Phase},
};
use serde::Deserialize;
use std::{
    path::PathBuf,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(untagged)]
enum ObserveRequest {
    Saved(Request),
    Selected(ObserveSession),
}

struct Selected {
    project: PathBuf,
    observation: Option<Request>,
}

impl Selected {
    fn observe(&mut self, phase: &Phase) {
        if let Phase::Published { roots, .. } = phase {
            self.observation = Some(Request {
                version: codecallnavigator_core::query::VERSION,
                project: self.project.clone(),
                roots: roots
                    .iter()
                    .filter(|(name, _)| name != "repo")
                    .cloned()
                    .collect(),
                expected_generation: None,
                query: Query::Freshness {
                    observation_version: freshness::VERSION,
                },
            });
        }
    }
}

pub(super) fn run(automatic: bool) -> anyhow::Result<i32> {
    let input = Input::<ObserveRequest>::start(automatic);
    let output = Output::start(input.stopped.clone());
    let Some(request) = input.receive_initial() else {
        return Ok(0);
    };
    let request = match request.and_then(|request| validate(request, automatic)) {
        Ok(request) => request,
        Err(error) => {
            output.finish(event(1, EventPayload::Failed { error }));
            return Ok(2);
        }
    };
    let mut monitor = Monitor::new(automatic);
    let mode = match request {
        ObserveRequest::Saved(request) => Mode::Saved(request),
        ObserveRequest::Selected(request) => monitor.select(request),
    };
    Ok(monitor.run(mode, &input, &output))
}

enum Mode {
    Saved(Request),
    Selected(Selected),
}

fn validate(
    request: ObserveRequest,
    automatic: bool,
) -> codecallnavigator_core::query::Result<ObserveRequest> {
    match &request {
        ObserveRequest::Saved(saved) => validate_saved(saved, automatic)?,
        ObserveRequest::Selected(selected) => validate_selected(selected, automatic)?,
    }
    Ok(request)
}

fn validate_selected(
    selected: &ObserveSession,
    automatic: bool,
) -> codecallnavigator_core::query::Result<()> {
    if !matches!(
        (automatic, selected.version, selected.session.version),
        (
            true,
            refresh::OBSERVE_SESSION_VERSION,
            refresh::SESSION_VERSION
        )
    ) {
        return Err(Error::new(
            "unsupported_capability",
            "Initial observation requires a supported automatic analysis session",
        ));
    }
    if selected.session.expected_generation.is_some() || selected.session.targets.is_empty() {
        return Err(Error::new(
            "invalid_request",
            "Initial observation requires an unpinned selection of builds",
        ));
    }
    Ok(())
}

fn validate_saved(request: &Request, automatic: bool) -> codecallnavigator_core::query::Result<()> {
    query::validate(request)?;
    if automatic && request.expected_generation.is_some() {
        return Err(Error::new(
            "invalid_request",
            "Automatic refresh requires an unpinned observation; omit expected_generation",
        ));
    }
    if !matches!(request.query, Query::Freshness { .. }) {
        return Err(Error::new(
            "invalid_request",
            "observe requires a freshness query",
        ));
    }
    Ok(())
}

#[derive(Default)]
struct Monitor {
    executor: Executor,
    delivered: Option<Vec<u8>>,
    sequence: u64,
    last_delivery: Option<Instant>,
    coordinator: Option<Coordinator>,
    delivered_activity: Option<Vec<u8>>,
    activity_at: Option<Instant>,
}

impl Monitor {
    fn new(automatic: bool) -> Self {
        Self {
            coordinator: automatic.then(Coordinator::default),
            ..Default::default()
        }
    }

    fn select(&mut self, request: ObserveSession) -> Mode {
        self.coordinator
            .as_mut()
            .expect("selected observation requires automatic refresh")
            .start_explicit(&request.session);
        Mode::Selected(Selected {
            project: request.session.project,
            observation: None,
        })
    }

    fn run(&mut self, mut mode: Mode, input: &Input<ObserveRequest>, output: &Output) -> i32 {
        while !input.stopped.load(Ordering::Acquire) {
            let started = Instant::now();
            self.apply_control(input);
            match self.reconcile(&mut mode, input, output) {
                Ok(true) => {}
                Ok(false) => return 2,
                Err(error) if !input.stopped.load(Ordering::Acquire) => {
                    self.fail(error, output);
                    return 2;
                }
                Err(_) => break,
            }
            self.wait(started, input);
        }
        let clean = self.shutdown(output);
        if clean && let Ok(Err(error)) = input.requests.try_recv() {
            output.finish(event(self.sequence + 1, EventPayload::Failed { error }));
            return 2;
        }
        if clean { 0 } else { 2 }
    }

    fn reconcile(
        &mut self,
        mode: &mut Mode,
        input: &Input<ObserveRequest>,
        output: &Output,
    ) -> codecallnavigator_core::query::Result<bool> {
        match mode {
            Mode::Saved(request) => {
                let observed = self.executor.observation(request, &input.stopped)?;
                self.deliver(observed.current.clone(), output);
                self.refresh(&observed.current, &input.stopped);
            }
            Mode::Selected(selected) => self.reconcile_selected(selected, input, output)?,
        }
        self.deliver_activity(output, matches!(mode, Mode::Selected(_)));
        Ok(!self.terminal(output))
    }

    fn reconcile_selected(
        &mut self,
        selected: &mut Selected,
        input: &Input<ObserveRequest>,
        output: &Output,
    ) -> codecallnavigator_core::query::Result<()> {
        let coordinator = self
            .coordinator
            .as_mut()
            .expect("selected observation coordinator");
        coordinator.poll();
        if let Some(publication) = coordinator.publication() {
            selected.observe(&publication.phase);
        }
        if let Some(request) = &selected.observation {
            let observed = self.executor.observation(request, &input.stopped)?;
            self.deliver(observed.current, output);
        }
        Ok(())
    }

    fn fail(&mut self, error: Error, output: &Output) {
        if self.shutdown(output) {
            output.finish(event(self.sequence + 1, EventPayload::Failed { error }));
        }
    }

    fn apply_control(&mut self, input: &Input<ObserveRequest>) {
        if input.cancel_refresh.swap(false, Ordering::AcqRel)
            && let Some(coordinator) = &mut self.coordinator
        {
            coordinator.cancel();
        }
    }

    fn refresh(
        &mut self,
        observation: &FreshnessObservation,
        cancel: &std::sync::atomic::AtomicBool,
    ) {
        if let Some(coordinator) = &mut self.coordinator {
            coordinator.observe(observation, || self.executor.refresh_plan(cancel));
        }
    }

    fn shutdown(&mut self, output: &Output) -> bool {
        let clean = self.coordinator.as_mut().is_none_or(Coordinator::shutdown);
        self.deliver_activity(output, false);
        let failed = self.terminal(output);
        clean && !failed
    }

    fn terminal(&mut self, output: &Output) -> bool {
        let Some(coordinator) = &mut self.coordinator else {
            return false;
        };
        let Some(mut error) = coordinator.take_terminal() else {
            return false;
        };
        coordinator.shutdown();
        if let Some(cleanup) = coordinator.take_terminal()
            && cleanup.message != error.message
        {
            error
                .message
                .push_str(&format!("; session shutdown: {}", cleanup.message));
        }
        output.finish(event(self.sequence + 1, EventPayload::Failed { error }));
        self.sequence += 1;
        true
    }

    fn deliver_activity(&mut self, output: &Output, allow_before_observation: bool) {
        if self.delivered.is_none() && !allow_before_observation {
            return;
        }
        let Some(coordinator) = &self.coordinator else {
            return;
        };
        let publication = coordinator.publication().is_some();
        let activity = coordinator
            .publication()
            .cloned()
            .unwrap_or_else(|| coordinator.activity());
        let key = serde_json::to_vec(&activity).expect("typed activity serializes");
        if !self.activity_due(&key) {
            self.release_publication(publication);
            return;
        }
        if output.offer(event(
            self.sequence + 1,
            EventPayload::Activity { activity },
        )) {
            self.sequence += 1;
            self.delivered_activity = Some(key);
            self.activity_at = Some(Instant::now());
            self.release_publication(publication);
        }
    }

    fn activity_due(&self, key: &[u8]) -> bool {
        self.delivered_activity.as_deref() != Some(key)
            || self.activity_at.is_none_or(|at| {
                at.elapsed() >= Duration::from_millis(freshness::HEARTBEAT_MILLISECONDS)
            })
    }

    fn release_publication(&mut self, publication: bool) {
        if publication && let Some(coordinator) = &mut self.coordinator {
            coordinator.publication_delivered();
        }
    }

    fn deliver(&mut self, observation: FreshnessObservation, output: &Output) {
        // Timings vary on every verification; they do not represent an input change.
        let mut state = observation.clone();
        state.verification_milliseconds = 0.0;
        let key = serde_json::to_vec(&state).expect("typed observation serializes");
        let payload = match &self.delivered {
            None => EventPayload::Initial { observation },
            Some(previous) if previous != &key => EventPayload::Change { observation },
            _ if self.last_delivery.is_some_and(|when| {
                when.elapsed() >= Duration::from_millis(freshness::HEARTBEAT_MILLISECONDS)
            }) =>
            {
                EventPayload::Heartbeat { observation }
            }
            _ => return,
        };
        if output.offer(event(self.sequence + 1, payload)) {
            self.sequence += 1;
            self.delivered = Some(key);
            self.last_delivery = Some(Instant::now());
        }
    }

    fn wait(&self, started: Instant, input: &Input<ObserveRequest>) {
        let deadline = started + Duration::from_millis(freshness::RECONCILE_MILLISECONDS);
        while !input.stopped.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

fn event(sequence: u64, payload: EventPayload) -> StreamEvent {
    StreamEvent {
        version: freshness::VERSION,
        sequence,
        payload,
    }
}
