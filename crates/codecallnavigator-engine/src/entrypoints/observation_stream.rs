//! Sequential verified polling, independent of analysis and notification delivery.
use super::{
    observation_transport::{Input, Output},
    query,
    query_executor::Executor,
};
use codecallnavigator_core::{
    freshness::{self, EventPayload, FreshnessObservation, StreamEvent},
    query::{Error, Query, Request},
};
use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

pub(super) fn run() -> anyhow::Result<i32> {
    let input = Input::start();
    let output = Output::start(input.stopped.clone());
    let request = input.requests.recv().map_err(anyhow::Error::new)?;
    let request = match request.and_then(validate) {
        Ok(request) => request,
        Err(error) => {
            output.finish(event(1, EventPayload::Failed { error }));
            return Ok(2);
        }
    };
    Ok(Monitor::default().run(&request, &input, &output))
}

fn validate(request: Request) -> codecallnavigator_core::query::Result<Request> {
    query::validate(&request)?;
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
}

impl Monitor {
    fn run(&mut self, request: &Request, input: &Input, output: &Output) -> i32 {
        while !input.stopped.load(Ordering::Acquire) {
            let started = Instant::now();
            match self.executor.observation(request, &input.stopped) {
                Ok(observed) => self.deliver(observed.current, output),
                Err(error) if !input.stopped.load(Ordering::Acquire) => {
                    output.finish(event(self.sequence + 1, EventPayload::Failed { error }));
                    return 2;
                }
                Err(_) => break,
            }
            self.wait(started, input);
        }
        if let Ok(Err(error)) = input.requests.try_recv() {
            output.finish(event(self.sequence + 1, EventPayload::Failed { error }));
            return 2;
        }
        0
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

    fn wait(&self, started: Instant, input: &Input) {
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
