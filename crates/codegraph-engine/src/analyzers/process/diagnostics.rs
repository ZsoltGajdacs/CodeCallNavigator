//! Shared deadline classification and bounded subprocess log diagnostics.
use anyhow::{Result, bail};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub(super) const PROCESS_TIMEOUT: Duration = Duration::from_secs(1800);
const LOG_TAIL_BYTES: u64 = 4000;

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
        if elapsed > timeout {
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
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(LOG_TAIL_BYTES)))?;
    let mut bytes = Vec::new();
    file.take(LOG_TAIL_BYTES).read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stderr_tail_is_bounded_and_tolerates_a_split_utf8_character() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("large.stderr");
        std::fs::write(&path, format!("{}END", "é".repeat(10000))).unwrap();
        let tail = log_tail(&path);
        assert!(tail.starts_with('\u{fffd}'));
        assert!(tail.ends_with("END"));
        assert!(tail.len() <= LOG_TAIL_BYTES as usize + 2);
    }
}
