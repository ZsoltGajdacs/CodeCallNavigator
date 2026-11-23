//! Read commit history from the checkout's HEAD captured for a Compare visit.

use super::{GitInput, git_output, valid_commit};
use anyhow::{Context, Result, ensure};
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

const PAGE_SIZE: usize = 100;
const OUTPUT_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct CommitHistory {
    project: PathBuf,
    head: Option<String>,
}

#[derive(Debug)]
pub struct CommitPage {
    pub rows: Vec<HistoryCommit>,
    pub next_offset: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct HistoryCommit {
    pub id: String,
    pub subject: String,
}

impl CommitHistory {
    /// Capture this checkout's HEAD so pagination is stable if the ref moves.
    pub fn open(project: &Path, cancel: &AtomicBool) -> Result<Self> {
        let project = project.canonicalize().context("Cannot open Git project")?;
        // Batch-check reports an unborn HEAD as missing without masking Git failures.
        let resolved = read_git(
            &project,
            &["cat-file", "--batch-check=%(objectname) %(objecttype)"],
            b"HEAD^{commit}\n",
            cancel,
        )?;
        let head = parse_head(std::str::from_utf8(&resolved)?.trim_end())?;
        Ok(Self { project, head })
    }

    pub fn project(&self) -> &Path {
        &self.project
    }

    pub fn head(&self) -> Option<&str> {
        self.head.as_deref()
    }

    pub fn page(&self, offset: usize, cancel: &AtomicBool) -> Result<CommitPage> {
        let Some(head) = self.head() else {
            return Ok(CommitPage {
                rows: Vec::new(),
                next_offset: None,
            });
        };
        let skip = format!("--skip={offset}");
        let limit = format!("--max-count={}", PAGE_SIZE + 1);
        let output = read_git(
            &self.project,
            &[
                "log",
                "--no-notes",
                "--no-show-signature",
                "--no-color",
                "--encoding=UTF-8",
                "--date-order",
                "--format=%H%x00%s",
                "-z",
                &skip,
                &limit,
                head,
                "--",
            ],
            &[],
            cancel,
        )?;
        parse_page(&output, offset)
    }

    /// Read the complete message only for the chosen commit.
    pub fn message(&self, commit: &str, cancel: &AtomicBool) -> Result<String> {
        ensure!(valid_commit(commit), "Invalid Git commit ID");
        let commit = format!("{commit}^{{commit}}");
        let output = read_git(
            &self.project,
            &[
                "log",
                "--no-notes",
                "--no-show-signature",
                "--no-color",
                "--encoding=UTF-8",
                "--max-count=1",
                "--format=format:%B",
                &commit,
                "--",
            ],
            &[],
            cancel,
        )?;
        Ok(String::from_utf8_lossy(&output).into_owned())
    }
}

fn parse_head(line: &str) -> Result<Option<String>> {
    let (name, kind) = line.split_once(' ').context("Invalid Git HEAD result")?;
    if line == "HEAD^{commit} missing" {
        return Ok(None);
    }
    ensure!(
        kind == "commit" && valid_commit(name),
        "Git returned an invalid HEAD commit"
    );
    Ok(Some(name.to_owned()))
}

fn parse_page(output: &[u8], offset: usize) -> Result<CommitPage> {
    let mut rows = Vec::new();
    if !output.is_empty() {
        let records = output
            .strip_suffix(&[0])
            .context("Incomplete Git history")?;
        let mut fields = records.split(|byte| *byte == 0);
        while let Some(id) = fields.next() {
            let id = std::str::from_utf8(id)?;
            ensure!(
                valid_commit(id),
                "Git returned an invalid history commit ID"
            );
            let subject = fields.next().context("Missing Git commit subject")?;
            rows.push(HistoryCommit {
                id: id.to_owned(),
                subject: String::from_utf8_lossy(subject).into_owned(),
            });
        }
    }
    let next_offset = if rows.len() > PAGE_SIZE {
        rows.truncate(PAGE_SIZE);
        Some(
            offset
                .checked_add(PAGE_SIZE)
                .context("Git history offset overflow")?,
        )
    } else {
        None
    };
    Ok(CommitPage { rows, next_offset })
}

fn read_git(project: &Path, args: &[&str], input: &[u8], cancel: &AtomicBool) -> Result<Vec<u8>> {
    let output = git_output(
        project,
        args,
        "revision_unavailable",
        GitInput {
            bytes: input,
            cancel: Some(cancel),
            output_limit: Some(OUTPUT_LIMIT),
        },
    )?;
    let mut bytes = Vec::new();
    output.take(OUTPUT_LIMIT).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() < OUTPUT_LIMIT as usize,
        "Git history output exceeded 8 MiB"
    );
    Ok(bytes)
}
