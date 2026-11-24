//! Project captured Git changes to an outline. Graph facts only enrich proven text edits.
mod attribution;

use super::{
    Change, ChangeKind, Facet, SourceDiff, SourceStatus, Status, snapshot::Snapshot, source,
};
use crate::{
    artifact::{
        Artifact,
        revision::{SourceChanges, SourceFile, SourceSide},
    },
    model::GraphData,
};
use anyhow::Result;
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

pub struct GitComparison {
    pub changes: Vec<Change>,
    pub entry_point_search_complete: bool,
    files: Vec<SourceFile>,
    sources: BTreeMap<String, usize>,
}

impl GitComparison {
    pub fn new(
        changes: SourceChanges,
        before: Option<&Artifact>,
        after: Option<&Artifact>,
    ) -> Result<Self> {
        let before = before.map(Evidence::new).transpose()?;
        let after = after.map(Evidence::new).transpose()?;
        Self::with_evidence(changes, before.as_ref(), after.as_ref())
    }

    fn with_evidence(
        captured: SourceChanges,
        before: Option<&Evidence>,
        after: Option<&Evidence>,
    ) -> Result<Self> {
        let before = before.map(Evidence::snapshot).transpose()?;
        let after = after.map(Evidence::snapshot).transpose()?;
        let mut attribution = attribution::Attribution::new(before.as_ref(), after.as_ref());
        let mut changes = Vec::new();
        let mut sources = BTreeMap::new();
        for (index, file) in captured.files.iter().enumerate() {
            sources.insert(file.id.clone(), index);
            changes.push(file_row(file));
            for row in attribution.rows(file) {
                if sources.contains_key(&row.id) {
                    continue;
                }
                sources.insert(row.id.clone(), index);
                changes.push(row);
            }
        }
        super::sort_changes(&mut changes);
        Ok(Self {
            changes,
            entry_point_search_complete: attribution.complete(),
            files: captured.files,
            sources,
        })
    }

    pub fn source_diff(&self, id: &str) -> Option<SourceDiff<'_>> {
        let (id, &index) = self.sources.get_key_value(id)?;
        let file = &self.files[index];
        let (status, hunks, reason) = match texts(file) {
            Ok((before, after)) => (
                SourceStatus::Ready,
                source::patch(&before, &after)
                    .hunks()
                    .iter()
                    .map(source::render)
                    .collect(),
                None,
            ),
            Err(reason) => (SourceStatus::Unavailable, Vec::new(), Some(reason)),
        };
        Some(SourceDiff {
            id,
            status,
            file: &file.path,
            hunks,
            reason,
        })
    }
}

struct Evidence {
    graph: GraphData,
    missing_source: BTreeSet<usize>,
}

impl Evidence {
    fn new(artifact: &Artifact) -> Result<Self> {
        let (portable, identities) = artifact.validated_portable()?;
        Ok(Self {
            graph: portable.into_runtime(&identities)?,
            missing_source: super::missing_source(artifact),
        })
    }

    fn snapshot(&self) -> Result<Snapshot<'_>> {
        Snapshot::new(&self.graph, &self.missing_source)
    }
}

fn file_row(file: &SourceFile) -> Change {
    Change {
        id: file.id.clone(),
        kind: ChangeKind::File,
        name: file
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&file.path)
            .to_owned(),
        name_details: None,
        file: file.path.clone(),
        status: Status::between(file.before.present(), file.after.present()),
        changes_in: vec![Facet::Source],
        entry_points: Vec::new(),
        class_id: None,
        symbol: None,
        flow_available: None,
    }
}

fn normalize(text: &str) -> Cow<'_, str> {
    if text.contains("\r\n") {
        Cow::Owned(text.replace("\r\n", "\n"))
    } else {
        Cow::Borrowed(text)
    }
}

