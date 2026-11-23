//! Read-only working and verified commit graphs from the local library.
mod changes;
mod history;
mod inspection;
mod repository;
use super::{Artifact, storage};
use anyhow::{Context, Result};
pub use changes::TrackedChanges;
pub use history::{CommitHistory, CommitPage, HistoryCommit};
pub use inspection::{
    ArtifactFailure, ArtifactResult, InspectedEndpoint, Inspection, ObservedSchema, inspect,
    inspect_cached, load_retained,
};
pub use repository::{Repository, TreeEntry};
use serde::Serialize;
#[cfg(test)]
use std::fs;
use std::{
    fmt,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

const GIT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointKind {
    Commit,
    Worktree,
}

#[derive(Clone, Debug, Serialize)]
pub struct Endpoint {
    pub kind: EndpointKind,
    pub requested_ref: String,
    pub commit: Option<String>,
    /// Absolute path to the captured manifest in the local graph library.
    pub path: String,
    pub schema: String,
    pub digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<super::local_snapshots::SnapshotRecord>,
}

pub struct Snapshot {
    pub artifact: Artifact,
    pub endpoint: Endpoint,
}

#[derive(Debug)]
struct Failure {
    code: &'static str,
    message: String,
}
impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}
impl std::error::Error for Failure {}

fn failure(code: &'static str, message: impl Into<String>) -> anyhow::Error {
    Failure {
        code,
        message: message.into(),
    }
    .into()
}

pub fn error_code(error: &anyhow::Error) -> &'static str {
    error
        .downcast_ref::<Failure>()
        .map_or("revision_unavailable", |failure| failure.code)
}

/// Reads a saved artifact exactly once. No checkout, analysis, fetch or cache writes.
pub fn load(project: &Path, revision: &str) -> Result<Snapshot> {
    inspect(project, revision)?.into_snapshot()
}

struct ReadSnapshot {
    bytes: Vec<u8>,
    decoded: Result<Artifact>,
    commit: Option<String>,
    kind: EndpointKind,
    path: String,
    snapshot: Option<super::local_snapshots::SnapshotRecord>,
}

fn read_worktree(project: &Path) -> Result<ReadSnapshot> {
    let (path, bytes, captured) = storage::Captured::inspect_located(project).map_err(|error| {
        failure(
            if error.downcast_ref::<storage::MissingGraph>().is_some() {
                "graph_missing"
            } else {
                "graph_unavailable"
            },
            format!("Cannot read saved working graph: {error:#}"),
        )
    })?;
    Ok(ReadSnapshot {
        bytes,
        decoded: captured.and_then(|captured| captured.complete()),
        commit: None,
        kind: EndpointKind::Worktree,
        path: path.display().to_string(),
        snapshot: None,
    })
}

fn resolve_commit(project: &Path, revision: &str) -> Result<String> {
    if invalid_revision(revision) {
        return Err(failure("invalid_request", "Invalid Git revision"));
    }
    let object = format!("{revision}^{{commit}}");
    let commit_bytes = git(
        project,
        &["rev-parse", "--verify", "--end-of-options", &object],
        "revision_unavailable",
    )?;
    let commit = String::from_utf8(commit_bytes)
        .context("Git commit identity is not UTF-8")?
        .trim()
        .to_owned();
    if !valid_commit(&commit) {
        return Err(failure(
            "revision_unavailable",
            "Git returned an invalid commit identity",
        ));
    }
    Ok(commit)
}

fn invalid_revision(revision: &str) -> bool {
    revision.is_empty() || revision.starts_with('-') || revision.contains([':', '\0', '\n', '\r'])
}

fn valid_commit(commit: &str) -> bool {
    matches!(commit.len(), 40 | 64) && commit.bytes().all(|b| b.is_ascii_hexdigit())
}

