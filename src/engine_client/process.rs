//! Own the engine process, its pipes and cancellation for one request.
use crate::runtime::Runtime;
use anyhow::{Context, Result, bail, ensure};
use codegraph::protocol::{Event, Request, VERSION};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::process::CommandExt,
    process::{Child, ChildStdin, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

pub(super) struct Session {
    child: Child,
    input: Option<ChildStdin>,
    events: mpsc::Receiver<Result<Event>>,
    log: tempfile::NamedTempFile,
    cancellation: Option<Instant>,
    completion: Option<Instant>,
    reaped: bool,
}

impl Session {
    pub fn start(runtime: &Runtime, request: &Request) -> Result<Self> {
        let log = tempfile::NamedTempFile::new()?;
        let mut child = Command::new(&runtime.engine)
            .arg("serve-once")
            .env("CODEGRAPH_RESOURCES", &runtime.resources)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log.reopen()?))
            .process_group(0)
            .spawn()
            .with_context(|| format!("Cannot start {}", runtime.engine.display()))?;
        let input = child.stdin.take().context("Engine stdin is unavailable")?;
        let output = child
            .stdout
            .take()
            .context("Engine stdout is unavailable")?;
        let (sender, events) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let parsed = line.map_err(anyhow::Error::from).and_then(|line| {
                    serde_json::from_str(&line).context("Invalid engine protocol event")
                });
                if sender.send(parsed).is_err() {
                    break;
                }
            }
        });
        let mut session = Self {
            child,
            input: Some(input),
            events,
            log,
            cancellation: None,
            completion: None,
            reaped: false,
        };
        serde_json::to_writer(session.input.as_mut().unwrap(), request)?;
        writeln!(session.input.as_mut().unwrap())?;
        session.input.as_mut().unwrap().flush()?;
        Ok(session)
    }

    pub fn next(&mut self, cancel: &AtomicBool) -> Result<Option<Event>> {
        loop {
            self.observe_exit()?;
            if cancel.load(Ordering::Relaxed) {
                self.request_cancel();
            }
            if self
                .cancellation
                .is_some_and(|started| started.elapsed() > Duration::from_secs(5))
            {
                self.stop()?;
                bail!("Analysis cancelled; engine did not stop within five seconds");
            }
            if self
                .completion
                .is_some_and(|started| started.elapsed() > Duration::from_secs(5))
            {
                self.stop()
                    .context("Engine result stream remained open after completion")?;
                bail!("Engine result stream remained open after completion");
            }
            match self.events.recv_timeout(Duration::from_millis(50)) {
                Ok(event) => return event.map(Some),
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(None),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }

    pub fn finish(&mut self) -> Result<ExitStatus> {
        let started = self
            .cancellation
            .into_iter()
            .chain(self.completion)
            .min()
            .unwrap_or_else(Instant::now);
        loop {
            if let Some(status) = self.child.try_wait()? {
                self.finish_group();
                return Ok(status);
            }
            if started.elapsed() > Duration::from_secs(5) {
                self.stop()?;
                bail!("Engine did not exit after closing its result stream");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn expect_exit(&mut self) {
        self.completion.get_or_insert_with(Instant::now);
    }

    fn observe_exit(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_some() {
            self.expect_exit();
        }
        Ok(())
    }

    fn finish_group(&mut self) {
        // The leader is reaped, but its group may still own pipes or build work.
        // Signal the group only: the leader's numeric PID can already be reused.
        signal_group(self.child.id(), libc::SIGTERM);
        signal_group(self.child.id(), libc::SIGKILL);
        self.reaped = true;
    }

    pub fn log_tail(&self) -> String {
        let text = std::fs::read_to_string(self.log.path()).unwrap_or_default();
        text.chars()
            .rev()
            .take(4_000)
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    }

    fn request_cancel(&mut self) {
        if self.cancellation.is_some() {
            return;
        }
        self.cancellation = Some(Instant::now());
        if let Some(mut input) = self.input.take() {
            let _ = writeln!(
                input,
                "{}",
                serde_json::json!({"command":"cancel", "version":VERSION})
            );
            let _ = input.flush();
        }
    }

    fn stop(&mut self) -> Result<()> {
        if self.reaped {
            return Ok(());
        }
        if self.child.try_wait().ok().flatten().is_some() {
            self.finish_group();
            return Ok(());
        }
        self.request_cancel();
        let descendants = descendants(self.child.id());
        let selected = descendants.as_deref().unwrap_or_default();
        signal(self.child.id(), libc::SIGTERM);
        for &pid in selected {
            signal(pid, libc::SIGTERM);
        }
        let started = Instant::now();
        let mut exited = false;
        while started.elapsed() < Duration::from_secs(1) {
            if self.child.try_wait().ok().flatten().is_some() {
                exited = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        for &pid in selected {
            signal(pid, libc::SIGKILL);
        }
        if !exited {
            signal(self.child.id(), libc::SIGKILL);
        }
        let _ = self.child.wait();
        self.reaped = true;
        descendants
            .context("Engine stopped, but detached descendant cleanup could not be verified")?;
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn signal(pid: u32, signal: i32) {
    // Engine and adapters each own process groups; signal both group and process.
    signal_group(pid, signal);
    unsafe {
        libc::kill(pid as i32, signal);
    }
}

fn signal_group(pid: u32, signal: i32) {
    unsafe {
        libc::kill(-(pid as i32), signal);
    }
}

fn descendants(parent: u32) -> Result<Vec<u32>> {
    let output = Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid="])
        .output()
        .context("Cannot enumerate engine descendants")?;
    ensure!(
        output.status.success(),
        "Cannot enumerate engine descendants: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let pairs: Vec<_> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((
                fields.next()?.parse::<u32>().ok()?,
                fields.next()?.parse::<u32>().ok()?,
            ))
        })
        .collect();
    let mut selected = vec![parent];
    let mut index = 0;
    while index < selected.len() {
        let parent = selected[index];
        selected.extend(
            pairs
                .iter()
                .filter(|(_, owner)| *owner == parent)
                .map(|(pid, _)| *pid),
        );
        index += 1;
    }
    selected.remove(0);
    Ok(selected)
}
