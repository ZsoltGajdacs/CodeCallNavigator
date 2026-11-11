//! Owned, cancellable two-phase adapter process. Protocol payloads stay adapter-specific.
use super::{
    diagnostics::{PROCESS_TIMEOUT, ProcessLogs, log_tail},
    stop_process_group,
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs,
    io::Write,
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
    sync::atomic::AtomicBool,
    thread,
    time::{Duration, Instant},
};

pub(in crate::analyzers) struct Worker {
    child: Child,
    logs: ProcessLogs,
    timeout: Duration,
    finished: bool,
}

impl Worker {
    pub(in crate::analyzers) fn start(
        command: &mut Command,
        job: &Path,
        name: &str,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        let logs = ProcessLogs::new(job, name);
        logs.check_wait(cancel, Instant::now(), PROCESS_TIMEOUT)?;
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::from(fs::File::create(&logs.stdout)?))
            .stderr(Stdio::from(fs::File::create(&logs.stderr)?))
            .process_group(0)
            .spawn()
            .context("Cannot start analysis worker")?;
        crate::metrics::count("worker_processes", 1);
        crate::metrics::count("subprocesses", 1);
        Ok(Self {
            child,
            logs,
            timeout: PROCESS_TIMEOUT,
            finished: false,
        })
    }

    pub(in crate::analyzers) fn wait_for_output(
        &mut self,
        output: &Path,
        cancel: &AtomicBool,
    ) -> Result<()> {
        let started = Instant::now();
        loop {
            self.check_wait(cancel, started)?;
            if output.is_file() {
                return Ok(());
            }
            if let Some(status) = self.child.try_wait()? {
                bail!(
                    "Analysis worker exited before output ({status}): {}",
                    log_tail(&self.logs.stderr)
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub(in crate::analyzers) fn request(&mut self, request: &str) -> Result<()> {
        let input = self
            .child
            .stdin
            .as_mut()
            .context("Analysis worker input is closed")?;
        writeln!(input, "{request}")?;
        input.flush()?;
        Ok(())
    }

    pub(in crate::analyzers) fn finish(&mut self, cancel: &AtomicBool) -> Result<()> {
        self.child.stdin.take();
        let started = Instant::now();
        loop {
            self.check_wait(cancel, started)?;
            if let Some(status) = self.child.try_wait()? {
                self.finished = true;
                ensure!(
                    status.success(),
                    "Analysis worker failed ({status}): {}",
                    log_tail(&self.logs.stderr)
                );
                return Ok(());
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn check_wait(&mut self, cancel: &AtomicBool, started: Instant) -> Result<()> {
        self.logs.check_wait(cancel, started, self.timeout)
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if !self.finished {
            stop_process_group(&mut self.child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn abandoned_worker_is_terminated_and_reaped() {
        let directory = tempfile::tempdir().unwrap();
        let mut worker = Worker::start(
            Command::new("/bin/sh").args(["-c", "read request; sleep 30"]),
            directory.path(),
            "abandoned",
            &AtomicBool::new(false),
        )
        .unwrap();
        let pid = worker.child.id();
        worker.request("waiting").unwrap();
        drop(worker);
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    }
    #[test]
    fn cancelled_wait_stops_worker_on_drop() {
        let directory = tempfile::tempdir().unwrap();
        let mut worker = Worker::start(
            Command::new("/bin/sh").args(["-c", "sleep 30"]),
            directory.path(),
            "cancelled-worker",
            &AtomicBool::new(false),
        )
        .unwrap();
        let pid = worker.child.id();
        let error = worker
            .wait_for_output(&directory.path().join("absent"), &AtomicBool::new(true))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("cancelled-worker cancelled after"),
            "{error}"
        );
        assert!(!error.contains("timed out"), "{error}");
        assert!(error.contains("cancelled-worker.stdout"), "{error}");
        assert!(error.contains("cancelled-worker.stderr"), "{error}");
        drop(worker);
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    }

    #[test]
    fn timeout_wait_reports_context_and_worker_is_reaped_on_drop() {
        let directory = tempfile::tempdir().unwrap();
        let mut worker = Worker::start(
            Command::new("/bin/sh").args(["-c", "echo worker-diagnostic >&2; sleep 30"]),
            directory.path(),
            "deadline-worker",
            &AtomicBool::new(false),
        )
        .unwrap();
        worker.timeout = Duration::from_millis(200);
        let pid = worker.child.id();
        let error = worker
            .wait_for_output(&directory.path().join("absent"), &AtomicBool::new(false))
            .unwrap_err()
            .to_string();
        assert!(error.contains("deadline-worker timed out after"), "{error}");
        assert!(error.contains("deadline-worker.stdout"), "{error}");
        assert!(error.contains("deadline-worker.stderr"), "{error}");
        assert!(error.contains("worker-diagnostic"), "{error}");
        drop(worker);
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    }

    #[test]
    fn timeout_finish_stops_worker_that_does_not_exit_after_eof() {
        let directory = tempfile::tempdir().unwrap();
        let mut worker = Worker::start(
            Command::new("/bin/sh").args(["-c", "cat >/dev/null; sleep 30"]),
            directory.path(),
            "finish-worker",
            &AtomicBool::new(false),
        )
        .unwrap();
        worker.timeout = Duration::from_millis(20);
        let pid = worker.child.id();
        let error = worker
            .finish(&AtomicBool::new(false))
            .unwrap_err()
            .to_string();
        assert!(error.contains("finish-worker timed out after"), "{error}");
        drop(worker);
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    }
}
