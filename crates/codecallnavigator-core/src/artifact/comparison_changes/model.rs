use crate::model::{Node, Visibility};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Class,
    Method,
    File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Added,
    Modified,
    Removed,
}

impl Status {
    pub(super) fn between(before: bool, after: bool) -> Self {
        match (before, after) {
            (false, true) => Self::Added,
            (true, false) => Self::Removed,
            _ => Self::Modified,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Facet {
    Declaration,
    Members,
    Flow,
    Source,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Certainty {
    Possible,
    Direct,
}

#[derive(Debug, Serialize)]
pub struct EntryPoint {
    pub id: String,
    pub name: String,
    pub before: Option<Certainty>,
    pub after: Option<Certainty>,
}

#[derive(Debug, Serialize)]
pub struct Change {
    pub id: String,
    pub kind: ChangeKind,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_details: Option<NameDetails>,
    pub file: String,
    pub status: Status,
    pub changes_in: Vec<Facet>,
    pub entry_points: Vec<EntryPoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_available: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NameDetails {
    pub label: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
}

impl NameDetails {
    pub(super) fn method(node: &Node) -> Self {
        Self {
            label: node.label.clone(),
            description: node.signature.clone(),
            visibility: node.visibility,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Ready,
    Unavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineKind {
    Context,
    Delete,
    Insert,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Range {
    pub start: usize,
    pub count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Hunk {
    pub before: Range,
    pub after: Range,
    pub lines: Vec<Line>,
}

#[derive(Debug, Serialize)]
pub struct SourceDiff<'a> {
    pub id: &'a str,
    pub status: SourceStatus,
    pub file: &'a str,
    pub hunks: Vec<Hunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
}

impl Hunk {
    /// Returns whether this side's changed lines overlap, and if their edit run touches both sides.
    pub fn changed_at(
        &self,
        before: bool,
        range: &std::ops::RangeInclusive<usize>,
    ) -> Option<bool> {
        let in_side = if before {
            LineKind::Delete
        } else {
            LineKind::Insert
        };
        let other_side = if before {
            LineKind::Insert
        } else {
            LineKind::Delete
        };
        let mut index = if before { self.before } else { self.after }.start;
        self.lines
            .split_inclusive(|line| line.kind == LineKind::Context)
            .find_map(|edit| {
                let changed = edit.iter().filter(|line| line.kind == in_side).count();
                let end = index.saturating_add(changed);
                let overlap = (index..end).any(|line| range.contains(&line));
                index = end.saturating_add(usize::from(
                    edit.last()
                        .is_some_and(|line| line.kind == LineKind::Context),
                ));
                overlap.then(|| edit.iter().any(|line| line.kind == other_side))
            })
    }
}
