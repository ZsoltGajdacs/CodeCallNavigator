//! Owned detached checkouts used by explicit snapshot preparation.
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator_core::artifact::revision::{Repository, git_for_project};
use codecallnavigator_engine::process_ownership::{CleanupFailure, OwnedProcess};
use std::{
    ffi::OsStr,
    io::{Read, Seek, SeekFrom},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

const READ_TIMEOUT: Duration = Duration::from_secs(30);
const CHECKOUT_TIMEOUT: Duration = Duration::from_secs(600);
const REMOVE_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_GIT_OUTPUT: u64 = 4 * 1024 * 1024;

pub(super) struct ResolvedRevision {
    pub commit: String,
    repository: PathBuf,
    project_path: PathBuf,
}

impl ResolvedRevision {
    pub fn resolve(project: &Path, revision: &str, cancel: &AtomicBool) -> Result<Self> {
        ensure!(
            revision != "WORKTREE" && revision.len() <= 16384,
            "Snapshot preparation requires a commit or ref of at most 16384 bytes"
        );
        require_running(cancel)?;
        let project = project
            .canonicalize()
            .context("Cannot open snapshot project")?;
        let repository = Repository::open(&project)?;
        let project_path = project.strip_prefix(&repository.root)?.to_path_buf();
        require_running(cancel)?;
        let commit = repository.resolve(revision)?;
        require_running(cancel)?;
        Ok(Self {
            commit,
            repository: repository.root,
            project_path,
        })
    }

    pub fn with_checkout<T>(
        &self,
        cancel: &AtomicBool,
        operation: impl FnOnce(&Path, &Path) -> Result<T>,
    ) -> Result<T> {
        require_running(cancel)?;
        let temporary = tempfile::Builder::new()
            .prefix("ccnavigator-snapshot-")
            .tempdir()
            .context("Cannot create a snapshot preparation directory")?;
        let path = temporary.path().canonicalize()?.join("checkout");
        // Only explicit cleanup may remove this directory. Preserve it if Git or
        // the analyzer cannot prove that owned processes have stopped.
        let temporary = temporary.keep();
        let result = self.run_checkout(&path, cancel, operation);
        if result
            .as_ref()
            .is_err_and(|error| error.is::<CleanupFailure>())
        {
            return result.with_context(|| {
                format!(
                    "Snapshot preparation directory retained at {}",
                    temporary.display()
                )
            });
        }
        finish(result, self.clean_checkout(&path, &temporary))
    }

    fn run_checkout<T>(
        &self,
        path: &Path,
        cancel: &AtomicBool,
        operation: impl FnOnce(&Path, &Path) -> Result<T>,
    ) -> Result<T> {
        git(
            &self.repository,
            &[
                "worktree".as_ref(),
                "add".as_ref(),
                "--detach".as_ref(),
                path.as_os_str(),
                self.commit.as_ref(),
            ],
            cancel,
            CHECKOUT_TIMEOUT,
        )?;
        let head = git(
            path,
            &["rev-parse".as_ref(), "--verify".as_ref(), "HEAD".as_ref()],
            cancel,
            READ_TIMEOUT,
        )?;
        ensure!(
            head.strip_suffix(b"\n") == Some(self.commit.as_bytes()),
            "Temporary worktree HEAD differs from the requested snapshot revision"
        );
        let expected_project = path.join(&self.project_path);
        let project = expected_project
            .canonicalize()
            .context("Project path is absent in the requested revision")?;
        ensure!(
            project == expected_project && project.is_dir(),
            "Project path in the requested revision must be a directory at the same repository path"
        );
        git(
            path,
            &["clean".as_ref(), "-ffdx".as_ref()],
            cancel,
            CHECKOUT_TIMEOUT,
        )
        .context("Cannot clear untracked files from the temporary snapshot checkout")?;
        require_running(cancel)?;
        operation(path, &project)
    }

    fn clean_checkout(&self, path: &Path, temporary: &Path) -> Result<()> {
        // Cancellation has already stopped and reaped the build process. Finish
        // the cleanup even if the caller has cancelled its request.
        let cleanup = AtomicBool::new(false);
        let result: Result<()> = (|| {
            if self.registered(path, &cleanup)? {
                git(
                    &self.repository,
                    &[
                        "worktree".as_ref(),
                        "remove".as_ref(),
                        "--force".as_ref(),
                        path.as_os_str(),
                    ],
                    &cleanup,
                    REMOVE_TIMEOUT,
                )?;
                ensure!(
                    !self.registered(path, &cleanup)?,
                    "Git still lists the temporary worktree"
                );
            }
            std::fs::remove_dir_all(temporary)?;
            Ok(())
        })();
        result.with_context(|| {
            format!(
                "Cannot clean snapshot preparation directory {}",
                temporary.display()
            )
        })
    }

    fn registered(&self, path: &Path, cancel: &AtomicBool) -> Result<bool> {
        let entries = git(
            &self.repository,
            &[
                "worktree".as_ref(),
                "list".as_ref(),
                "--porcelain".as_ref(),
                "-z".as_ref(),
            ],
            cancel,
            READ_TIMEOUT,
        )?;
        Ok(entries
            .split(|byte| *byte == 0)
            .any(|field| field.strip_prefix(b"worktree ") == Some(path.as_os_str().as_bytes())))
    }
}

fn require_running(cancel: &AtomicBool) -> Result<()> {
    ensure!(
        !cancel.load(Ordering::Acquire),
        "Snapshot preparation was cancelled"
    );
    Ok(())
}

fn finish<T>(result: Result<T>, cleanup: Result<()>) -> Result<T> {
    match (result, cleanup) {
        (result, Ok(())) => result,
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(operation), Err(cleanup)) => {
            Err(cleanup.context(format!("Snapshot preparation also failed: {operation:#}")))
        }
    }
}

fn git(
    directory: &Path,
    arguments: &[&OsStr],
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<Vec<u8>> {
    require_running(cancel)?;
    let mut stdout = tempfile::tempfile().context("Cannot capture Git output")?;
    let mut stderr = tempfile::tempfile().context("Cannot capture Git diagnostics")?;
    let mut child = OwnedProcess::spawn(
        git_for_project(directory)
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "submodule.recurse=false",
            ])
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(stdout.try_clone()?)
            .stderr(stderr.try_clone()?),
    )
    .context("Cannot start Git for snapshot preparation")?;
    let result = (|| {
        let start = Instant::now();
        loop {
            require_running(cancel)?;
            if let Some(status) = child.try_wait().context("Cannot observe Git")? {
                if !status.success() {
                    let output = read_output(&mut stderr, 64 * 1024)?;
                    bail!(
                        "Git {} failed: {}",
                        arguments[0].to_string_lossy(),
                        String::from_utf8_lossy(&output).trim()
                    );
                }
                return read_output(&mut stdout, MAX_GIT_OUTPUT);
            }
            ensure!(
                start.elapsed() < timeout,
                "Git snapshot preparation timed out"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    })();
    child.complete(result)
}

fn read_output(file: &mut std::fs::File, limit: u64) -> Result<Vec<u8>> {
    file.seek(SeekFrom::Start(0))?;
    let mut output = Vec::new();
    file.take(limit + 1).read_to_end(&mut output)?;
    ensure!(
        output.len() as u64 <= limit,
        "Git snapshot output exceeded its limit"
    );
    Ok(output)
}
