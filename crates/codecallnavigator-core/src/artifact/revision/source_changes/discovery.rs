//! Enumerate the Git changed paths and checkout attributes independently of content reads.
use super::super::{
    GitInput,
    changes::{ChangedPath, paths},
    failure, git_output,
};
use anyhow::{Result, ensure};
use std::{collections::BTreeMap, io::Read, path::Path, sync::atomic::AtomicBool};

pub(super) const MAX_PATH_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Default)]
pub(super) struct Candidate<'a> {
    pub(super) tracked: Option<ChangedPath<'a>>,
    untracked: bool,
    pub(super) embedded_repository: bool,
}

impl Candidate<'_> {
    pub(super) fn checkout_mode(&self) -> Option<&str> {
        self.tracked.as_ref().map(|entry| entry.new_mode)
    }

    pub(super) fn removed_from_checkout(&self) -> bool {
        !self.untracked
            && self
                .tracked
                .as_ref()
                .is_some_and(|entry| entry.status == "D")
    }
}

#[derive(Clone, Copy)]
pub(super) struct RevisionPair<'a> {
    pub(super) base: &'a str,
    pub(super) head: Option<&'a str>,
}

pub(super) struct Paths {
    raw: Vec<u8>,
    untracked: Vec<u8>,
}

impl Paths {
    pub(super) fn read(
        project: &Path,
        revisions: RevisionPair<'_>,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        let raw = raw_changes(project, revisions, cancel)?;
        let untracked = if revisions.head.is_none() {
            path_output(
                project,
                &["ls-files", "--others", "--exclude-standard", "-z", "--"],
                &[],
                cancel,
            )?
        } else {
            Vec::new()
        };
        Ok(Self { raw, untracked })
    }

    pub(super) fn candidates(&self) -> Result<BTreeMap<&[u8], Candidate<'_>>> {
        candidates(&self.raw, &self.untracked)
    }
}

pub(super) fn metadata_limit() -> anyhow::Error {
    failure(
        "diff_too_large",
        "Git change metadata exceeds the source comparison path limit",
    )
}

fn candidates<'a>(raw: &'a [u8], untracked: &'a [u8]) -> Result<BTreeMap<&'a [u8], Candidate<'a>>> {
    let mut candidates = BTreeMap::new();
    for entry in paths(raw)? {
        candidates
            .entry(entry.path)
            .or_insert_with(Candidate::default)
            .tracked = Some(entry);
    }
    for path in nul_fields(untracked)? {
        let bare_path = path.strip_suffix(b"/").unwrap_or(path);
        let candidate = candidates
            .entry(bare_path)
            .or_insert_with(Candidate::default);
        candidate.untracked = true;
        candidate.embedded_repository = bare_path != path;
    }
    Ok(candidates)
}

fn raw_changes(
    project: &Path,
    revisions: RevisionPair<'_>,
    cancel: &AtomicBool,
) -> Result<Vec<u8>> {
    let mut args = vec![
        "-c",
        "core.fsmonitor=false",
        "diff",
        "--raw",
        "-z",
        "--abbrev=64",
        "--no-ext-diff",
        "--no-textconv",
        "--no-renames",
        "--relative",
        "--ignore-submodules=none",
        revisions.base,
    ];
    args.extend(revisions.head);
    args.push("--");
    path_output(project, &args, &[], cancel)
}

fn path_output(
    project: &Path,
    args: &[&str],
    input: &[u8],
    cancel: &AtomicBool,
) -> Result<Vec<u8>> {
    let output = git_output(
        project,
        args,
        "diff_unavailable",
        GitInput {
            bytes: input,
            cancel: Some(cancel),
            output_limit: Some(MAX_PATH_BYTES),
        },
    )?;
    let mut bytes = Vec::new();
    output.take(MAX_PATH_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 >= MAX_PATH_BYTES {
        return Err(metadata_limit());
    }
    Ok(bytes)
}

pub(super) fn worktree_attributes<'a>(
    project: &Path,
    paths: impl Iterator<Item = &'a [u8]>,
    cancel: &AtomicBool,
) -> Result<BTreeMap<Vec<u8>, &'static str>> {
    let mut input = Vec::new();
    for path in paths {
        input.extend_from_slice(path);
        input.push(0);
    }
    if input.is_empty() {
        return Ok(BTreeMap::new());
    }
    let bytes = path_output(
        project,
        &[
            "check-attr",
            "-z",
            "--stdin",
            "filter",
            "working-tree-encoding",
            "diff",
        ],
        &input,
        cancel,
    )?;
    let fields = nul_fields(&bytes)?.collect::<Vec<_>>();
    let (records, remainder) = fields.as_chunks::<3>();
    ensure!(remainder.is_empty(), "Invalid Git attribute response");
    let mut attributes = BTreeMap::new();
    for &[path, attribute, value] in records {
        match (attribute, value) {
            (b"diff", b"unset") => {
                attributes.entry(path.to_vec()).or_insert("binary_content");
            }
            (b"filter" | b"working-tree-encoding", value)
                if !matches!(value, b"unspecified" | b"unset") =>
            {
                attributes.insert(path.to_vec(), "worktree_content_transformation");
            }
            _ => {}
        }
    }
    Ok(attributes)
}

fn nul_fields(bytes: &[u8]) -> Result<impl Iterator<Item = &[u8]>> {
    ensure!(
        bytes.is_empty() || bytes.ends_with(&[0]),
        "Incomplete Git path response"
    );
    Ok(bytes
        .split_inclusive(|&byte| byte == 0)
        .map(|path| &path[..path.len() - 1]))
}
