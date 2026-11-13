//! One fresh analysis process per refresh; cancellation never resets global state.
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator_core::{
    process_ownership::{Descendants, reap_eventually},
    protocol::{self, AnalysisFailureCode, Event, Request},
    refresh::CancelReason,
};
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const MAX_FRAME: u64 = 64 * 1024 * 1024;
const MAX_REQUEST: usize = 4 * 1024 * 1024;

pub(super) enum Outcome {
    Published {
        generation: String,
        metrics: serde_json::Value,
    },
    Failed {
        code: AnalysisFailureCode,
        message: String,
    },
    Cancelled(CancelReason),
    CleanupFailed(String),
}

pub(super) struct Finished {
    pub outcome: Outcome,
    pub milliseconds: f64,
}

pub(super) struct Job {
    cancellation: Arc<Mutex<Option<CancelReason>>>,
    result: mpsc::Receiver<Finished>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Job {
    pub fn start(request: &Request) -> Result<Self> {
        let mut bytes = serde_json::to_vec(request)?;
        ensure!(
            bytes.len() <= MAX_REQUEST,
            "Automatic analysis request exceeds 4 MiB"
        );
        bytes.push(b'\n');
        let mut session = Session::start(bytes)?;
        let cancellation = Arc::new(Mutex::new(None));
        let cancel = cancellation.clone();
        let (send, result) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            let started = Instant::now();
            let result = session.run(&cancel);
            let cleanup = session.finish();
            let outcome = match (result, cleanup) {
                (Ok(outcome), Ok(())) => outcome,
                (_, Err(error)) => Outcome::CleanupFailed(session.diagnostic(&error)),
                (Err(error), Ok(())) => match reason(&cancel) {
                    Some(reason) => Outcome::Cancelled(reason),
                    None => Outcome::Failed {
                        code: AnalysisFailureCode::Failed,
                        message: session.diagnostic(&error),
                    },
                },
            };
            let _ = send.send(Finished {
                outcome,
                milliseconds: started.elapsed().as_secs_f64() * 1000.0,
            });
        });
        Ok(Self {
            cancellation,
            result,
            worker: Some(worker),
        })
    }

    pub fn cancel(&self, reason: CancelReason) {
        if let Ok(mut current) = self.cancellation.lock()
            && current.is_none_or(|previous| priority(reason) > priority(previous))
        {
            *current = Some(reason);
        }
    }

    pub fn take(&mut self) -> Option<Finished> {
        let result = self.result.try_recv().ok()?;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        Some(result)
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.cancel(CancelReason::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn priority(reason: CancelReason) -> u8 {
    match reason {
        CancelReason::Superseded => 1,
        CancelReason::UserCancelled => 2,
        CancelReason::Shutdown => 3,
    }
}

fn reason(cancel: &Mutex<Option<CancelReason>>) -> Option<CancelReason> {
    cancel.lock().ok().and_then(|value| *value)
}

enum Frame {
    Terminal(Outcome),
    Closed,
    Failed(String),
}

struct Session {
    child: Option<Child>,
    owned: Option<Descendants>,
    input: Option<ChildStdin>,
    outgoing: Vec<u8>,
    offset: usize,
    frames: mpsc::Receiver<Frame>,
    stderr: Arc<Mutex<VecDeque<u8>>>,
    cancel_started: Option<Instant>,
    readers: Readers,
}

impl Session {
    fn start(outgoing: Vec<u8>) -> Result<Self> {
        let child = Command::new(std::env::current_exe()?)
            .arg("serve-once")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .context("Cannot start automatic analysis child")?;
        let (send, frames) = mpsc::sync_channel(2);
        let mut session = Self {
            child: Some(child),
            owned: None,
            input: None,
            outgoing,
            offset: 0,
            frames,
            stderr: Arc::new(Mutex::new(VecDeque::new())),
            cancel_started: None,
            readers: Readers::default(),
        };
        let child = session.child.as_mut().expect("owned analysis child");
        session.owned = Some(Descendants::new(child)?);
        let input = child.stdin.take().context("Analysis stdin unavailable")?;
        nonblocking(&input)?;
        session.input = Some(input);
        let stdout = child.stdout.take().context("Analysis stdout unavailable")?;
        let stderr = child.stderr.take().context("Analysis stderr unavailable")?;
        nonblocking(&stdout)?;
        nonblocking(&stderr)?;
        let log = session.stderr.clone();
        let stderr = session.readers.pipe(stderr);
        session.readers.spawn(move || drain_log(stderr, log));
        let stdout = session.readers.pipe(stdout);
        session.readers.spawn(move || read_events(stdout, send));
        Ok(session)
    }

    fn run(&mut self, cancel: &Mutex<Option<CancelReason>>) -> Result<Outcome> {
        let mut conversation = Conversation::default();
        let mut inventory_at = Instant::now();
        loop {
            if reason(cancel).is_some() {
                self.request_cancel()?;
            }
            self.write_pending()?;
            self.refresh_inventory(&mut inventory_at)?;
            conversation.receive(&self.frames)?;
            let status = self
                .child
                .as_mut()
                .expect("owned analysis child")
                .try_wait()?;
            if let Some(status) = status
                && let Some(outcome) = conversation.exited(status, cancel)?
            {
                return Ok(outcome);
            }
            if self.cancellation_expired() {
                return Ok(Outcome::Cancelled(
                    reason(cancel).context("Cancellation reason missing")?,
                ));
            }
            conversation.check_deadlines()?;
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn refresh_inventory(&mut self, at: &mut Instant) -> Result<()> {
        if at.elapsed() >= Duration::from_millis(100) {
            self.owned
                .as_mut()
                .expect("owned descendant inventory")
                .refresh()?;
            *at = Instant::now();
        }
        Ok(())
    }

    fn cancellation_expired(&self) -> bool {
        self.cancel_started
            .is_some_and(|when| when.elapsed() >= Duration::from_secs(5))
    }

    fn request_cancel(&mut self) -> Result<()> {
        if self.cancel_started.is_some() {
            return Ok(());
        }
        self.cancel_started = Some(Instant::now());
        if self.offset < self.outgoing.len() {
            self.input.take();
            return Ok(());
        }
        self.outgoing = serde_json::to_vec(
            &serde_json::json!({"version":protocol::VERSION,"command":"cancel"}),
        )?;
        self.outgoing.push(b'\n');
        self.offset = 0;
        Ok(())
    }

    fn write_pending(&mut self) -> Result<()> {
        let Some(input) = self.input.as_mut() else {
            return Ok(());
        };
        if self.offset == self.outgoing.len() {
            return Ok(());
        }
        let end = (self.offset + 65536).min(self.outgoing.len());
        match input.write(&self.outgoing[self.offset..end]) {
            Ok(0) => bail!("Analysis closed its request pipe"),
            Ok(count) => self.offset += count,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error.into()),
        }
        if self.offset == self.outgoing.len() && self.cancel_started.is_some() {
            self.input.take();
        }
        Ok(())
    }

    fn diagnostic(&self, error: &anyhow::Error) -> String {
        let tail = self
            .stderr
            .lock()
            .map(|log| {
                String::from_utf8_lossy(&log.iter().copied().collect::<Vec<_>>()).into_owned()
            })
            .unwrap_or_default();
        if tail.is_empty() {
            format!("{error:#}")
        } else {
            format!("{error:#} · child stderr: {tail}")
        }
    }

    fn finish(&mut self) -> Result<()> {
        self.input.take();
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        let result = match self.owned.as_mut() {
            Some(owned) => owned.stop(&mut child),
            None => stop_untracked(&mut child),
        };
        if !matches!(child.try_wait(), Ok(Some(_))) {
            reap_eventually(child);
        }
        self.readers.stop();
        result
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}

fn nonblocking(input: &impl AsRawFd) -> Result<()> {
    let descriptor = input.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    ensure!(
        flags >= 0
            && unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } >= 0,
        "Cannot configure bounded analysis pipe"
    );
    Ok(())
}

struct Conversation {
    started: Instant,
    ending: Option<Instant>,
    terminal: Option<Outcome>,
    closed: bool,
}
impl Default for Conversation {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            ending: None,
            terminal: None,
            closed: false,
        }
    }
}
impl Conversation {
    fn receive(&mut self, frames: &mpsc::Receiver<Frame>) -> Result<()> {
        for frame in frames.try_iter() {
            match frame {
                Frame::Terminal(value) => {
                    self.terminal = Some(value);
                    self.ending.get_or_insert_with(Instant::now);
                }
                Frame::Closed => self.closed = true,
                Frame::Failed(error) => bail!("Invalid analysis event stream: {error}"),
            }
        }
        Ok(())
    }
    fn exited(
        &mut self,
        status: std::process::ExitStatus,
        cancel: &Mutex<Option<CancelReason>>,
    ) -> Result<Option<Outcome>> {
        self.ending.get_or_insert_with(Instant::now);
        if !self.closed {
            return Ok(None);
        }
        let outcome = self
            .terminal
            .take()
            .context("Analysis exited without a terminal result")?;
        if matches!(outcome, Outcome::Published { .. }) {
            ensure!(
                status.success(),
                "Analysis failed after publication ({status})"
            );
            return Ok(Some(outcome));
        }
        Ok(Some(match reason(cancel) {
            Some(reason) => Outcome::Cancelled(reason),
            None => outcome,
        }))
    }
    fn check_deadlines(&self) -> Result<()> {
        ensure!(
            !self
                .ending
                .is_some_and(|when| when.elapsed() >= Duration::from_secs(5)),
            "Analysis did not close its terminal stream"
        );
        ensure!(
            self.started.elapsed() < Duration::from_secs(1800),
            "Automatic analysis timed out"
        );
        Ok(())
    }
}

// Readers must stop independently of EOF: an unrecorded inherited pipe holder
// must not leak a reader thread in a persistent observer after failed analysis.
#[derive(Default)]
struct Readers {
    stopped: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}
impl Readers {
    fn pipe<T: Read>(&self, pipe: T) -> StoppablePipe<T> {
        StoppablePipe {
            pipe,
            stopped: self.stopped.clone(),
        }
    }
    fn spawn(&mut self, run: impl FnOnce() + Send + 'static) {
        self.threads.push(std::thread::spawn(run));
    }
    fn stop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}
struct StoppablePipe<T> {
    pipe: T,
    stopped: Arc<AtomicBool>,
}
impl<T: Read> Read for StoppablePipe<T> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        while !self.stopped.load(Ordering::Acquire) {
            match self.pipe.read(bytes) {
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                result => return result,
            }
        }
        Ok(0)
    }
}
fn stop_untracked(child: &mut Child) -> Result<()> {
    let _ = child.kill();
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    bail!("Cannot confirm cleanup of analysis child before ownership initialization")
}