fn project_prefix(project: &Path) -> Result<String> {
    let prefix = git(
        project,
        &["rev-parse", "--show-prefix"],
        "revision_unavailable",
    )?;
    let prefix = String::from_utf8(prefix).context("Git project prefix is not UTF-8")?;
    Ok(prefix.strip_suffix('\n').unwrap_or(&prefix).to_owned())
}

fn not_repository(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<Failure>()
        .is_some_and(|error| error.message.contains("not a git repository"))
}

/// Bind Git to an explicit project even when invoked from a Git hook or a caller
/// that has selected another repository through its environment.
pub fn git_for_project(project: &Path) -> Command {
    let mut command = Command::new("git");
    for variable in [
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_ATTR_SOURCE",
        "GIT_CEILING_DIRECTORIES",
        "GIT_COMMON_DIR",
        "GIT_CONFIG",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_GLOBAL",
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_SYSTEM",
        "GIT_DIR",
        "GIT_DISCOVERY_ACROSS_FILESYSTEM",
        "GIT_GRAFT_FILE",
        "GIT_IMPLICIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_NAMESPACE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_PREFIX",
        "GIT_QUARANTINE_PATH",
        "GIT_REPLACE_REF_BASE",
        "GIT_SHALLOW_FILE",
        "GIT_WORK_TREE",
    ] {
        command.env_remove(variable);
    }
    command
        .arg("--no-pager")
        .arg("-C")
        .arg(project)
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C");
    command
}

/// Git writes into anonymous temporary files instead of pipes, avoiding deadlocks
/// on large trees and input batches. It inherits the CLI process group.
fn git(project: &Path, arguments: &[&str], code: &'static str) -> Result<Vec<u8>> {
    let mut output = git_output(project, arguments, code, GitInput::default())?;
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[derive(Default)]
struct GitInput<'a> {
    bytes: &'a [u8],
    cancel: Option<&'a AtomicBool>,
    output_limit: Option<u64>,
}

fn git_output(
    project: &Path,
    arguments: &[&str],
    code: &'static str,
    input: GitInput<'_>,
) -> Result<std::fs::File> {
    let mut output = tempfile::tempfile().context("Cannot capture Git output")?;
    let mut errors = tempfile::tempfile().context("Cannot capture Git diagnostics")?;
    let stdin = if input.bytes.is_empty() {
        Stdio::null()
    } else {
        let mut file = tempfile::tempfile().context("Cannot prepare Git input")?;
        file.write_all(input.bytes)?;
        file.rewind()?;
        file.into()
    };
    let child = git_for_project(project)
        .args(arguments)
        .stdin(stdin)
        .stdout(output.try_clone()?)
        .stderr(errors.try_clone()?)
        .spawn()
        .context("Cannot start Git")?;
    let mut child = OwnedChild(child);
    let start = Instant::now();
    let status = loop {
        if input
            .cancel
            .is_some_and(|flag| flag.load(Ordering::Acquire))
        {
            return Err(failure("cancelled", "Git snapshot read cancelled"));
        }
        if output_limit_reached(&output, input.output_limit)? {
            drop(child);
            output.rewind()?;
            return Ok(output);
        }
        if let Some(status) = child.0.try_wait().context("Cannot observe Git")? {
            break status;
        }
        if start.elapsed() >= GIT_TIMEOUT {
            return Err(failure(
                "git_timeout",
                "Git snapshot read exceeded 30 seconds",
            ));
        }
        thread::sleep(Duration::from_millis(10));
    };
    if !status.success() {
        errors.seek(SeekFrom::Start(0))?;
        let mut diagnostic = String::new();
        errors.take(64 * 1024).read_to_string(&mut diagnostic)?;
        return Err(failure(
            code,
            format!("Git {} failed: {}", arguments[0], diagnostic.trim()),
        ));
    }
    output.seek(SeekFrom::Start(0))?;
    Ok(output)
}

fn output_limit_reached(output: &std::fs::File, limit: Option<u64>) -> Result<bool> {
    match limit {
        Some(limit) => Ok(output.metadata()?.len() >= limit),
        None => Ok(false),
    }
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(test)]
mod tests;
