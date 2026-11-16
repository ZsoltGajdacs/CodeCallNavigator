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
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(120);
const MAX_RESPONSE: u64 = 512 * 1024 * 1024;
const MAX_REQUEST: usize = 4 * 1024 * 1024;

pub(super) struct ReleaseGuard<'a> {
    revision: &'a AtomicU64,
    expected: u64,
}

impl<'a> ReleaseGuard<'a> {
    pub(super) fn new(revision: &'a AtomicU64, expected: u64) -> Self {
        Self { revision, expected }
    }
}

enum Dispatch<'a> {
    Required,
    Release(ReleaseGuard<'a>),
}

impl Dispatch<'_> {
    fn allowed(&self, connected: bool) -> bool {
        match self {
            Self::Required => true,
            Self::Release(guard) => {
                if guard.revision.load(Ordering::Acquire) != guard.expected {
                    return false;
                }
                connected
            }
        }
    }
}

pub(super) struct Client {
    runtime: Runtime,
    session: Mutex<Option<Session>>,
    capabilities: OnceLock<super::capabilities::Capabilities>,
}

impl Client {
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            session: Mutex::new(None),
            capabilities: OnceLock::new(),
        }
    }

    pub fn query(&self, request: &Request, cancel: &AtomicBool) -> Result<Response> {
        self.query_when(request, cancel, Dispatch::Required)?
            .context("CLI query was not dispatched")
    }

    /// Recheck UI intent while holding the same lock that serializes flow loads.
    /// An obsolete release never follows a newer flow request, and releasing an
    /// unused connection must not launch a CLI process merely to unload it.
    pub(super) fn release_flows(
        &self,
        request: &Request,
        guard: ReleaseGuard<'_>,
        cancel: &AtomicBool,
    ) -> Result<Option<Response>> {
        self.query_when(request, cancel, Dispatch::Release(guard))
    }

    fn query_when(
        &self,
        request: &Request,
        cancel: &AtomicBool,
        dispatch: Dispatch<'_>,
    ) -> Result<Option<Response>> {
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
        if !dispatch.allowed(session.is_some()) {
            return Ok(None);
        }
        if session.is_none() {
            self.capabilities(cancel, started)?;
            *session = Some(Session::start(&self.runtime)?);
        }
        let result = session
            .as_mut()
            .expect("started session")
            .query(request, cancel, started);
        if result.is_err() {
            session.take();
        }
        result.map(Some)
    }

    pub(super) fn comparison_supported(&self, cancel: &AtomicBool) -> Result<bool> {
        Ok(self.capabilities(cancel, Instant::now())?.flow_comparison)
    }

    pub(super) fn observation_supported(&self, cancel: &AtomicBool) -> Result<bool> {
        // A monitor start/retry must notice an installed CLI upgraded in place.
        Ok(super::capabilities::check(&self.runtime, cancel, Instant::now())?.observation)
    }

    pub(super) fn automatic_refresh_supported(&self, cancel: &AtomicBool) -> Result<bool> {
        Ok(super::capabilities::check(&self.runtime, cancel, Instant::now())?.automatic_refresh)
    }

    fn capabilities(
        &self,
        cancel: &AtomicBool,
        started: Instant,
    ) -> Result<&super::capabilities::Capabilities> {
        if self.capabilities.get().is_none() {
            let capabilities = super::capabilities::check(&self.runtime, cancel, started)?;
            let _ = self.capabilities.set(capabilities);
        }
        Ok(self.capabilities.get().expect("validated CLI capabilities"))
    }

    pub(super) fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub(super) fn expansion_supported(&self, cancel: &AtomicBool) -> Result<bool> {
        Ok(self.capabilities(cancel, Instant::now())?.flow_expansion)
    }
    pub fn flow_expansion_supported(&self) -> bool {
        self.capabilities
            .get()
            .is_some_and(|capabilities| capabilities.flow_expansion)
    }
    pub fn flow_supported(&self, language: &str) -> bool {
        self.capabilities.get().is_some_and(|capabilities| {
            capabilities
                .flow_languages
                .iter()
                .any(|supported| supported == language)
        })
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

pub(super) fn nonblocking(input: &ChildStdin) -> Result<()> {
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

pub(super) fn write_request(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_guard_requires_connection_and_current_activity_revision() {
        let revision = AtomicU64::new(4);
        let release = Dispatch::Release(ReleaseGuard::new(&revision, 4));
        assert!(!release.allowed(false));
        assert!(release.allowed(true));
        revision.fetch_add(1, Ordering::AcqRel);
        assert!(!release.allowed(true));
        assert!(Dispatch::Required.allowed(false));
    }
}
