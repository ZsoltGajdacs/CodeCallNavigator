//! Bounded, read-only capture of changed paths and file contents from Git and the checkout.
use super::{
    GitInput, Repository,
    changes::{ChangedPath, paths},
    failure, git_output,
    repository::read_blob,
    valid_commit,
    worktree_file::{self, Entry},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, Read},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

const MAX_PATH_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CAPTURE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TEXT_BYTES: u64 = 64 * 1024 * 1024;

pub struct SourceChanges {
    pub digest: String,
    pub files: Vec<SourceFile>,
}

pub struct SourceFile {
    pub id: String,
    pub path: String,
    pub before: SourceSide,
    pub after: SourceSide,
    pub reason: Option<&'static str>,
}

#[derive(Clone)]
pub enum SourceSide {
    Absent,
    Text(Arc<str>),
    Unavailable(&'static str),
}

impl SourceSide {
    pub fn present(&self) -> bool {
        !matches!(self, Self::Absent)
    }
}

impl Repository {
    /// With no head commit, read the effective checkout, including untracked files.
    pub fn source_changes(
        &self,
        base: &str,
        head: Option<&str>,
        cancel: &AtomicBool,
    ) -> Result<SourceChanges> {
        ensure!(valid_commit(base), "Invalid exact baseline commit");
        ensure!(head.is_none_or(valid_commit), "Invalid exact head commit");
        let revisions = RevisionPair { base, head };
        let mut result = capture(self, revisions, cancel, true)?;
        if head.is_none() && result.digest != capture(self, revisions, cancel, false)?.digest {
            return Err(failure(
                "comparison_mismatch",
                "Worktree source changed while reading the comparison. Reload comparison.",
            ));
        }
        // Even an empty diff belongs to this repository and exact endpoint pair.
        let mut digest = blake3::Hasher::new();
        hash(&mut digest, self.root.as_os_str().as_encoded_bytes());
        hash(&mut digest, self.project.as_bytes());
        hash(&mut digest, base.as_bytes());
        hash(&mut digest, head.unwrap_or("WORKTREE").as_bytes());
        hash(&mut digest, result.digest.as_bytes());
        result.digest = digest.finalize().to_hex().to_string();
        Ok(result)
    }
}

#[derive(Default)]
struct Candidate<'a> {
    tracked: Option<ChangedPath<'a>>,
    untracked: bool,
    embedded_repository: bool,
}

impl Candidate<'_> {
    fn checkout_mode(&self) -> Option<&str> {
        self.tracked.as_ref().map(|entry| entry.new_mode)
    }

    fn removed_from_checkout(&self) -> bool {
        !self.untracked
            && self
                .tracked
                .as_ref()
                .is_some_and(|entry| entry.status == "D")
    }
}

#[derive(Clone, Copy)]
struct RevisionPair<'a> {
    base: &'a str,
    head: Option<&'a str>,
}

fn capture(
    repository: &Repository,
    revisions: RevisionPair<'_>,
    cancel: &AtomicBool,
    retain: bool,
) -> Result<SourceChanges> {
    let project = repository.root.join(&repository.project);
    let raw = raw_changes(&project, revisions, cancel)?;
    let untracked = if revisions.head.is_none() {
        path_output(
            &project,
            &["ls-files", "--others", "--exclude-standard", "-z", "--"],
            &[],
            cancel,
        )?
    } else {
        Vec::new()
    };
    let candidates = candidates(&raw, &untracked)?;
    let mut budget = Budget::new(cancel, retain);
    let blobs = capture_blobs(
        repository,
        blob_ids(&candidates, revisions.head.is_some()),
        &mut budget,
    )?;
    let attributes = if revisions.head.is_none() {
        worktree_attributes(&project, candidates.keys().copied(), cancel)?
    } else {
        BTreeMap::new()
    };
    let mut digest = blake3::Hasher::new();
    let mut files = Vec::new();
    for (path, candidate) in candidates {
        check_cancelled(cancel)?;
        let pair = FilePair::new(path, candidate.tracked, &blobs);
        let pair = if revisions.head.is_none() {
            FilePair {
                after: worktree(&project, path, &candidate, &mut budget)?,
                ..pair
            }
        } else {
            pair
        };
        if let Some(file) = pair.change(attributes.get(path).copied(), &mut digest, retain) {
            files.push(file);
        }
    }
    Ok(SourceChanges {
        digest: digest.finalize().to_hex().to_string(),
        files,
    })
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

fn blob_ids<'a>(
    candidates: &BTreeMap<&[u8], Candidate<'a>>,
    commit_pair: bool,
) -> BTreeSet<&'a str> {
    candidates
        .values()
        .filter_map(|candidate| candidate.tracked.as_ref())
        .flat_map(|entry| {
            let head_blob = if commit_pair {
                blob_object(entry.new_mode, entry.new_object)
            } else {
                None
            };
            blob_object(entry.old_mode, entry.old_object)
                .into_iter()
                .chain(head_blob)
        })
        .collect()
}

