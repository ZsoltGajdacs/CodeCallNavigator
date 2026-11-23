//! Read old-side line ranges from one Git diff without checking out either endpoint.
use super::{GitInput, Repository, git_output, valid_commit};
use crate::artifact::impact::Change;
use anyhow::{Context, Result, ensure};
use std::{borrow::Cow, collections::BTreeMap, io::Read};

const MAX_PATHS: usize = 64;
const MAX_RANGES: usize = 256;
const MAX_DIFF_BYTES: u64 = 16 * 1024 * 1024;

pub struct TrackedChanges {
    pub records: Vec<Change>,
    pub changed_paths: usize,
    pub truncated: bool,
}

impl Repository {
    /// With no head commit, Git compares against the tracked working tree.
    pub fn tracked_changes(&self, base: &str, head: Option<&str>) -> Result<TrackedChanges> {
        ensure!(valid_commit(base), "Invalid exact baseline commit");
        ensure!(head.is_none_or(valid_commit), "Invalid exact head commit");
        let mut args = vec![
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.quotePath=false",
            "diff",
            "--raw",
            "-z",
            "--patch",
            "--unified=0",
            "--inter-hunk-context=0",
            "--diff-algorithm=myers",
            "--no-ext-diff",
            "--no-textconv",
            "--no-renames",
            "--no-color",
            "--relative",
            "--ignore-submodules=none",
            "--submodule=short",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            "--output-indicator-new=+",
            "--output-indicator-old=-",
            "--output-indicator-context= ",
            base,
        ];
        args.extend(head);
        args.push("--");
        let mut bytes = Vec::new();
        git_output(
            &self.root.join(&self.project),
            &args,
            "diff_unavailable",
            GitInput {
                output_limit: Some(MAX_DIFF_BYTES),
                ..GitInput::default()
            },
        )?
        .take(MAX_DIFF_BYTES + 1)
        .read_to_end(&mut bytes)?;
        parse(&bytes)
    }
}

fn parse(bytes: &[u8]) -> Result<TrackedChanges> {
    let oversized = bytes.len() as u64 >= MAX_DIFF_BYTES;
    let (raw, patch) = match bytes.windows(2).position(|pair| pair == b"\0\0") {
        Some(separator) => (&bytes[..=separator], &bytes[separator + 2..]),
        None => (bytes, &[][..]),
    };
    let paths = paths(raw)?;
    let patches = Patches::new(patch)?;
    let mut changes = TrackedChanges {
        records: Vec::new(),
        changed_paths: paths.len(),
        truncated: oversized || paths.len() > MAX_PATHS,
    };
    for path in paths.into_iter().take(MAX_PATHS) {
        let remaining = MAX_RANGES - changes.records.len();
        if remaining == 0 {
            changes.truncated = true;
            break;
        }
        let (records, truncated) = path.records(&patches, remaining, oversized)?;
        changes.records.extend(records);
        changes.truncated |= truncated;
    }
    Ok(changes)
}

#[derive(Clone, Copy)]
pub(super) struct ChangedPath<'a> {
    pub(super) path: &'a [u8],
    pub(super) status: &'a str,
    pub(super) old_mode: &'a str,
    pub(super) new_mode: &'a str,
    pub(super) old_object: &'a str,
    pub(super) new_object: &'a str,
}

impl ChangedPath<'_> {
    fn unsupported(&self) -> Option<&'static str> {
        if std::str::from_utf8(self.path).is_err() {
            return Some("non_utf8_path");
        }
        match (
            self.status,
            self.old_mode.starts_with("100"),
            self.new_mode.starts_with("100"),
        ) {
            ("A", _, _) => Some("no_baseline_file"),
            ("M", true, true) | ("D", true, false) => None,
            _ => Some("unsupported_change"),
        }
    }

    fn record(&self, old_start: u32, old_count: u32, unresolved: Option<&str>) -> Change {
        Change {
            path: String::from_utf8_lossy(self.path).into_owned(),
            status: self.status.to_owned(),
            old_start,
            old_count,
            unresolved: unresolved.map(str::to_owned),
        }
    }

    fn records(
        &self,
        patches: &Patches<'_>,
        remaining: usize,
        oversized: bool,
    ) -> Result<(Vec<Change>, bool)> {
        if let Some(reason) = self.unsupported() {
            return Ok((vec![self.record(0, 0, Some(reason))], false));
        }
        let headers = patches.headers(self.path);
        let mut records = Vec::new();
        for &header in headers.iter().take(remaining) {
            let (start, count) = old_range(header)?;
            records.push(self.record(start, count, (count == 0).then_some("insert_only")));
        }
        if records.is_empty() {
            let reason = if oversized {
                "diff_truncated"
            } else {
                "no_text_range"
            };
            records.push(self.record(0, 0, Some(reason)));
        }
        Ok((records, headers.len() > remaining))
    }
}

