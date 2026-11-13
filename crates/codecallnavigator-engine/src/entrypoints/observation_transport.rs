//! Process-owned observer I/O. Delivery is bounded and coalesced, never a journal.
use codecallnavigator_core::{
    freshness::StreamEvent,
    query::{Error, Request, Result},
};
use std::{
    io::{self, BufRead, Read, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    time::{Duration, Instant},
};

pub(super) struct Input {
    pub requests: Receiver<Result<Request>>,
    pub stopped: Arc<AtomicBool>,
    pub cancel_refresh: Arc<AtomicBool>,
}

impl Input {
    pub fn start(automatic: bool) -> Self {
        let (send, requests) = mpsc::sync_channel(2);
        let stopped = Arc::new(AtomicBool::new(false));
        let finished = stopped.clone();
        let cancel_refresh = Arc::new(AtomicBool::new(false));
        let control = cancel_refresh.clone();
        if automatic {
            bridge_signals(stopped.clone());
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

fn bridge_signals(stopped: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        while !stopped.load(Ordering::Acquire) {
            if super::cancellation::CANCELLED.load(Ordering::Acquire) {
                stopped.store(true, Ordering::Release);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
}

fn consume_input(automatic: bool, send: &SyncSender<Result<Request>>, control: &AtomicBool) {
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

fn first_request(input: &mut impl BufRead, automatic: bool) -> Option<Result<Request>> {
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

fn read_request(input: &mut impl BufRead) -> Result<Option<Request>> {
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
