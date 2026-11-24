//! Fallible cleanup for analysis-owned adapter processes.
use anyhow::Result;
use codecallnavigator_core::process_ownership::{Descendants, reap_eventually};
use std::{
    os::unix::process::CommandExt,
    process::{Child, Command, ExitStatus},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct CleanupFailure(String);

impl std::fmt::Display for CleanupFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Cannot confirm analyzer process cleanup: {}",
            self.0
        )
    }
}

impl std::error::Error for CleanupFailure {}

/// Owns one explicitly spawned process group and its observed descendants.
/// Call complete to report cleanup; Drop is a best-effort fallback only.
pub struct OwnedProcess {
    child: Option<Child>,
    descendants: Descendants,
    next_observation: Instant,
}

impl OwnedProcess {
    pub fn spawn(command: &mut Command) -> Result<Self> {
        let mut child = command.process_group(0).spawn()?;
        let descendants = match Descendants::for_group(&child) {
            Ok(descendants) => descendants,
            Err(error) => {
                // The direct child has not been reaped, so its freshly assigned
                // process group cannot identify an unrelated process group.
                unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
                let _ = child.kill();
                reap_eventually(child);
                return Err(
                    CleanupFailure(format!("could not track spawned process: {error:#}")).into(),
                );
            }
        };
        Ok(Self {
            child: Some(child),
            descendants,
            next_observation: Instant::now() + Duration::from_millis(100),
        })
    }

    pub fn child_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("analysis process is active")
    }

    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>> {
        if Instant::now() >= self.next_observation {
            let diagnostic = self.descendants.track().map_err(|error| {
                CleanupFailure(format!("could not observe descendants: {error:#}"))
            })?;
            self.next_observation = Instant::now() + Duration::from_millis(100);
            if let Some(error) = diagnostic {
                eprintln!("Analyzer process tracking temporarily unavailable: {error:#}");
            }
        }
        Ok(self.child_mut().try_wait()?)
    }

    pub fn complete<T>(&mut self, result: Result<T>) -> Result<T> {
        match (result, self.stop()) {
            (result, Ok(())) => result,
            (Ok(_), Err(cleanup)) => Err(CleanupFailure(format!("{cleanup:#}")).into()),
            (Err(operation), Err(cleanup)) => Err(CleanupFailure(format!(
                "{cleanup:#}; analyzer operation also reported: {operation:#}"
            ))
            .into()),
        }
    }

    fn stop(&mut self) -> Result<()> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        let result = self.descendants.stop(&mut child);
        if !matches!(child.try_wait(), Ok(Some(_))) {
            reap_eventually(child);
        }
        result
    }
}

impl Drop for OwnedProcess {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