pub(super) fn paths(raw: &[u8]) -> Result<Vec<ChangedPath<'_>>> {
    let mut fields = raw.split_inclusive(|byte| *byte == 0);
    let mut changes = Vec::new();
    while let Some(header) = fields.next().and_then(terminated) {
        let Some(path) = fields.next().and_then(terminated) else {
            break;
        };
        let header = std::str::from_utf8(header)?
            .strip_prefix(':')
            .context("Invalid Git diff header")?;
        let fields: Vec<_> = header.split_ascii_whitespace().collect();
        let [old_mode, new_mode, old_object, new_object, status] = fields.as_slice() else {
            anyhow::bail!("Invalid Git raw diff record");
        };
        changes.push(ChangedPath {
            path,
            status,
            old_mode,
            new_mode,
            old_object,
            new_object,
        });
    }
    Ok(changes)
}

fn terminated(bytes: &[u8]) -> Option<&[u8]> {
    bytes.strip_suffix(&[0])
}

#[derive(Default)]
struct Patches<'a>(BTreeMap<Cow<'a, [u8]>, Vec<&'a [u8]>>);

impl<'a> Patches<'a> {
    fn new(patch: &'a [u8]) -> Result<Self> {
        let mut patches = Self::default();
        let mut path = None;
        for line in patch.split_inclusive(|byte| *byte == b'\n') {
            let Some(line) = line.strip_suffix(b"\n") else {
                break;
            };
            if let Some(header) = line.strip_prefix(b"diff --git ") {
                path = Some(patch_path(header)?);
            } else if let Some(header) = line.strip_prefix(b"@@ -")
                && let Some(path) = &path
            {
                let headers = patches.0.entry(path.clone()).or_default();
                if headers.len() <= MAX_RANGES {
                    headers.push(header);
                }
            }
        }
        Ok(patches)
    }

    fn headers(&self, path: &[u8]) -> &[&'a [u8]] {
        self.0.get(path).map_or(&[], Vec::as_slice)
    }
}

fn patch_path(header: &[u8]) -> Result<Cow<'_, [u8]>> {
    // Rename detection is disabled, so both sides encode the same path with
    // equally long prefixes. Splitting at the midpoint preserves literal spaces.
    let midpoint = header.len() / 2;
    let (old, new) = header.split_at(midpoint);
    let new = new
        .strip_prefix(b" ")
        .context("Invalid Git patch separator")?;
    let (old, new) = if let Some(old) = old.strip_prefix(b"\"a/") {
        (
            old.strip_suffix(b"\""),
            new.strip_prefix(b"\"b/")
                .and_then(|path| path.strip_suffix(b"\"")),
        )
    } else {
        (old.strip_prefix(b"a/"), new.strip_prefix(b"b/"))
    };
    let old = old
        .filter(|old| Some(*old) == new)
        .context("Invalid same-path Git patch")?;
    if header.starts_with(b"\"") {
        Ok(Cow::Owned(unquote(old)?))
    } else {
        Ok(Cow::Borrowed(old))
    }
}

fn unquote(path: &[u8]) -> Result<Vec<u8>> {
    let mut bytes = path.iter().copied();
    let mut decoded = Vec::new();
    while let Some(byte) = bytes.next() {
        let value = if byte == b'\\' {
            escape(&mut bytes)?
        } else {
            byte
        };
        decoded.push(value);
    }
    Ok(decoded)
}

fn escape(bytes: &mut impl Iterator<Item = u8>) -> Result<u8> {
    Ok(match bytes.next() {
        Some(b'a') => 7,
        Some(b'b') => 8,
        Some(b't') => b'\t',
        Some(b'n') => b'\n',
        Some(b'v') => 11,
        Some(b'f') => 12,
        Some(b'r') => b'\r',
        Some(byte @ (b'\\' | b'"')) => byte,
        Some(first @ b'0'..=b'3') => {
            let mut value = first - b'0';
            for _ in 0..2 {
                let digit = bytes
                    .next()
                    .filter(|byte| matches!(byte, b'0'..=b'7'))
                    .context("Invalid Git path escape")?;
                value = (value << 3) | (digit - b'0');
            }
            value
        }
        _ => anyhow::bail!("Invalid Git path escape"),
    })
}

fn old_range(header: &[u8]) -> Result<(u32, u32)> {
    let end = header
        .iter()
        .position(|byte| *byte == b' ')
        .context("Invalid Git hunk header")?;
    let range = std::str::from_utf8(&header[..end])?;
    let (start, count) = range.split_once(',').unwrap_or((range, "1"));
    Ok((start.parse()?, count.parse()?))
}
