//! Readable, snapshot-bound code changes and on-demand unified source.
mod class;
mod class_graph;
mod class_source;
mod declarations;
mod git;
mod methods;
mod model;
mod reachability;
mod snapshot;
mod source;
#[cfg(test)]
mod tests;

use super::Artifact;
use crate::{
    architecture::FileFacts,
    model::{GraphData, Node},
};
use anyhow::Result;
use class::Class;
pub use class_graph::{ClassGraph, ClassLink, ClassNode, GraphChange};
pub use class_source::*;
pub use git::GitComparison;
pub use model::*;
use reachability::Search;
use snapshot::{FileId, Snapshot};
use std::collections::{BTreeMap, BTreeSet};

pub struct Comparison {
    pub changes: Vec<Change>,
    pub entry_point_search_complete: bool,
    sources: BTreeMap<String, source::Pair>,
    before_files: Vec<FileFacts>,
    after_files: Vec<FileFacts>,
}

struct ClassChanges<'a> {
    file: FileId<'a>,
    facets: BTreeSet<Facet>,
}

impl Comparison {
    pub fn new(before: &Artifact, after: &Artifact) -> Result<Self> {
        let (old, old_ids) = before.validated_portable()?;
        let (new, new_ids) = after.validated_portable()?;
        Self::build(
            old.into_runtime(&old_ids)?,
            new.into_runtime(&new_ids)?,
            &missing_source(before),
            &missing_source(after),
        )
    }

    #[cfg(test)]
    fn from_graphs(before: GraphData, after: GraphData) -> Result<Self> {
        Self::build(before, after, &BTreeSet::new(), &BTreeSet::new())
    }

    fn build(
        before: GraphData,
        after: GraphData,
        old_missing_source: &BTreeSet<usize>,
        new_missing_source: &BTreeSet<usize>,
    ) -> Result<Self> {
        let old = Snapshot::new(&before, old_missing_source)?;
        let new = Snapshot::new(&after, new_missing_source)?;
        let mut outline = Outline::new(&old, &new);
        outline.collect();
        let complete = outline.before_routes.complete && outline.after_routes.complete;
        Ok(Self {
            changes: outline.rows,
            entry_point_search_complete: complete,
            sources: outline.sources,
            before_files: before
                .architecture
                .map_or_else(Vec::new, |facts| facts.files),
            after_files: after
                .architecture
                .map_or_else(Vec::new, |facts| facts.files),
        })
    }

    pub fn source_diff(&self, id: &str) -> Option<SourceDiff<'_>> {
        let row = self.changes.iter().find(|row| row.id == id)?;
        let source = self.sources.get(id)?;
        Some(source.diff(row, &self.before_files, &self.after_files))
    }
}

struct Outline<'a, 'snapshot> {
    before: &'a Snapshot<'snapshot>,
    after: &'a Snapshot<'snapshot>,
    before_routes: Search<'snapshot>,
    after_routes: Search<'snapshot>,
    classes: BTreeMap<Class, ClassChanges<'snapshot>>,
    rows: Vec<Change>,
    sources: BTreeMap<String, source::Pair>,
}

impl<'a, 'snapshot> Outline<'a, 'snapshot> {
    fn new(before: &'a Snapshot<'snapshot>, after: &'a Snapshot<'snapshot>) -> Self {
        Self {
            before,
            after,
            before_routes: Search::new(before.graph),
            after_routes: Search::new(after.graph),
            classes: BTreeMap::new(),
            rows: Vec::new(),
            sources: BTreeMap::new(),
        }
    }

    fn collect(&mut self) {
        let methods = methods::changes(self.before, self.after);
        let changed: BTreeSet<_> = methods
            .iter()
            .map(|method| method.node().id.as_str())
            .collect();
        for file in declarations::changed_files(self.before, self.after, &changed) {
            if let Some(class) = declarations::owner(self.before, self.after, file) {
                if self
                    .classes
                    .get(&class)
                    .is_some_and(|existing| existing.file != file)
                {
                    self.add_file(file);
                } else {
                    self.class_facet(class, file, Facet::Declaration);
                }
            } else {
                self.add_file(file);
            }
        }
        for method in methods {
            self.add_method(method);
        }
        for (class, change) in std::mem::take(&mut self.classes) {
            self.add_class(class, change);
        }
        sort_changes(&mut self.rows);
    }

    fn class_facet(&mut self, class: Class, file: FileId<'snapshot>, facet: Facet) {
        self.classes
            .entry(class)
            .or_insert_with(|| ClassChanges {
                file,
                facets: BTreeSet::new(),
            })
            .facets
            .insert(facet);
    }

