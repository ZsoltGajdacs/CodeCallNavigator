//! Bound optional file reads and retained text separately from Git path discovery.
use super::{
    SourceSide, check_cancelled,
    discovery::{MAX_PATH_BYTES, metadata_limit},
};
use crate::artifact::revision::{
    GitInput, Repository, failure,
    repository::{BlobBatch, read_blob, read_blob_size},
};
use anyhow::{Result, ensure};
use std::{
    collections::BTreeMap,
    io::{BufRead, Read},
    sync::{Arc, atomic::AtomicBool},
};

const MAX_CAPTURE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TEXT_BYTES: u64 = 64 * 1024 * 1024;
const BUDGET_EXCEEDED: &str = "source_capture_budget_exceeded";
const TOO_LARGE: &str = "source_too_large";
const BINARY: &str = "binary_content";

pub(super) struct Budget<'a> {
    read_remaining: u64,
    text_remaining: u64,
    retain: bool,
    cancel: &'a AtomicBool,
}

#[derive(Clone)]
pub(super) enum Preview {
    Absent,
    Text(Option<Arc<str>>),
    Unavailable(&'static str),
}

impl Preview {
    pub(super) fn reason(&self) -> &'static str {
        match self {
            Self::Unavailable(reason) => reason,
            Self::Absent | Self::Text(_) => "",
        }
    }

    pub(super) fn into_source(self) -> SourceSide {
        match self {
            Self::Absent => SourceSide::Absent,
            Self::Text(text) => {
                SourceSide::Text(text.expect("source rows are built only from retaining capture"))
            }
            Self::Unavailable(reason) => SourceSide::Unavailable(reason),
        }
    }
}

pub(super) struct Content {
    pub(super) digest: Option<blake3::Hash>,
    pub(super) preview: Preview,
}

impl Content {
    fn skipped(reason: &'static str) -> Self {
        Self {
            digest: None,
            preview: Preview::Unavailable(reason),
        }
    }
}

impl<'a> Budget<'a> {
    pub(super) fn new(cancel: &'a AtomicBool, retain: bool) -> Self {
        Self {
            read_remaining: MAX_CAPTURE_BYTES,
            text_remaining: MAX_TEXT_BYTES,
            retain,
            cancel,
        }
    }

    pub(super) fn read_worktree(
        &mut self,
        source: &mut dyn Read,
        size: u64,
        policy: ReadPolicy,
    ) -> Result<Content> {
        if let Some(reason) = unavailable(size, self.read_remaining) {
            return Ok(Content::skipped(reason));
        }
        self.read(source, size, policy)
    }

    fn read(&mut self, source: &mut dyn Read, size: u64, policy: ReadPolicy) -> Result<Content> {
        let mut asset = source.take(size);
        let mut digest = blake3::Hasher::new();
        let mut bytes = policy.forced_reason.is_none().then(Vec::new);
        let mut buffer = [0; 64 * 1024];
        while asset.limit() != 0 {
            let chunk = self.read_chunk(&mut asset, &mut buffer)?;
            digest.update(chunk);
            let binary = chunk.contains(&0);
            if binary && policy.may_stop_on_nul(asset.limit()) {
                return Ok(Content::skipped(policy.forced_reason.unwrap_or(BINARY)));
            }
            append_text(&mut bytes, chunk, binary);
        }
        let preview = match policy.forced_reason {
            Some(reason) => Preview::Unavailable(reason),
            None => self.preview(bytes),
        };
        Ok(Content {
            digest: Some(digest.finalize()),
            preview,
        })
    }

    fn git_input<'b>(&'b self, input: &'b str, limit: u64) -> GitInput<'b> {
        GitInput {
            bytes: input.as_bytes(),
            cancel: Some(self.cancel),
            output_limit: Some(limit),
        }
    }

    fn read_chunk<'b>(&mut self, asset: &mut dyn Read, buffer: &'b mut [u8]) -> Result<&'b [u8]> {
        check_cancelled(self.cancel)?;
        let count = asset.read(buffer)?;
        if count == 0 {
            return Err(failure(
                "comparison_mismatch",
                "A compared file changed while reading source",
            ));
        }
        self.read_remaining -= count as u64;
        Ok(&buffer[..count])
    }

    fn preview(&mut self, bytes: Option<Vec<u8>>) -> Preview {
        let Some(text) = bytes.and_then(|value| String::from_utf8(value).ok()) else {
            return Preview::Unavailable(BINARY);
        };
        if text.len() as u64 > self.text_remaining {
            return Preview::Unavailable(BUDGET_EXCEEDED);
        }
        self.text_remaining -= text.len() as u64;
        Preview::Text(self.retain.then(|| Arc::from(text)))
    }
}

fn append_text(bytes: &mut Option<Vec<u8>>, chunk: &[u8], binary: bool) {
    if binary {
        *bytes = None;
    } else if let Some(text) = bytes {
        text.extend_from_slice(chunk);
    }
}

#[derive(Clone, Copy)]
pub(super) struct ReadPolicy {
    pub(super) hash_binary_to_end: bool,
    pub(super) forced_reason: Option<&'static str>,
}

impl ReadPolicy {
    fn may_stop_on_nul(self, remaining: u64) -> bool {
        !self.hash_binary_to_end && remaining != 0
    }
}

