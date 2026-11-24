//! Bounded, read-only capture of changed paths and file contents from Git and the checkout.
mod content;
mod discovery;

use super::{
    Repository, failure, valid_commit,
    worktree_file::{self, Entry},
};
use anyhow::{Context, Result, ensure};
use content::{BlobRequest, Budget, Content, Preview, ReadPolicy, capture_blobs};
use discovery::{Candidate, Paths, RevisionPair, worktree_attributes};
use std::{
    collections::{BTreeMap, btree_map::Entry as MapEntry},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

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

fn capture(
    repository: &Repository,
    revisions: RevisionPair<'_>,
    cancel: &AtomicBool,
    retain: bool,
) -> Result<SourceChanges> {
    let project = repository.root.join(&repository.project);
    let paths = Paths::read(&project, revisions, cancel)?;
    let candidates = paths.candidates()?;
    let checkout = revisions.head.is_none();
    let attributes = if checkout {
        worktree_attributes(&project, candidates.keys().copied(), cancel)?
    } else {
        BTreeMap::new()
    };
    let mut budget = Budget::new(cancel, retain);
    let requests = blob_requests(&candidates, &attributes, !checkout)?;
    let blobs = capture_blobs(repository, &requests, &mut budget)?;
    let mut capture = FileCapture {
        project: checkout.then_some(&project),
        blobs,
        budget,
    };

    let mut digest = blake3::Hasher::new();
    let mut files = BTreeMap::new();
    let tracked = candidates
        .iter()
        .filter(|(_, entry)| entry.tracked.is_some());
    let untracked = candidates
        .iter()
        .filter(|(_, entry)| entry.tracked.is_none());
    for (&path, candidate) in tracked.chain(untracked) {
        check_cancelled(cancel)?;
        let attribute = attributes.get(path).copied();
        let pair = capture.read(path, candidate, attribute)?;
        if let Some(file) = pair.change(attribute, &mut digest, retain) {
            files.insert(path, file);
        }
    }
    Ok(SourceChanges {
        digest: digest.finalize().to_hex().to_string(),
        files: files.into_values().collect(),
    })
}

fn blob_requests<'a>(
    candidates: &BTreeMap<&[u8], Candidate<'a>>,
    attributes: &BTreeMap<Vec<u8>, &'static str>,
    commit_pair: bool,
) -> Result<Vec<BlobRequest<'a>>> {
    let mut by_id = BTreeMap::new();
    let mut requests: Vec<BlobRequest<'a>> = Vec::new();
    let side_count = if commit_pair { 2 } else { 1 };
    for (&path, candidate) in candidates {
        let Some(entry) = candidate.tracked else {
            continue;
        };
        let sides = [
            (entry.old_mode, entry.old_object),
            (entry.new_mode, entry.new_object),
        ];
        for (mode, object) in sides.into_iter().take(side_count) {
            let Some(id) = blob_object(mode, object) else {
                continue;
            };
            ensure!(valid_commit(id), "Invalid object identity in Git diff");
            let forced_reason = attributes
                .get(path)
                .copied()
                .or_else(|| (!regular_mode(mode)).then(|| type_reason(mode)));
            match by_id.entry(id) {
                MapEntry::Vacant(position) => {
                    position.insert(requests.len());
                    requests.push(BlobRequest { id, forced_reason });
                }
                MapEntry::Occupied(position) => {
                    let request = &mut requests[*position.get()];
                    request.forced_reason = request.forced_reason.and(forced_reason);
                }
            }
        }
    }
    Ok(requests)
}

struct FileCapture<'a> {
    project: Option<&'a Path>,
    blobs: BTreeMap<&'a str, Content>,
    budget: Budget<'a>,
}

impl FileCapture<'_> {
    fn read<'a>(
        &mut self,
        path: &'a [u8],
        candidate: &'a Candidate<'a>,
        forced_reason: Option<&'static str>,
    ) -> Result<FilePair<'a>> {
        let before = candidate
            .tracked
            .map(|entry| saved(entry.old_mode, entry.old_object, &self.blobs))
            .unwrap_or_default();
        // A tracked regular file can be byte-identical after a mode change or
        // a staged removal followed by recreation. Keep the full hash to prove it.
        let policy = ReadPolicy {
            hash_binary_to_end: regular_mode(before.mode)
                && matches!(before.identity, Identity::Content(_)),
            forced_reason,
        };
        let after = self
            .read_worktree(path, candidate, policy)?
            .unwrap_or_else(|| {
                candidate
                    .tracked
                    .map(|entry| saved(entry.new_mode, entry.new_object, &self.blobs))
                    .unwrap_or_default()
            });
        Ok(FilePair {
            path,
            before,
            after,
        })
    }

    fn read_worktree<'a>(
        &mut self,
        path: &[u8],
        candidate: &'a Candidate<'_>,
        policy: ReadPolicy,
    ) -> Result<Option<FileSide<'a>>> {
        let Some(project) = self.project else {
            return Ok(None);
        };
        let mode = candidate.checkout_mode();
        if candidate.removed_from_checkout() {
            return Ok(Some(FileSide::default()));
        }
        let side = match worktree_file::read(project, path)? {
            Some(Entry::Regular { file, executable }) => {
                let mode = mode
                    .filter(|mode| matches!(*mode, "100644" | "100755"))
                    .unwrap_or(if executable { "100755" } else { "100644" });
                capture_regular_file(file, mode, &mut self.budget, policy)?
            }
            Some(Entry::Link(target)) => FileSide {
                mode: "120000",
                identity: Identity::Content(blake3::hash(&target)),
                preview: Preview::Unavailable("symlink"),
            },
            Some(Entry::Directory) if candidate.embedded_repository => FileSide {
                mode: "160000",
                identity: Identity::Absent,
                preview: Preview::Unavailable("embedded_repository"),
            },
            Some(Entry::Directory) if mode == Some("160000") => FileSide {
                mode: "160000",
                identity: Identity::GitObject(
                    candidate
                        .tracked
                        .as_ref()
                        .context("Git submodule has no diff entry")?
                        .new_object,
                ),
                preview: Preview::Unavailable("submodule"),
            },
            Some(Entry::Special) => FileSide {
                mode: "special",
                identity: Identity::Absent,
                preview: Preview::Unavailable("unsupported_file_type"),
            },
            _ => FileSide::default(),
        };
        Ok(Some(side))
    }
}

