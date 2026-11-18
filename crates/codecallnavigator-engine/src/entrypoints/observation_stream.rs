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
};
use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

pub(super) fn run(automatic: bool) -> anyhow::Result<i32> {
    let input = Input::start(automatic);
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
    Ok(Monitor::new(automatic).run(&request, &input, &output))
}

fn validate(request: Request, automatic: bool) -> codecallnavigator_core::query::Result<Request> {
    query::validate(&request)?;
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
    Ok(request)
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

    fn run(&mut self, request: &Request, input: &Input<Request>, output: &Output) -> i32 {
        while !input.stopped.load(Ordering::Acquire) {
            let started = Instant::now();
            self.apply_control(input);
            match self.reconcile(request, input, output) {
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
        request: &Request,
        input: &Input<Request>,
        output: &Output,
    ) -> codecallnavigator_core::query::Result<bool> {
        let observed = self.executor.observation(request, &input.stopped)?;
        self.deliver(observed.current.clone(), output);
        self.refresh(&observed.current, &input.stopped);
        self.deliver_activity(output);
        Ok(!self.terminal(output))
    }

    fn fail(&mut self, error: Error, output: &Output) {
        if self.shutdown(output) {
            output.finish(event(self.sequence + 1, EventPayload::Failed { error }));
        }
    }

    fn apply_control(&mut self, input: &Input<Request>) {
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
        self.deliver_activity(output);
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

    fn deliver_activity(&mut self, output: &Output) {
        if self.delivered.is_none() {
            return;
        }
        let Some(coordinator) = &self.coordinator else {
            return;
        };
        let publication = coordinator.publication().cloned();
        let activity = publication
            .clone()
            .unwrap_or_else(|| coordinator.activity());
        let key = serde_json::to_vec(&activity).expect("typed activity serializes");
        let changed = self.delivered_activity.as_ref() != Some(&key);
        let heartbeat = self.activity_at.is_none_or(|at| {
            at.elapsed() >= Duration::from_millis(freshness::HEARTBEAT_MILLISECONDS)
        });
        if !changed && !heartbeat {
            if publication.is_some() {
                self.coordinator
                    .as_mut()
                    .expect("selected coordinator")
                    .publication_delivered();
            }
            return;
        }
        if output.offer(event(
            self.sequence + 1,
            EventPayload::Activity { activity },
        )) {
            self.sequence += 1;
            self.delivered_activity = Some(key);
            self.activity_at = Some(Instant::now());
            if publication.is_some() {
                self.coordinator
                    .as_mut()
                    .expect("selected coordinator")
                    .publication_delivered();
            }
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

    fn wait(&self, started: Instant, input: &Input<Request>) {
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