fn read_events(stdout: impl Read, send: mpsc::SyncSender<Frame>) {
    let result = parse_events(stdout, &send);
    let final_frame = match result {
        Ok(()) => Frame::Closed,
        Err(error) => Frame::Failed(format!("{error:#}")),
    };
    let _ = send.send(final_frame);
}

fn parse_events(stdout: impl Read, send: &mpsc::SyncSender<Frame>) -> Result<()> {
    let mut reader = BufReader::new(stdout);
    let (mut hello, mut terminal) = (false, false);
    loop {
        let mut bytes = Vec::new();
        let count = (&mut reader)
            .take(MAX_FRAME + 1)
            .read_until(b'\n', &mut bytes)?;
        if count == 0 {
            return Ok(());
        }
        ensure!(count as u64 <= MAX_FRAME, "Analysis event exceeds 64 MiB");
        let event: Event = serde_json::from_slice(&bytes).context("Malformed analysis event")?;
        ensure!(!terminal, "Analysis sent events after its terminal result");
        if !hello {
            ensure!(
                matches!(
                    event,
                    Event::Hello {
                        version: protocol::VERSION
                    }
                ),
                "Analysis omitted compatible hello"
            );
            hello = true;
            continue;
        }
        let outcome = match event {
            Event::Progress { .. } => continue,
            Event::Complete {
                digest, metrics, ..
            } => {
                ensure!(
                    digest.len() == 64
                        && digest
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
                    "Invalid analysis publication digest"
                );
                ensure!(metrics.is_object(), "Invalid analysis metrics object");
                Outcome::Published {
                    generation: digest,
                    metrics,
                }
            }
            Event::Failed { code, message } => Outcome::Failed {
                code: code.unwrap_or(AnalysisFailureCode::Failed),
                message,
            },
            _ => bail!("Unexpected automatic-analysis event"),
        };
        terminal = true;
        send.send(Frame::Terminal(outcome))
            .map_err(|_| anyhow::anyhow!("Analysis receiver closed"))?;
    }
}