struct FilePair<'a> {
    path: &'a [u8],
    before: FileSide<'a>,
    after: FileSide<'a>,
}

impl FilePair<'_> {
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
            side.identity.bind(digest);
            hash(digest, side.preview.reason().as_bytes());
        }
        hash(digest, reason.unwrap_or_default().as_bytes());
        retain.then(|| SourceFile {
            id: format!("git-file-{}", blake3::hash(path).to_hex()),
            path: display_path(path),
            before: before.preview.into_source(),
            after: after.preview.into_source(),
            reason,
        })
    }
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

fn check_cancelled(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        return Err(failure("cancelled", "Source comparison cancelled"));
    }
    Ok(())
}

fn blob_object<'a>(mode: &str, object: &'a str) -> Option<&'a str> {
    ((regular_mode(mode) || mode == "120000") && !object.bytes().all(|byte| byte == b'0'))
        .then_some(object)
}

fn regular_mode(mode: &str) -> bool {
    matches!(mode, "100644" | "100755")
}

struct FileSide<'a> {
    mode: &'a str,
    identity: Identity<'a>,
    preview: Preview,
}

#[derive(PartialEq, Eq)]
enum Identity<'a> {
    Absent,
    Content(blake3::Hash),
    GitObject(&'a str),
    WorktreeMetadata(blake3::Hash),
}

impl Identity<'_> {
    fn bind(&self, digest: &mut blake3::Hasher) {
        let (tag, value): (&[u8], &[u8]) = match self {
            Self::Absent => (b"absent", b""),
            Self::Content(content) => (b"content", content.as_bytes()),
            Self::GitObject(object) => (b"object", object.as_bytes()),
            Self::WorktreeMetadata(metadata) => (b"metadata", metadata.as_bytes()),
        };
        hash(digest, tag);
        hash(digest, value);
    }

    fn same_content(&self, other: &Self) -> bool {
        !matches!(self, Self::WorktreeMetadata(_)) && self == other
    }
}

impl Default for FileSide<'_> {
    fn default() -> Self {
        Self {
            mode: "000000",
            identity: Identity::Absent,
            preview: Preview::Absent,
        }
    }
}

impl FileSide<'_> {
    fn same_as(&self, other: &Self) -> bool {
        self.mode == other.mode && self.identity.same_content(&other.identity)
    }

    fn mode_only_change(&self, other: &Self) -> bool {
        regular_mode(self.mode)
            && regular_mode(other.mode)
            && self.mode != other.mode
            && self.identity.same_content(&other.identity)
    }
}

fn saved<'a>(mode: &'a str, object: &'a str, blobs: &BTreeMap<&str, Content>) -> FileSide<'a> {
    if mode == "000000" {
        return FileSide::default();
    }
    let blob = blobs.get(object);
    let identity = blob
        .and_then(|blob| blob.digest)
        .map_or(Identity::GitObject(object), Identity::Content);
    let preview = if regular_mode(mode) {
        blob.map_or(Preview::Unavailable("git_blob_unavailable"), |blob| {
            blob.preview.clone()
        })
    } else {
        Preview::Unavailable(type_reason(mode))
    };
    FileSide {
        mode,
        identity,
        preview,
    }
}

fn type_reason(mode: &str) -> &'static str {
    match mode {
        "120000" => "symlink",
        "160000" => "submodule",
        _ => "unsupported_file_type",
    }
}

fn capture_regular_file<'a>(
    mut file: std::fs::File,
    mode: &'a str,
    budget: &mut Budget<'_>,
    policy: ReadPolicy,
) -> Result<FileSide<'a>> {
    let metadata = file.metadata()?;
    let stamp = worktree_file::metadata_stamp(&metadata);
    let captured = budget.read_worktree(&mut file, metadata.len(), policy)?;
    if stamp != worktree_file::metadata_stamp(&file.metadata()?) {
        return Err(failure(
            "comparison_mismatch",
            "A compared file changed while reading source",
        ));
    }
    let identity = captured
        .digest
        .map_or(Identity::WorktreeMetadata(stamp), Identity::Content);
    Ok(FileSide {
        mode,
        identity,
        preview: captured.preview,
    })
}
