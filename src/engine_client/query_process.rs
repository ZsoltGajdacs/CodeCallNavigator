//! A serialized, cancellable connection to the public CLI query session.
use crate::runtime::Runtime;
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator::query::{Request, Response, VERSION};
use std::{
    io::{BufRead, BufReader, Read, Write},
    os::{fd::AsRawFd, unix::process::CommandExt},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Mutex, OnceLock, TryLockError,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(120);
const MAX_RESPONSE: u64 = 512 * 1024 * 1024;
const MAX_REQUEST: usize = 4 * 1024 * 1024;

pub(super) struct Client {
    runtime: Runtime,
    session: Mutex<Option<Session>>,
    flow_languages: OnceLock<Vec<String>>,
}

impl Client {
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            session: Mutex::new(None),
            flow_languages: OnceLock::new(),
        }
    }

    pub fn query(&self, request: &Request, cancel: &AtomicBool) -> Result<Response> {
        let started = Instant::now();
        let mut session = loop {
            active(cancel, started)?;
            match self.session.try_lock() {
                Ok(session) => break session,
                Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(10)),
                Err(TryLockError::Poisoned(_)) => bail!("CLI query connection failed"),
            }
        };
        active(cancel, started)?;
        if session.is_none() {
            let languages = super::capabilities::check(&self.runtime, cancel, started)?;
            let _ = self.flow_languages.set(languages);
            *session = Some(Session::start(&self.runtime)?);
        }
        let result = session
            .as_mut()
            .expect("started session")
            .query(request, cancel, started);
        if result.is_err() {
            session.take();
        }
        result
    }

    pub fn flow_supported(&self, language: &str) -> bool {
        self.flow_languages
            .get()
            .is_some_and(|languages| languages.iter().any(|supported| supported == language))
    }
}

pub(super) fn active(cancel: &AtomicBool, started: Instant) -> Result<()> {
    ensure!(!cancel.load(Ordering::Relaxed), "CLI query cancelled");
    ensure!(started.elapsed() < TIMEOUT, "CLI query timed out");
    Ok(())
}

struct Session {
    child: Child,
    input: Option<ChildStdin>,
    responses: mpsc::Receiver<Result<Response>>,
    _log: tempfile::NamedTempFile,
}

impl Session {
    fn start(runtime: &Runtime) -> Result<Self> {
        let log = tempfile::NamedTempFile::new()?;
        let mut child = Command::new(&runtime.engine)
            .arg("query-session")
            .env("CODECALLNAVIGATOR_RESOURCES", &runtime.resources)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log.reopen()?))
            .process_group(0)
            .spawn()
            .context("Cannot start CLI query session")?;
        let input = child
            .stdin
            .take()
            .context("CLI query stdin is unavailable")?;
        nonblocking(&input).inspect_err(|_| stop_child(&mut child))?;
        let output = child
            .stdout
            .take()
            .context("CLI query stdout is unavailable")?;
        let (sender, responses) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let parsed = read_response(&mut reader);
                let failed = parsed.is_err();
                if sender.send(parsed).is_err() || failed {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            input: Some(input),
            responses,
            _log: log,
        })
    }

    fn query(
        &mut self,
        request: &Request,
        cancel: &AtomicBool,
        started: Instant,
    ) -> Result<Response> {
        active(cancel, started)?;
        let input = self.input.as_mut().context("CLI query session is closed")?;
        let mut bytes = serde_json::to_vec(request)?;
        ensure!(
            bytes.len() <= MAX_REQUEST,
            "CLI query request exceeds 4 MiB"
        );
        bytes.push(b'\n');
        write_request(input, &bytes, cancel, started)?;
        loop {
            active(cancel, started)?;
            if let Some(status) = self.child.try_wait()? {
                bail!("CLI query session exited ({status})");
            }
            match self.responses.recv_timeout(Duration::from_millis(20)) {
                Ok(response) => {
                    let response = response?;
                    ensure!(
                        response.version == VERSION,
                        "Unsupported CLI query response version"
                    );
                    return Ok(response);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    bail!("CLI query session disconnected")
                }
            }
        }
    }
}

fn nonblocking(input: &ChildStdin) -> Result<()> {
    let descriptor = input.as_raw_fd();
    // SAFETY: ChildStdin owns this live descriptor throughout configuration.
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    ensure!(
        flags >= 0,
        "Cannot inspect CLI query pipe flags: {}",
        std::io::Error::last_os_error()
    );
    let result = unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    ensure!(
        result >= 0,
        "Cannot make CLI query writes cancellable: {}",
        std::io::Error::last_os_error()
    );
    Ok(())
}

fn write_request(
    input: &mut ChildStdin,
    mut bytes: &[u8],
    cancel: &AtomicBool,
    started: Instant,
) -> Result<()> {
    while !bytes.is_empty() {
        active(cancel, started)?;
        match input.write(bytes) {
            Ok(0) => bail!("CLI query closed its request stream"),
            Ok(count) => bytes = &bytes[count..],
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn read_response(reader: &mut impl BufRead) -> Result<Response> {
    let mut bytes = Vec::new();
    let count = reader
        .take(MAX_RESPONSE + 1)
        .read_until(b'\n', &mut bytes)?;
    ensure!(count > 0, "CLI query session closed its response stream");
    ensure!(
        count as u64 <= MAX_RESPONSE,
        "CLI query response exceeds 512 MiB"
    );
    serde_json::from_slice(&bytes).context("Invalid CLI query response")
}

pub(super) fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_some() {
        return;
    }
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGTERM);
    }
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(200) {
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.wait();
}

impl Drop for Session {
    fn drop(&mut self) {
        self.input.take();
        // Read-only query sessions own no detached build processes. Analysis has
        // its own descendant-aware lifecycle in process.rs.
        stop_child(&mut self.child);
    }
}
