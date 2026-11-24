//! Saved source for a class node in a bound comparison.
use super::{
    Artifact, Hunk,
    class::Class,
    class_graph,
    class_neighborhood::Neighborhood,
    display_path,
    snapshot::{FileId, Snapshot},
    source,
};
use crate::query::{ComparisonSide, validate_class_comparison_depth};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const EXCERPT_LINES: usize = 12;
const EXCERPT_BYTES: usize = 4096;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClassSource {
    Diff {
        file: String,
        hunks: Vec<Hunk>,
    },
    Excerpt {
        file: String,
        start: usize,
        text: String,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ClassSourceTarget<'a> {
    pub root: &'a str,
    pub node: &'a str,
    pub side: ComparisonSide,
    pub depth: u32,
}

impl ClassSource {
    pub fn between(
        before: &Artifact,
        after: &Artifact,
        target: ClassSourceTarget<'_>,
    ) -> Result<Option<Self>> {
        validate_class_comparison_depth(target.depth)?;
        class_graph::with_snapshots(before, after, |before, after| {
            preview(before, after, target)
        })
    }
}

fn preview(
    before: &Snapshot<'_>,
    after: &Snapshot<'_>,
    ClassSourceTarget {
        root,
        node,
        side,
        depth,
    }: ClassSourceTarget<'_>,
) -> Option<ClassSource> {
    let hovered = match side {
        ComparisonSide::Before => before,
        ComparisonSide::After => after,
    };
    let root = class_graph::class_for_id(before, root)
        .or_else(|| class_graph::class_for_id(after, root))?;
    let neighborhood = Neighborhood::new(hovered, &root, depth);
    if !neighborhood.names.contains_key(node) {
        return None;
    }
    let class = class_graph::class_for_id(before, node)
        .or_else(|| class_graph::class_for_id(after, node))?;
    let files = owned_files(hovered, &class);
    let diff = files.iter().find_map(|&file| {
        let hunks = file_hunks(before, after, file, &class)
            .unwrap_or_else(|| member_hunks(before, after, file, &class));
        (!hunks.is_empty()).then(|| ClassSource::Diff {
            file: display_path(file.2).into(),
            hunks,
        })
    });
    Some(
        diff.or_else(|| excerpt(hovered, &class, &files))
            .unwrap_or_else(|| ClassSource::Unavailable {
                reason: "No attributable saved source for this class".into(),
            }),
    )
}

fn owned_files<'a>(snapshot: &Snapshot<'a>, class: &Class) -> BTreeSet<FileId<'a>> {
    let files: BTreeSet<_> = members(snapshot, class).map(FileId::of_node).collect();
    if !files.is_empty() {
        return files;
    }
    snapshot
        .files
        .keys()
        .copied()
        .filter(|&file| sole_owner(snapshot, file, class))
        .collect()
}

fn members<'a, 'b>(
    snapshot: &'b Snapshot<'a>,
    class: &Class,
) -> impl Iterator<Item = &'a crate::model::Node> + 'b {
    snapshot
        .classes
        .get(class)
        .into_iter()
        .flatten()
        .map(|&index| &snapshot.graph.nodes[index])
}

fn sole_owner(snapshot: &Snapshot<'_>, file: FileId<'_>, class: &Class) -> bool {
    snapshot
        .single_class(file)
        .is_some_and(|owner| owner.can_confirm(class))
        && snapshot
            .file_nodes
            .get(&file)
            .into_iter()
            .flatten()
            .all(|&id| Class::from_node(&snapshot.graph.nodes[id]).as_ref() == Some(class))
}

fn owned_file_text<'a>(
    snapshot: &Snapshot<'a>,
    file: FileId<'_>,
    class: &Class,
) -> Option<&'a str> {
    if matches!(snapshot.source(file), source::Side::Missing) || sole_owner(snapshot, file, class) {
        snapshot.text(file)
    } else {
        None
    }
}

fn file_hunks(
    before: &Snapshot<'_>,
    after: &Snapshot<'_>,
    file: FileId<'_>,
    class: &Class,
) -> Option<Vec<Hunk>> {
    let old = owned_file_text(before, file, class)?;
    let new = owned_file_text(after, file, class)?;
    Some(if old != new && source::fits(old, new) {
        source::patch(old, new)
            .hunks()
            .iter()
            .map(source::render)
            .collect()
    } else {
        Vec::new()
    })
}

/// Shared files can still prove edits within exact saved spans of matching methods.
fn member_hunks(
    before: &Snapshot<'_>,
    after: &Snapshot<'_>,
    file: FileId<'_>,
    class: &Class,
) -> Vec<Hunk> {
    let mut hunks: Vec<_> = members(before, class)
        .filter(|node| FileId::of_node(node) == file)
        .filter_map(|old| {
            let new = after.node(&old.id)?;
            if FileId::of_node(new) != file || Class::from_node(new).as_ref() != Some(class) {
                return None;
            }
            let old = member_source(before, &old.id, false)?;
            let new = member_source(after, &new.id, false)?;
            (old.text != new.text).then(|| offset_hunks(old, new))
        })
        .flatten()
        .collect();
    hunks.sort_by_key(|hunk| (hunk.before.start, hunk.after.start));
    hunks
}

struct Snippet<'a> {
    line_offset: usize,
    text: &'a str,
}

fn offset_hunks(old: Snippet<'_>, new: Snippet<'_>) -> Vec<Hunk> {
    source::patch(old.text, new.text)
        .hunks()
        .iter()
        .map(|hunk| {
            let mut hunk = source::render(hunk);
            hunk.before.start += old.line_offset;
            hunk.after.start += new.line_offset;
            hunk
        })
        .collect()
}

fn member_source<'a>(
    snapshot: &'a Snapshot<'_>,
    id: &str,
    expand_line: bool,
) -> Option<Snippet<'a>> {
    let (content, span) = snapshot.flow_source(id)?;
    let offset = if expand_line {
        content[..span.start]
            .rfind('\n')
            .map_or(0, |newline| newline + 1)
    } else {
        span.start
    };
    let text = content.get(offset..span.end)?;
    let line_offset = content.as_bytes()[..offset]
        .iter()
        .filter(|&&byte| byte == b'\n')
        .count();
    Some(Snippet { line_offset, text })
}

fn excerpt(
    snapshot: &Snapshot<'_>,
    class: &Class,
    files: &BTreeSet<FileId<'_>>,
) -> Option<ClassSource> {
    members(snapshot, class)
        .filter_map(|node| {
            let source = member_source(
                snapshot,
                &node.id,
                sole_owner(snapshot, FileId::of_node(node), class),
            )?;
            Some((&node.file[..], source.line_offset + 1, source.text))
        })
        .min_by_key(|(file, start, _)| (*file, *start))
        .or_else(|| {
            files.iter().find_map(|&file| {
                let text = sole_owner(snapshot, file, class)
                    .then(|| snapshot.text(file))
                    .flatten()?;
                Some((file.2, 1, text))
            })
        })
        .and_then(|(file, start, source)| {
            let mut end = source
                .split_inclusive('\n')
                .take(EXCERPT_LINES)
                .map(str::len)
                .sum::<usize>();
            end = source.floor_char_boundary(end.min(EXCERPT_BYTES));
            (end > 0).then(|| ClassSource::Excerpt {
                file: display_path(file).into(),
                start,
                text: source[..end].trim_end_matches(['\r', '\n']).into(),
            })
        })
}

#[cfg(test)]
#[path = "class_source_tests.rs"]
mod tests;
