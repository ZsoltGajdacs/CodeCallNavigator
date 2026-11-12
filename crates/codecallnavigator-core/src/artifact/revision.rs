//! Read-only saved graph snapshots from immutable Git blobs or the working tree.
use super::{Artifact, GRAPH_PATH};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    fmt, fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
    process::{Child, Command, Stdio},
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
    /// Repository-relative blob path; project-relative outside a Git repository.
    pub path: String,
    pub schema: String,
    pub digest: String,
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
    let (bytes, commit, kind, path) = if revision == "WORKTREE" {
        let bytes = fs::read(project.join(GRAPH_PATH)).map_err(|error| {
            failure(
                if error.kind() == std::io::ErrorKind::NotFound {
                    "graph_missing"
                } else {
                    "graph_unavailable"
                },
                format!("Cannot read saved graph: {error}"),
            )
        })?;
        let path = working_path(project_prefix(project))?;
        (bytes, None, EndpointKind::Worktree, path)
    } else {
        let (bytes, commit, path) = committed(project, revision)?;
        (bytes, Some(commit), EndpointKind::Commit, path)
    };
    let artifact = Artifact::from_bytes(&bytes)
        .map_err(|error| failure("invalid_graph", format!("{error:#}")))?;
    let endpoint = Endpoint {
        kind,
        requested_ref: revision.into(),
        commit,
        path,
        schema: artifact.schema.clone(),
        digest: blake3::hash(&bytes).to_hex().to_string(),
    };
    Ok(Snapshot { artifact, endpoint })
}

fn committed(project: &Path, revision: &str) -> Result<(Vec<u8>, String, String)> {
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
    let path = format!("{}{GRAPH_PATH}", project_prefix(project)?);
    let object = format!("{commit}:{path}");
    let bytes = git(project, &["cat-file", "blob", &object], "graph_missing")?;
    Ok((bytes, commit, path))
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

fn working_path(prefix: Result<String>) -> Result<String> {
    match prefix {
        Ok(prefix) => Ok(format!("{prefix}{GRAPH_PATH}")),
        Err(error) if not_repository(&error) => Ok(GRAPH_PATH.into()),
        Err(error) => Err(error),
    }
}

fn not_repository(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<Failure>()
        .is_some_and(|error| error.message.contains("not a git repository"))
}

/// Git writes into anonymous temporary files instead of pipes, avoiding deadlocks
/// on large graphs. It inherits the CLI process group for cancellation ownership.
fn git(project: &Path, arguments: &[&str], code: &'static str) -> Result<Vec<u8>> {
    let mut output = tempfile::tempfile().context("Cannot capture Git output")?;
    let mut errors = tempfile::tempfile().context("Cannot capture Git diagnostics")?;
    let child = Command::new("git")
        .arg("--no-pager")
        .arg("-C")
        .arg(project)
        .args(arguments)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(errors.try_clone()?)
        .spawn()
        .context("Cannot start Git")?;
    let mut child = OwnedChild(child);
    let start = Instant::now();
    let status = loop {
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
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes)?;
    Ok(bytes)
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
