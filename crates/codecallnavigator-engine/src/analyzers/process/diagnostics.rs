//! Shared deadline classification and bounded subprocess log diagnostics.
use anyhow::{Result, bail};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub(super) const PROCESS_TIMEOUT: Duration = Duration::from_secs(1800);
const LOG_TAIL_BYTES: u64 = 4000;

/// Only a reaped native command's exit is eligible for adapter diagnostics.
/// Timeouts, cancellation, spawn and I/O failures retain their own classification.
#[derive(Debug)]
pub(crate) struct NativeExitFailure {
    status: ExitStatus,
    message: String,
}

impl NativeExitFailure {
    pub(super) fn require_success(logs: &ProcessLogs, status: ExitStatus) -> Result<()> {
        if status.success() {
            Ok(())
        } else {
            Err(Self::new(logs, status).into())
        }
    }

    fn new(logs: &ProcessLogs, status: ExitStatus) -> Self {
        Self {
            status,
            message: format!(
                "{} failed ({status}). {}",
                logs.name,
                log_tail(&logs.stderr)
            ),
        }
    }

    pub(crate) fn exited(&self) -> bool {
        self.status.code().is_some()
    }
}

impl std::fmt::Display for NativeExitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for NativeExitFailure {}

pub(super) struct ProcessLogs {
    pub(super) name: String,
    pub(super) stdout: PathBuf,
    pub(super) stderr: PathBuf,
}

impl ProcessLogs {
    pub(super) fn new(job: &Path, name: &str) -> Self {
        Self {
            name: name.into(),
            stdout: job.join(format!("{name}.stdout")),
            stderr: job.join(format!("{name}.stderr")),
        }
    }

    pub(super) fn check_wait(
        &self,
        cancel: &AtomicBool,
        started: Instant,
        timeout: Duration,
    ) -> Result<()> {
        let elapsed = started.elapsed();
        if cancel.load(Ordering::Relaxed) {
            bail!(
                "{} cancelled after {:.2}s; {}",
                self.name,
                elapsed.as_secs_f64(),
                self.paths()
            );
        }
        if elapsed >= timeout {
            bail!(
                "{} timed out after {:.2}s (limit {:.2}s); {}; stderr tail: {}",
                self.name,
                elapsed.as_secs_f64(),
                timeout.as_secs_f64(),
                self.paths(),
                log_tail(&self.stderr)
            );
        }
        Ok(())
    }

    fn paths(&self) -> String {
        format!(
            "stdout: {}; stderr: {}",
            self.stdout.display(),
            self.stderr.display()
        )
    }
}

pub(super) fn log_tail(path: &Path) -> String {
    read_log_tail(path).unwrap_or_else(|error| format!("Cannot read {}: {error}", path.display()))
}

fn read_log_tail(path: &Path) -> std::io::Result<String> {
    let file = File::open(path)?;
    let length = file.metadata()?.len();
    read_tail(file, length)
}

fn read_tail(mut input: impl Read + Seek, length: u64) -> std::io::Result<String> {
    input.seek(SeekFrom::Start(length.saturating_sub(LOG_TAIL_BYTES)))?;
    let mut bytes = Vec::new();
    input.take(LOG_TAIL_BYTES).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stderr_tail_is_bounded_and_tolerates_a_split_utf8_character() {
        let input = format!("{}END", "é".repeat(10000));
        let tail = read_tail(std::io::Cursor::new(input.as_bytes()), input.len() as u64).unwrap();
        assert!(tail.starts_with('\u{fffd}'));
        assert!(tail.ends_with("END"));
        assert!(tail.len() <= LOG_TAIL_BYTES as usize + 2);
    }

    #[test]
    fn stderr_tail_reads_real_logs_and_reports_unavailable_paths() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("process.stderr");
        std::fs::write(&path, "compiler error\n").unwrap();
        assert_eq!(log_tail(&path), "compiler error\n");
        std::fs::remove_file(&path).unwrap();
        assert!(log_tail(&path).starts_with(&format!("Cannot read {}: ", path.display())));
    }
}
