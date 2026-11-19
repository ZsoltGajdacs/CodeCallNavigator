//! Resolve repository identity and read the exact inputs recorded in a commit.
use super::{GitInput, git, git_output, project_prefix, resolve_commit};
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Repository {
    pub root: PathBuf,
    pub common_dir: PathBuf,
    pub project: String,
}

impl Repository {
    pub fn detect(project: &Path) -> Result<Option<Self>> {
        if !has_worktree_marker(project)? && std::env::var_os("GIT_DIR").is_none() {
            return Ok(None);
        }
        match Self::open(project) {
            Ok(repository) => Ok(Some(repository)),
            Err(error) if super::not_repository(&error) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn open(project: &Path) -> Result<Self> {
        let root = git_path(project, &["rev-parse", "--show-toplevel"])?;
        let common_dir = git_path(
            project,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?;
        Ok(Self {
            root,
            common_dir,
            project: project_prefix(project)?.trim_end_matches('/').into(),
        })
    }

    pub fn resolve(&self, revision: &str) -> Result<String> {
        resolve_commit(&self.root, revision)
    }

    pub fn tree(&self, revision: &str, cancel: &AtomicBool) -> Result<BTreeMap<String, TreeEntry>> {
        let output = git_output(
            &self.root,
            &["ls-tree", "-rtz", revision],
            "revision_unavailable",
            GitInput {
                bytes: &[],
                cancel: Some(cancel),
            },
        )?;
        BufReader::new(output)
            .split(0)
            .map(|entry| tree_entry(&entry?))
            .collect()
    }

    pub fn verify_blobs(
        &self,
        expected: &BTreeMap<String, String>,
        cancel: &AtomicBool,
    ) -> Result<()> {
        if expected.is_empty() {
            return Ok(());
        }
        let input: String = expected.keys().map(|id| format!("{id}\n")).collect();
        let output = git_output(
            &self.root,
            &["cat-file", "--batch"],
            "revision_unavailable",
            GitInput {
                bytes: input.as_bytes(),
                cancel: Some(cancel),
            },
        )?;
        verify_batch(BufReader::new(output), expected, cancel)
    }
}

fn has_worktree_marker(project: &Path) -> Result<bool> {
    for directory in project.ancestors() {
        if directory.join(".git").try_exists()? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub struct TreeEntry {
    mode: String,
    object: String,
}

impl TreeEntry {
    pub fn is_directory(&self) -> bool {
        self.mode == "040000"
    }

    pub fn is_link(&self) -> bool {
        matches!(self.mode.as_str(), "120000" | "160000")
    }

    pub fn regular_blob(&self) -> Option<&str> {
        matches!(self.mode.as_str(), "100644" | "100755").then_some(&self.object)
    }
}

fn tree_entry(bytes: &[u8]) -> Result<(String, TreeEntry)> {
    let (metadata, path) = std::str::from_utf8(bytes)?
        .split_once('\t')
        .context("Invalid Git tree entry")?;
    let mut fields = metadata.split(' ');
    let mode = fields.next().context("Missing Git tree mode")?.into();
    fields.next().context("Missing Git tree type")?;
    let object = fields.next().context("Missing Git tree object")?.into();
    Ok((path.into(), TreeEntry { mode, object }))
}

fn git_path(project: &Path, arguments: &[&str]) -> Result<PathBuf> {
    let bytes = git(project, arguments, "revision_unavailable")?;
    let output = std::str::from_utf8(&bytes)?;
    let path = output
        .strip_suffix('\n')
        .context("Invalid Git path response")?;
    Path::new(path)
        .canonicalize()
        .context("Cannot open Git directory")
}

fn verify_batch(
    mut output: impl BufRead,
    expected: &BTreeMap<String, String>,
    cancel: &AtomicBool,
) -> Result<()> {
    let mut header = String::new();
    for (id, hash) in expected {
        ensure!(
            !cancel.load(Ordering::Acquire),
            "Git snapshot read cancelled"
        );
        header.clear();
        output.read_line(&mut header)?;
        let size: u64 = header
            .trim_end_matches('\n')
            .strip_prefix(id)
            .and_then(|rest| rest.strip_prefix(" blob "))
            .context("Git blob is unavailable")?
            .parse()?;
        let mut blob = (&mut output).take(size);
        let mut digest = blake3::Hasher::new();
        digest.update_reader(&mut blob)?;
        ensure!(blob.limit() == 0, "Truncated Git blob");
        let mut newline = [0];
        output.read_exact(&mut newline)?;
        ensure!(newline == *b"\n", "Invalid Git blob separator");
        ensure!(
            digest.finalize().to_hex().as_str() == hash,
            "An analyzed input differs from its Git blob {id}"
        );
    }
    ensure!(
        output.fill_buf()?.is_empty(),
        "Unexpected data after Git blob batch"
    );
    Ok(())
}