struct FilePair<'a> {
    path: &'a [u8],
    before: FileSide<'a>,
    after: FileSide<'a>,
}

impl<'a> FilePair<'a> {
    fn new(
        path: &'a [u8],
        entry: Option<ChangedPath<'a>>,
        blobs: &BTreeMap<&str, CapturedBlob>,
    ) -> Self {
        // A live regular file has no Git object ID. Its side is replaced by the
        // checkout capture before the pair is used.
        Self {
            path,
            before: entry
                .map(|entry| saved(entry.old_mode, entry.old_object, blobs))
                .unwrap_or_default(),
            after: entry
                .map(|entry| saved(entry.new_mode, entry.new_object, blobs))
                .unwrap_or_default(),
        }
    }

    fn change(
        self,
        attribute: Option<&'static str>,
        digest: &mut blake3::Hasher,
        retain: bool,
    ) -> Option<SourceFile> {
        let Self {
            path,
            before,
            after,
        } = self;
        if before.same_as(&after) && !matches!(after.mode, "160000" | "special") {
            return None;
        }
        let reason = std::str::from_utf8(path)
            .is_err()
            .then_some("non_utf8_path")
            .or(attribute)
            .or_else(|| {
                before
                    .mode_only_change(&after)
                    .then_some("mode_only_change")
            });
        hash(digest, path);
        for side in [&before, &after] {
            hash(digest, side.mode.as_bytes());
            hash(digest, side.digest.as_bytes());
        }
        hash(digest, reason.unwrap_or_default().as_bytes());
        retain.then(|| SourceFile {
            id: format!("git-file-{}", blake3::hash(path).to_hex()),
            path: display_path(path),
            before: before.source,
            after: after.source,
            reason,
        })
    }
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
        return Err(over_budget());
    }
    Ok(bytes)
}

fn worktree_attributes<'a>(
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

fn display_path(path: &[u8]) -> String {
    std::str::from_utf8(path).map_or_else(
        |_| {
            path.iter()
                .flat_map(|byte| std::ascii::escape_default(*byte))
                .map(char::from)
                .collect()
        },
        str::to_owned,
    )
}

fn hash(digest: &mut blake3::Hasher, field: &[u8]) {
    digest.update(&(field.len() as u64).to_le_bytes());
    digest.update(field);
}

struct Budget<'a> {
    consumed: u64,
    text: u64,
    retain: bool,
    cancel: &'a AtomicBool,
}

impl<'a> Budget<'a> {
    fn new(cancel: &'a AtomicBool, retain: bool) -> Self {
        Self {
            consumed: 0,
            text: 0,
            retain,
            cancel,
        }
    }

    fn read(&mut self, source: &mut dyn Read, size: u64) -> Result<CapturedBlob> {
        let retain = self.reserve(size)?;
        let mut asset = source.take(size);
        let mut digest = blake3::Hasher::new();
        let mut data = Vec::new();
        let mut buffer = [0; 64 * 1024];
        while asset.limit() != 0 {
            check_cancelled(self.cancel)?;
            let count = asset.read(&mut buffer)?;
            if count == 0 {
                return Err(failure(
                    "comparison_mismatch",
                    "A compared file changed while reading source",
                ));
            }
            digest.update(&buffer[..count]);
            if retain {
                data.extend_from_slice(&buffer[..count]);
            }
        }
        let contents = if retain {
            text(data)
        } else {
            SourceSide::Unavailable("source_too_large")
        };
        Ok(CapturedBlob {
            digest: digest.finalize().to_hex().to_string(),
            contents,
        })
    }

    fn reserve(&mut self, size: u64) -> Result<bool> {
        self.consumed = self.consumed.saturating_add(size);
        if self.consumed > MAX_CAPTURE_BYTES {
            return Err(over_budget());
        }
        let retain = self.retain && size <= crate::flow::MAX_SOURCE_BYTES;
        if retain {
            self.text = self.text.saturating_add(size);
            if self.text > MAX_TEXT_BYTES {
                return Err(over_budget());
            }
        }
        Ok(retain)
    }
}

fn check_cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        return Err(failure("cancelled", "Source comparison cancelled"));
    }
    Ok(())
}

fn text(data: Vec<u8>) -> SourceSide {
    if !data.contains(&0)
        && let Ok(text) = String::from_utf8(data)
    {
        SourceSide::Text(Arc::from(text))
    } else {
        SourceSide::Unavailable("binary_content")
    }
}

fn over_budget() -> anyhow::Error {
    failure(
        "diff_too_large",
        "Git changes exceed the source comparison capture budget",
    )
}

struct CapturedBlob {
    digest: String,
    contents: SourceSide,
}

