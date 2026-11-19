//! Process-owned observer I/O. Delivery is bounded and coalesced, never a journal.
use codecallnavigator_core::{
    freshness::StreamEvent,
    query::{Error, Result},
};
use serde::de::DeserializeOwned;
use std::{
    io::{self, BufRead, Read, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    time::{Duration, Instant},
};

pub(super) struct Input<T> {
    pub requests: Receiver<Result<T>>,
    pub stopped: Arc<AtomicBool>,
    pub cancel_refresh: Arc<AtomicBool>,
}

impl<T: DeserializeOwned + Send + 'static> Input<T> {
    pub fn receive_initial(&self) -> Option<Result<T>> {
        loop {
            match self.requests.recv_timeout(Duration::from_millis(20)) {
                Ok(request) => return Some(request),
                Err(RecvTimeoutError::Timeout) if !self.stopped.load(Ordering::Acquire) => {}
                Err(_) => return None,
            }
        }
    }

    pub fn start(automatic: bool) -> Self {
        let (send, requests) = mpsc::sync_channel(2);
        let stopped = Arc::new(AtomicBool::new(false));
        let finished = stopped.clone();
        let cancel_refresh = Arc::new(AtomicBool::new(false));
        let control = cancel_refresh.clone();
        if automatic {
            bridge_stop(stopped.clone(), cancel_refresh.clone());
        }
        std::thread::spawn(move || {
            consume_input(automatic, &send, &control);
            finished.store(true, Ordering::Release);
        });
        Self {
            requests,
            stopped,
            cancel_refresh,
        }
    }
}

pub(super) fn bridge_stop(stopped: Arc<AtomicBool>, cancel_attempt: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        while !stopped.load(Ordering::Acquire) {
            if super::cancellation::CANCELLED.load(Ordering::Acquire) {
                stopped.store(true, Ordering::Release);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        cancel_attempt.store(true, Ordering::Release);
    });
}

fn consume_input<T: DeserializeOwned>(
    automatic: bool,
    send: &SyncSender<Result<T>>,
    control: &AtomicBool,
) {
    let mut input = io::stdin().lock();
    let Some(request) = first_request(&mut input, automatic) else {
        return;
    };
    let valid = request.is_ok();
    let _ = send.send(request);
    if !valid {
        return;
    }
    if let Err(error) = wait_for_commands(&mut input, automatic, control) {
        let _ = send.send(Err(error));
    }
}

fn first_request<T: DeserializeOwned>(
    input: &mut impl BufRead,
    automatic: bool,
) -> Option<Result<T>> {
    match read_request(input) {
        Ok(Some(request)) => Some(Ok(request)),
        Ok(None) if automatic => None,
        Ok(None) => Some(Err(Error::new(
            "invalid_request",
            "Observation request is empty",
        ))),
        Err(error) => Some(Err(error)),
    }
}

fn read_request<T: DeserializeOwned>(input: &mut impl BufRead) -> Result<Option<T>> {
    let mut bytes = Vec::new();
    input
        .take(4 * 1024 * 1024 + 1)
        .read_until(b'\n', &mut bytes)
        .map_err(|error| Error::new("invalid_request", error.to_string()))?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(Error::new(
            "invalid_request",
            "Observation request exceeds 4 MiB",
        ));
    }
    if bytes.is_empty() {
        return Ok(None);
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| Error::new("invalid_request", error.to_string()))
}

fn wait_for_commands(input: &mut impl BufRead, automatic: bool, cancel: &AtomicBool) -> Result<()> {
    loop {
        let mut bytes = Vec::new();
        let count = input
            .take(65537)
            .read_until(b'\n', &mut bytes)
            .map_err(|error| Error::new("invalid_request", error.to_string()))?;
        if count == 0 {
            return Ok(());
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        if !automatic || bytes.len() > 65536 {
            return Err(Error::new(
                "invalid_request",
                "observe accepts one request; automatic controls are bounded",
            ));
        }
        let control: codecallnavigator_core::refresh::Control = serde_json::from_slice(&bytes)
            .map_err(|error| Error::new("invalid_request", error.to_string()))?;
        match control {
            codecallnavigator_core::refresh::Control::CancelRefresh { version: 1 } => {
                cancel.store(true, Ordering::Release)
            }
            _ => {
                return Err(Error::new(
                    "invalid_request",
                    "Unsupported automatic-refresh control version",
                ));
            }
        }
    }
}

pub(super) struct Output {
    send: SyncSender<StreamEvent>,
    acknowledged: Arc<AtomicU64>,
}

impl Output {
    pub fn start(stopped: Arc<AtomicBool>) -> Self {
        let (send, receive) = mpsc::sync_channel::<StreamEvent>(1);
        let acknowledged = Arc::new(AtomicU64::new(0));
        let acknowledge = acknowledged.clone();
        // Never join: a client may stop draining stdout. Process exit releases
        // this writer even when its OS pipe is full, while EOF remains effective.
        std::thread::spawn(move || {
            let mut output = io::stdout().lock();
            for event in receive {
                let written = serde_json::to_writer(&mut output, &event)
                    .map_err(io::Error::other)
                    .and_then(|()| output.write_all(b"\n"))
                    .and_then(|()| output.flush());
                if written.is_err() {
                    stopped.store(true, Ordering::Release);
                    break;
                }
                acknowledge.store(event.sequence, Ordering::Release);
            }
        });
        Self { send, acknowledged }
    }

    pub fn offer(&self, event: StreamEvent) -> bool {
        match self.send.try_send(event) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => false,
        }
    }

    pub fn finish(&self, event: StreamEvent) {
        let deadline = Instant::now() + Duration::from_millis(250);
        while Instant::now() < deadline {
            if self.offer(event.clone()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        self.wait_for_delivery(event.sequence, deadline);
    }

    fn wait_for_delivery(&self, expected: u64, deadline: Instant) {
        while self.acknowledged.load(Ordering::Acquire) < expected && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entrypoints::session_notice::Notice;
    use codecallnavigator_core::{
        freshness::EventPayload,
        refresh::{Activity, Phase},
    };

    #[test]
    fn latest_running_stage_is_delivered_when_output_recovers_during_attempt() {
        let (send, received) = mpsc::sync_channel(1);
        let output = Output {
            send,
            acknowledged: Arc::new(AtomicU64::new(0)),
        };
        let mut notice = Notice::new(&output, false);
        notice.activity(0, Phase::Watching);
        let (ready, stages_recorded) = mpsc::channel();
        let (finish, finish_attempt) = mpsc::channel();
        std::thread::scope(|scope| {
            scope.spawn(move || {
                let phase = Phase::Running {
                    run: 1,
                    input_revision: None,
                    debounce_milliseconds: 300.0,
                    wait_milliseconds: 0.0,
                    progress_message: None,
                };
                notice.during_attempt(1, &phase, |progress| {
                    progress("Preparing inputs".into());
                    progress("Extracting TypeScript".into());
                    ready.send(()).unwrap();
                    let _ = finish_attempt.recv_timeout(Duration::from_secs(3));
                });
            });
            let timeout = Duration::from_secs(2);
            stages_recorded.recv_timeout(timeout).unwrap();
            received.recv_timeout(timeout).unwrap();
            let frame = received.recv_timeout(timeout).unwrap();
            assert_eq!(frame.sequence, 2);
            assert!(matches!(
                frame.payload,
                EventPayload::Activity { activity: Activity {
                    runs_started: 1,
                    phase: Phase::Running { run: 1, progress_message: Some(message), .. },
                    ..
                }} if message == "Extracting TypeScript"
            ));
            finish.send(()).unwrap();
        });
    }
}
