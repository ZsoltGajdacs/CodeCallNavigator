use super::{Change, Hunk, Line, LineKind, Range, SourceDiff, SourceStatus};
use crate::architecture::FileFacts;
use diffy::{DiffOptions, HunkRange, Line as DiffLine, Patch};

#[derive(Clone, Copy, Debug)]
pub(super) enum Side {
    Missing,
    Unavailable,
    Recorded(usize),
}

pub(super) struct Pair {
    pub before: Side,
    pub after: Side,
}

impl Side {
    fn text(self, facts: &[FileFacts]) -> Option<&str> {
        match self {
            Self::Missing => Some(""),
            Self::Unavailable => None,
            Self::Recorded(index) => Some(&facts[index].content),
        }
    }
}

pub(super) fn fits(before: &str, after: &str) -> bool {
    let max = crate::flow::MAX_SOURCE_BYTES as usize;
    before.len() <= max && after.len() <= max
}

pub(super) fn patch<'a>(before: &'a str, after: &'a str) -> Patch<'a, str> {
    DiffOptions::new().create_patch(before, after)
}

pub(super) fn changed_lines<'a>(
    patch: &'a Patch<'_, str>,
    before: bool,
) -> impl Iterator<Item = usize> + 'a {
    patch.hunks().iter().flat_map(move |hunk| {
        let mut at = if before {
            hunk.old_range()
        } else {
            hunk.new_range()
        }
        .start();
        hunk.lines()
            .iter()
            .filter_map(move |line| match (line, before) {
                (DiffLine::Context(_), _) => {
                    at += 1;
                    None
                }
                (DiffLine::Delete(_), true) | (DiffLine::Insert(_), false) => {
                    let changed = at;
                    at += 1;
                    Some(changed)
                }
                _ => None,
            })
    })
}

impl Pair {
    pub fn diff<'a>(
        &self,
        change: &'a Change,
        before: &'a [FileFacts],
        after: &'a [FileFacts],
    ) -> SourceDiff<'a> {
        let available = self.before.text(before).zip(self.after.text(after));
        let (status, hunks, reason) = match available {
            Some((before, after)) if fits(before, after) => (
                SourceStatus::Ready,
                patch(before, after).hunks().iter().map(render).collect(),
                None,
            ),
            Some(_) => (
                SourceStatus::Unavailable,
                Vec::new(),
                Some("saved_source_too_large"),
            ),
            None => (
                SourceStatus::Unavailable,
                Vec::new(),
                Some("missing_saved_source"),
            ),
        };
        SourceDiff {
            id: &change.id,
            status,
            file: &change.file,
            hunks,
            reason,
        }
    }
}

pub(super) fn render(hunk: &diffy::Hunk<'_, str>) -> Hunk {
    let lines = hunk
        .lines()
        .iter()
        .map(|line| {
            let (kind, text) = match *line {
                DiffLine::Context(text) => (LineKind::Context, text),
                DiffLine::Delete(text) => (LineKind::Delete, text),
                DiffLine::Insert(text) => (LineKind::Insert, text),
            };
            Line {
                kind,
                text: text.to_owned(),
            }
        })
        .collect();
    Hunk {
        before: range(hunk.old_range()),
        after: range(hunk.new_range()),
        lines,
    }
}

fn range(range: HunkRange) -> Range {
    Range {
        start: range.start(),
        count: range.len(),
    }
}