fn unavailable(size: u64, remaining: u64) -> Option<&'static str> {
    if size > crate::flow::MAX_SOURCE_BYTES {
        Some(TOO_LARGE)
    } else if size > remaining {
        Some(BUDGET_EXCEEDED)
    } else {
        None
    }
}

#[derive(Clone, Copy)]
pub(super) struct BlobRequest<'a> {
    pub(super) id: &'a str,
    pub(super) forced_reason: Option<&'static str>,
}

struct SizedBlob<'a> {
    request: BlobRequest<'a>,
    size: u64,
}

pub(super) fn capture_blobs<'a>(
    repository: &Repository,
    requests: &[BlobRequest<'a>],
    budget: &mut Budget<'_>,
) -> Result<BTreeMap<&'a str, Content>> {
    let (selected, mut blobs) = select_blobs(repository, requests, budget)?;
    if selected.is_empty() {
        return Ok(blobs);
    }

    let input = batch_input(selected.iter().map(|blob| blob.request.id));
    // The size preflight bounds headers. Content adds one separator per record.
    let limit = MAX_CAPTURE_BYTES + MAX_PATH_BYTES + selected.len() as u64 + 1;
    let mut output = repository.blob_batch(BlobBatch::Content, budget.git_input(&input, limit))?;
    ensure!(
        output.get_ref().metadata()?.len() < limit,
        "Git blob batch exceeded its output limit"
    );
    for blob in selected {
        let request = blob.request;
        let content = read_blob(&mut output, request.id, |source, size| {
            ensure!(size == blob.size, "Git blob size disagrees with preflight");
            let policy = ReadPolicy {
                hash_binary_to_end: true,
                forced_reason: request.forced_reason,
            };
            budget.read(source, size, policy)
        })?;
        blobs.insert(request.id, content);
    }
    ensure!(
        output.fill_buf()?.is_empty(),
        "Unexpected data after Git blob batch"
    );
    Ok(blobs)
}

fn select_blobs<'a>(
    repository: &Repository,
    requests: &[BlobRequest<'a>],
    budget: &Budget<'_>,
) -> Result<(Vec<SizedBlob<'a>>, BTreeMap<&'a str, Content>)> {
    let mut selected = Vec::new();
    let mut skipped = BTreeMap::new();
    if requests.is_empty() {
        return Ok((selected, skipped));
    }

    let input = batch_input(requests.iter().map(|request| request.id));
    let mut sizes =
        repository.blob_batch(BlobBatch::Size, budget.git_input(&input, MAX_PATH_BYTES))?;
    if sizes.get_ref().metadata()?.len() >= MAX_PATH_BYTES {
        return Err(metadata_limit());
    }
    let mut remaining = budget.read_remaining;
    for request in requests {
        check_cancelled(budget.cancel)?;
        let size = read_blob_size(&mut sizes, request.id)?;
        if let Some(reason) = unavailable(size, remaining) {
            skipped.insert(request.id, Content::skipped(reason));
        } else {
            selected.push(SizedBlob {
                request: *request,
                size,
            });
            remaining -= size;
        }
    }
    ensure!(
        sizes.fill_buf()?.is_empty(),
        "Unexpected data after Git blob size batch"
    );
    Ok((selected, skipped))
}

fn batch_input<'a>(ids: impl Iterator<Item = &'a str>) -> String {
    let mut input = String::new();
    for id in ids {
        input.push_str(id);
        input.push('\n');
    }
    input
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn retention_limits_only_charge_text_and_keep_read_budget_bounded() {
        let cancel = AtomicBool::new(false);
        let sample = ReadPolicy {
            hash_binary_to_end: false,
            forced_reason: None,
        };
        for retain in [false, true] {
            let mut budget = Budget {
                read_remaining: 14,
                text_remaining: 4,
                retain,
                cancel: &cancel,
            };
            let binary = budget
                .read_worktree(&mut Cursor::new(b"x\0xx"), 4, sample)
                .unwrap();
            assert_eq!(binary.preview.reason(), BINARY);
            let policy = ReadPolicy {
                forced_reason: Some(BINARY),
                ..sample
            };
            let forced = budget
                .read_worktree(&mut Cursor::new(b"ok"), 2, policy)
                .unwrap();
            assert_eq!(forced.preview.reason(), BINARY);
            assert!(forced.digest.is_some());
            assert_eq!(budget.text_remaining, 4);
            let text = budget
                .read_worktree(&mut Cursor::new(b"text"), 4, sample)
                .unwrap();
            assert!(matches!(text.preview, Preview::Text(value) if value.is_some() == retain));
            let exhausted = budget
                .read_worktree(&mut Cursor::new(b"no"), 2, sample)
                .unwrap();
            assert_eq!(exhausted.preview.reason(), BUDGET_EXCEEDED);
            assert!(exhausted.digest.is_some());
            let mut too_much = Cursor::new(b"abc");
            let skipped = budget.read_worktree(&mut too_much, 3, sample).unwrap();
            assert_eq!(skipped.preview.reason(), BUDGET_EXCEEDED);
            assert!(skipped.digest.is_none());
            assert_eq!(too_much.position(), 0);
        }
    }
}
