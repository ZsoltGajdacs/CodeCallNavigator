mod diagnostics;
pub(super) mod worker;
use anyhow::{Context, Result, ensure};
use diagnostics::{PROCESS_TIMEOUT, ProcessLogs, log_tail};
use std::os::unix::process::CommandExt;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

pub(super) fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    ensure!(!cancel.load(Ordering::Relaxed), "Analysis cancelled");
    Ok(())
}

pub(super) fn run_process(
    command: &mut Command,
    job: &Path,
    name: &str,
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    run_process_with_timeout(
        command,
        ProcessLogs::new(job, name),
        cancel,
        PROCESS_TIMEOUT,
    )
}

fn run_process_with_timeout(
    command: &mut Command,
    logs: ProcessLogs,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<PathBuf> {
    logs.check_wait(cancel, Instant::now(), timeout)?;
    let _timing = crate::metrics::stage("subprocess");
    crate::metrics::count("subprocesses", 1);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(fs::File::create(&logs.stdout)?))
        .stderr(Stdio::from(fs::File::create(&logs.stderr)?))
        .process_group(0);
    let mut child = command
        .spawn()
        .with_context(|| format!("Cannot start {}", command.get_program().to_string_lossy()))?;
    let started = Instant::now();
    loop {
        if let Err(error) = logs.check_wait(cancel, started, timeout) {
            stop_process_group(&mut child);
            return Err(error);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                ensure!(
                    status.success(),
                    "{} failed ({status}). {}",
                    logs.name,
                    log_tail(&logs.stderr)
                );
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(error) => {
                stop_process_group(&mut child);
                return Err(error.into());
            }
        }
    }
    Ok(logs.stdout)
}

fn stop_process_group(child: &mut std::process::Child) {
    let group = -(child.id() as i32);
    // Each child was created in its own process group above; never signal our UI's group.
    unsafe {
        libc::kill(group, libc::SIGTERM);
    }
    for _ in 0..20 {
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    unsafe {
        libc::kill(group, libc::SIGKILL);
    }
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn analyzer_children_receive_eof_instead_of_the_controlling_terminal() {
        if let Some(directory) = std::env::var_os("CODEGRAPH_TEST_PROCESS_TTY") {
            assert_noninteractive_child(Path::new(&directory));
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let status = Command::new("python3")
            .arg(crate::repository_root().join("tests/process_tty.py"))
            .arg(std::env::current_exe().unwrap())
            .arg("analyzers::process::tests::analyzer_children_receive_eof_instead_of_the_controlling_terminal")
            .arg(directory.path())
            .status()
            .unwrap();
        assert!(
            status.success(),
            "Analyzer child inherited the controlling terminal"
        );
    }

    fn assert_noninteractive_child(directory: &Path) {
        assert_eq!(
            unsafe { libc::isatty(0) },
            1,
            "Helper must have a controlling terminal"
        );
        let marker = directory.join("stdin-child.pid");
        let output = run_process(
            Command::new("python3")
                .args(["-c", "import os,sys; from pathlib import Path; Path(sys.argv[1]).write_text(str(os.getpid())); data=os.read(0,1); assert data==b''; assert not os.isatty(0); print('stdin EOF')"])
                .arg(marker),
            directory,
            "stdin-child",
            &AtomicBool::new(false),
        ).unwrap();
        assert_eq!(fs::read_to_string(output).unwrap().trim(), "stdin EOF");
    }

    #[test]
    fn already_cancelled_process_never_starts() {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join("started");
        let result = run_process(
            Command::new("/usr/bin/touch").arg(&marker),
            directory.path(),
            "cancelled",
            &AtomicBool::new(true),
        );
        let error = result.unwrap_err().to_string();
        assert!(error.contains("cancelled cancelled after"), "{error}");
        assert!(error.contains("cancelled.stdout"), "{error}");
        assert!(error.contains("cancelled.stderr"), "{error}");
        assert!(!marker.exists());
    }

    #[test]
    fn timeout_reports_bounded_stderr_and_stops_descendants() {
        let directory = tempfile::tempdir().unwrap();
        let late_write = directory.path().join("should-not-be-created");
        let error = run_process_with_timeout(
            Command::new("/bin/sh")
                .args([
                    "-c",
                    "echo diagnostic-from-child >&2; (sleep 1; touch \"$1\") & wait",
                    "codegraph-timeout-test",
                ])
                .arg(&late_write),
            ProcessLogs::new(directory.path(), "deadline-child"),
            &AtomicBool::new(false),
            Duration::from_millis(200),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("deadline-child timed out after"), "{error}");
        assert!(error.contains("limit 0.20s"), "{error}");
        assert!(error.contains("deadline-child.stdout"), "{error}");
        assert!(error.contains("deadline-child.stderr"), "{error}");
        assert!(error.contains("diagnostic-from-child"), "{error}");
        assert!(!error.contains("cancelled"), "{error}");
        thread::sleep(Duration::from_millis(1200));
        assert!(
            !late_write.exists(),
            "A timed-out descendant continued executing"
        );
    }

    #[test]
    fn zero_deadline_never_starts_process_and_cancellation_takes_precedence() {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join("started");
        for (cancelled, expected) in [(false, "timed out"), (true, "cancelled")] {
            let error = run_process_with_timeout(
                Command::new("/usr/bin/touch").arg(&marker),
                ProcessLogs::new(directory.path(), "zero-deadline"),
                &AtomicBool::new(cancelled),
                Duration::ZERO,
            )
            .unwrap_err()
            .to_string();
            assert!(
                error.contains(&format!("zero-deadline {expected} after")),
                "{error}"
            );
            assert!(!marker.exists());
        }
    }

    #[test]
    fn cancellation_stops_descendants_in_the_owned_process_group() {
        let directory = tempfile::tempdir().unwrap();
        let started = directory.path().join("started");
        let late_write = directory.path().join("should-not-be-created");
        let cancel = Arc::new(AtomicBool::new(false));
        let trigger = cancel.clone();
        let marker = started.clone();
        let cancellation = thread::spawn(move || {
            for _ in 0..100 {
                if marker.exists() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            trigger.store(true, Ordering::Relaxed);
        });
        let result = run_process(
            Command::new("/bin/sh")
                .args([
                    "-c",
                    "touch \"$1\"; (sleep 1; touch \"$2\") & wait",
                    "codegraph-cancellation-test",
                ])
                .arg(&started)
                .arg(&late_write),
            directory.path(),
            "cancel-group",
            &cancel,
        );
        cancellation.join().unwrap();
        let error = result.unwrap_err().to_string();
        assert!(error.contains("cancel-group cancelled after"), "{error}");
        assert!(!error.contains("exceeded"), "{error}");
        assert!(error.contains("cancel-group.stdout"), "{error}");
        assert!(error.contains("cancel-group.stderr"), "{error}");
        thread::sleep(Duration::from_millis(1200));
        assert!(
            !late_write.exists(),
            "A cancelled descendant continued executing"
        );
    }
}