fn texts(file: &SourceFile) -> Result<(Cow<'_, str>, Cow<'_, str>), &'static str> {
    const MAX_DIFF_NEWLINES: usize = 20_000;
    fn side(source: &SourceSide) -> Result<Cow<'_, str>, &'static str> {
        match source {
            SourceSide::Absent => Ok(Cow::Borrowed("")),
            SourceSide::Text(text) => Ok(normalize(text)),
            SourceSide::Unavailable(reason) => Err(reason),
        }
    }
    if let Some(reason) = file.reason {
        return Err(reason);
    }
    let before = side(&file.before)?;
    let after = side(&file.after)?;
    let lines = before
        .bytes()
        .chain(after.bytes())
        .filter(|&byte| byte == b'\n');
    if lines.take(MAX_DIFF_NEWLINES + 1).count() > MAX_DIFF_NEWLINES {
        return Err("source_too_many_lines");
    }
    Ok((before, after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::comparison_changes::tests::snapshot;

    const BEFORE: &str =
        "class OrderService {\n// entry\nvoid Entry() { Run(); }\nint Run() { return 1; }\n}\n";
    const AFTER: &str =
        "class OrderService {\n// entry\nvoid Entry() { Run(); }\nint Run() { return 2; }\n}\n";

    fn evidence(source: &str, compiler_id: &str) -> Evidence {
        let mut graph = snapshot(source, "direct");
        graph.nodes[0].id = compiler_id.into();
        graph.entry_points.as_mut().unwrap().framework_entries[0] = compiler_id.into();
        graph.flow_inventory.as_mut().unwrap().functions[0].symbol = compiler_id.into();
        Evidence {
            graph,
            missing_source: BTreeSet::new(),
        }
    }

    fn edits(before: &str, after: &str) -> SourceChanges {
        SourceChanges {
            digest: String::new(),
            files: vec![SourceFile {
                id: "git-file-test".into(),
                path: "OrderService.cs".into(),
                before: SourceSide::Text(before.into()),
                after: SourceSide::Text(after.into()),
                reason: None,
            }],
        }
    }

    #[test]
    fn git_attribution_ignores_compiler_renumbering_and_matches_changed_method() {
        let old = evidence(BEFORE, "lambda$Entry$1");
        let new = evidence(AFTER, "lambda$Entry$8");
        let no_edits = SourceChanges {
            digest: String::new(),
            files: Vec::new(),
        };
        let unchanged = GitComparison::with_evidence(no_edits, Some(&old), Some(&new)).unwrap();
        assert!(unchanged.changes.is_empty());

        let live = AFTER.replace('\n', "\r\n");
        let change =
            GitComparison::with_evidence(edits(BEFORE, &live), Some(&old), Some(&new)).unwrap();
        assert_eq!(change.changes.len(), 3);
        let methods: Vec<_> = change
            .changes
            .iter()
            .filter(|row| row.kind == ChangeKind::Method)
            .collect();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].symbol.as_deref(), Some("run"));
        assert_eq!(methods[0].flow_available, Some(true));
        let diff = change.source_diff(&methods[0].id).unwrap();
        assert_eq!(diff.hunks.len(), 1);
        assert!(diff.hunks[0].lines.iter().any(|line| {
            line.kind == super::super::LineKind::Insert && line.text.contains("return 2")
        }));
    }

    #[test]
    fn git_attribution_keeps_file_diff_without_matching_snapshots() {
        let stale = evidence(&BEFORE.replace("// entry", "// stale"), "lambda$Entry$1");
        let change =
            GitComparison::with_evidence(edits(BEFORE, AFTER), Some(&stale), Some(&stale)).unwrap();
        assert_eq!(change.changes.len(), 1);
        assert_eq!(change.changes[0].kind, ChangeKind::File);
        let diff = change.source_diff(&change.changes[0].id).unwrap();
        assert_eq!(diff.status, SourceStatus::Ready);
        assert_eq!(diff.hunks.len(), 1);
    }
}