    fn add_method(&mut self, method: methods::Method<'snapshot>) {
        let node = method.node();
        let file = FileId::of_node(node);
        let class = Class::from_node(node);
        let class_id = class.as_ref().map(Class::id);
        if let Some(class) = class {
            self.class_facet(class, file, Facet::Members);
        }
        let entry_points = self.routes(
            self.before.symbols.get(node.id.as_str()).copied(),
            self.after.symbols.get(node.id.as_str()).copied(),
        );
        let row = Change {
            id: stable_id("method", &[&node.id]),
            kind: ChangeKind::Method,
            name: display_signature(node),
            name_details: Some(NameDetails::method(node)),
            file: display_path(file.2).to_owned(),
            status: method.status(),
            changes_in: method.facts,
            entry_points,
            class_id,
            symbol: Some(node.id.clone()),
            flow_available: Some(
                self.before.flow_source(&node.id).is_some()
                    || self.after.flow_source(&node.id).is_some(),
            ),
        };
        let old_file = method.before.map_or(file, FileId::of_node);
        let new_file = method.after.map_or(file, FileId::of_node);
        self.insert(row, old_file, new_file);
    }

    fn add_class(&mut self, class: Class, facts: ClassChanges<'snapshot>) {
        let old_members = self
            .before
            .classes
            .get(&class)
            .into_iter()
            .flatten()
            .copied();
        let new_members = self
            .after
            .classes
            .get(&class)
            .into_iter()
            .flatten()
            .copied();
        let entry_points = self.routes(old_members, new_members);
        let after_present = self.after.has_class(&class);
        let naming_snapshot = if after_present {
            self.after
        } else {
            self.before
        };
        let row = Change {
            id: class.id(),
            kind: ChangeKind::Class,
            name: class.name().to_owned(),
            name_details: Some(class.name_details(naming_snapshot.class_visibility(&class))),
            file: display_path(facts.file.2).to_owned(),
            status: Status::between(self.before.has_class(&class), after_present),
            changes_in: facts.facets.into_iter().collect(),
            entry_points,
            class_id: None,
            symbol: None,
            flow_available: None,
        };
        self.insert(row, facts.file, facts.file);
    }

    fn add_file(&mut self, file: FileId<'snapshot>) {
        let row = Change {
            id: stable_id("file", &[file.0, file.1, file.2]),
            kind: ChangeKind::File,
            name: file.2.rsplit('/').next().unwrap_or(file.2).to_owned(),
            name_details: None,
            file: display_path(file.2).to_owned(),
            status: Status::between(
                self.before.file(file).is_some(),
                self.after.file(file).is_some(),
            ),
            changes_in: vec![Facet::Source],
            entry_points: Vec::new(),
            class_id: None,
            symbol: None,
            flow_available: None,
        };
        self.insert(row, file, file);
    }

    fn routes(
        &mut self,
        before: impl IntoIterator<Item = usize>,
        after: impl IntoIterator<Item = usize>,
    ) -> Vec<EntryPoint> {
        reachability::routes(
            Some(&mut self.before_routes),
            before,
            Some(&mut self.after_routes),
            after,
        )
    }

    fn insert(&mut self, change: Change, before: FileId<'_>, after: FileId<'_>) {
        self.sources.insert(
            change.id.clone(),
            source::Pair {
                before: self.before.source(before),
                after: self.after.source(after),
            },
        );
        self.rows.push(change);
    }
}

fn missing_source(artifact: &Artifact) -> BTreeSet<usize> {
    artifact.graph["architecture"]["files"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, file)| file.get("content").is_none().then_some(index))
        .collect()
}

fn stable_id(kind: &str, parts: &[&str]) -> String {
    let mut digest = blake3::Hasher::new();
    for part in parts {
        digest.update(&(part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
    format!("{kind}-{}", digest.finalize().to_hex())
}

fn sort_changes(rows: &mut [Change]) {
    fn order(row: &Change) -> (bool, u8, &str, &str, &str) {
        let kind = match row.kind {
            ChangeKind::Class => 0,
            ChangeKind::Method => 1,
            ChangeKind::File => 2,
        };
        (
            row.entry_points.is_empty(),
            kind,
            &row.file,
            &row.name,
            &row.id,
        )
    }
    rows.sort_by(|left, right| order(left).cmp(&order(right)));
}

fn display_path(path: &str) -> &str {
    path.strip_prefix("@repo/").unwrap_or(path)
}

fn display_signature(node: &Node) -> String {
    let parameters = node.signature.find('(').zip(node.signature.rfind(')'));
    parameters.filter(|(start, end)| start < end).map_or_else(
        || node.label.clone(),
        |(start, end)| format!("{}{}", node.label, &node.signature[start..=end]),
    )
}
