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
}

impl Input {
    pub fn start() -> Self {
        let (send, requests) = mpsc::sync_channel(2);
        let stopped = Arc::new(AtomicBool::new(false));
        let finished = stopped.clone();
        std::thread::spawn(move || {
            let mut input = io::stdin().lock();
            let request = read_request(&mut input);
            let valid = request.is_ok();
            let _ = send.send(request);
            if valid && let Err(error) = wait_for_eof(&mut input) {
                let _ = send.send(Err(error));
            }
            finished.store(true, Ordering::Release);
        });
        Self { requests, stopped }
    }
}

fn read_request(input: &mut impl BufRead) -> Result<Request> {
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
    serde_json::from_slice(&bytes).map_err(|error| Error::new("invalid_request", error.to_string()))
}

fn wait_for_eof(input: &mut impl Read) -> Result<()> {
    let mut bytes = [0u8; 4096];
    loop {
        let count = input
            .read(&mut bytes)
            .map_err(|error| Error::new("invalid_request", error.to_string()))?;
        if count == 0 {
            return Ok(());
        }
        if bytes[..count]
            .iter()
            .any(|byte| !byte.is_ascii_whitespace())
        {
            return Err(Error::new(
                "invalid_request",
                "observe accepts one request; keep stdin open until shutdown",
            ));
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