fn drain_log(mut stderr: impl Read, log: Arc<Mutex<VecDeque<u8>>>) {
    let mut bytes = [0u8; 4096];
    while let Ok(count) = stderr.read(&mut bytes) {
        if count == 0 {
            break;
        }
        append_log(&log, &bytes[..count]);
    }
}

fn append_log(log: &Mutex<VecDeque<u8>>, bytes: &[u8]) {
    if let Ok(mut log) = log.lock() {
        log.extend(bytes);
        while log.len() > 8192 {
            log.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_cancel_overrides_internal_supersession_and_shutdown_is_final() {
        let cancellation = Arc::new(Mutex::new(None));
        let (_, result) = mpsc::sync_channel(1);
        let job = Job {
            cancellation: cancellation.clone(),
            result,
            worker: None,
        };
        job.cancel(CancelReason::Superseded);
        job.cancel(CancelReason::UserCancelled);
        job.cancel(CancelReason::Superseded);
        assert_eq!(reason(&cancellation), Some(CancelReason::UserCancelled));
        drop(job);
        assert_eq!(reason(&cancellation), Some(CancelReason::Shutdown));
    }

    struct HeldPipe(Child);
    impl Drop for HeldPipe {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn reader_shutdown_does_not_require_pipe_holder_exit() {
        // A still-live holder has the same pipe behavior as an unrecorded orphan
        // inheriting an exited compiler's descriptors. No analyzer is simulated.
        let mut holder = HeldPipe(
            Command::new("/bin/sleep")
                .arg("30")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let stdout = holder.0.stdout.take().unwrap();
        let stderr = holder.0.stderr.take().unwrap();
        nonblocking(&stdout).unwrap();
        nonblocking(&stderr).unwrap();
        let mut readers = Readers::default();
        let stdout = readers.pipe(stdout);
        let stderr = readers.pipe(stderr);
        let (send, result) = mpsc::sync_channel(2);
        let first = send.clone();
        readers.spawn(move || {
            let mut bytes = Vec::new();
            let mut stdout = stdout;
            stdout.read_to_end(&mut bytes).unwrap();
            first.send(()).unwrap();
        });
        readers.spawn(move || {
            let mut bytes = Vec::new();
            let mut stderr = stderr;
            stderr.read_to_end(&mut bytes).unwrap();
            send.send(()).unwrap();
        });
        assert!(result.recv_timeout(Duration::from_millis(50)).is_err());
        let started = Instant::now();
        readers.stop();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(result.try_iter().count(), 2);
        assert!(holder.0.try_wait().unwrap().is_none());
    }
}