fn blob_object<'a>(mode: &str, object: &'a str) -> Option<&'a str> {
    ((regular_mode(mode) || mode == "120000") && !object.bytes().all(|byte| byte == b'0'))
        .then_some(object)
}

fn regular_mode(mode: &str) -> bool {
    matches!(mode, "100644" | "100755")
}

fn capture_blobs<'a>(
    repository: &Repository,
    ids: BTreeSet<&'a str>,
    budget: &mut Budget<'_>,
) -> Result<BTreeMap<&'a str, CapturedBlob>> {
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut input = String::new();
    for id in &ids {
        ensure!(valid_commit(id), "Invalid object identity in Git diff");
        input.push_str(id);
        input.push('\n');
    }
    let limit = MAX_CAPTURE_BYTES + MAX_PATH_BYTES;
    let mut output = repository.blob_batch(&input, Some(budget.cancel), Some(limit))?;
    if output.get_ref().metadata()?.len() >= limit {
        return Err(over_budget());
    }
    let mut blobs = BTreeMap::new();
    for id in ids {
        blobs.insert(
            id,
            read_blob(&mut output, id, |source, size| budget.read(source, size))?,
        );
    }
    ensure!(
        output.fill_buf()?.is_empty(),
        "Unexpected data after Git blob batch"
    );
    Ok(blobs)
}

struct FileSide<'a> {
    mode: &'a str,
    digest: String,
    source: SourceSide,
}

impl Default for FileSide<'_> {
    fn default() -> Self {
        Self {
            mode: "000000",
            digest: String::new(),
            source: SourceSide::Absent,
        }
    }
}

impl FileSide<'_> {
    fn same_as(&self, other: &Self) -> bool {
        self.mode == other.mode && self.digest == other.digest
    }

    fn mode_only_change(&self, other: &Self) -> bool {
        regular_mode(self.mode)
            && regular_mode(other.mode)
            && self.mode != other.mode
            && self.digest == other.digest
    }
}

fn saved<'a>(mode: &'a str, object: &str, blobs: &BTreeMap<&str, CapturedBlob>) -> FileSide<'a> {
    match mode {
        "000000" => FileSide::default(),
        "100644" | "100755" | "120000" => blobs.get(object).map_or_else(
            || FileSide {
                mode,
                digest: object.into(),
                source: SourceSide::Unavailable("git_blob_unavailable"),
            },
            |blob| FileSide {
                mode,
                digest: blob.digest.clone(),
                source: if mode == "120000" {
                    SourceSide::Unavailable("symlink")
                } else {
                    blob.contents.clone()
                },
            },
        ),
        _ => FileSide {
            mode,
            digest: object.to_owned(),
            source: SourceSide::Unavailable(type_reason(mode)),
        },
    }
}

fn type_reason(mode: &str) -> &'static str {
    match mode {
        "120000" => "symlink",
        "160000" => "submodule",
        _ => "unsupported_file_type",
    }
}

fn worktree<'a>(
    project: &Path,
    path: &[u8],
    candidate: &'a Candidate<'_>,
    budget: &mut Budget<'_>,
) -> Result<FileSide<'a>> {
    let mode = candidate.checkout_mode();
    if candidate.removed_from_checkout() {
        return Ok(FileSide::default());
    }
    let side = match worktree_file::read(project, path)? {
        Some(Entry::Regular {
            mut file,
            executable,
        }) => {
            let mode = mode
                .filter(|mode| matches!(*mode, "100644" | "100755"))
                .unwrap_or(if executable { "100755" } else { "100644" });
            let size = file.metadata()?.len();
            let captured = budget.read(&mut file, size)?;
            if file.read(&mut [0])? != 0 {
                return Err(failure(
                    "comparison_mismatch",
                    "A compared file grew while reading source",
                ));
            }
            FileSide {
                mode,
                digest: captured.digest,
                source: captured.contents,
            }
        }
        Some(Entry::Link(target)) => FileSide {
            mode: "120000",
            digest: blake3::hash(&target).to_hex().to_string(),
            source: SourceSide::Unavailable("symlink"),
        },
        Some(Entry::Directory) if candidate.embedded_repository => FileSide {
            mode: "160000",
            digest: String::new(),
            source: SourceSide::Unavailable("embedded_repository"),
        },
        Some(Entry::Directory) if mode == Some("160000") => FileSide {
            mode: "160000",
            digest: candidate
                .tracked
                .as_ref()
                .context("Git submodule has no diff entry")?
                .new_object
                .into(),
            source: SourceSide::Unavailable("submodule"),
        },
        Some(Entry::Special) => FileSide {
            mode: "special",
            digest: String::new(),
            source: SourceSide::Unavailable("unsupported_file_type"),
        },
        _ => FileSide::default(),
    };
    Ok(side)
}
