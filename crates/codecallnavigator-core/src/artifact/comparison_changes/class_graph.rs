//! Compare incoming and outgoing saved calls for a selected class neighborhood.
use super::{Class, Snapshot, class_neighborhood::Neighborhood, missing_source};
use crate::{
    artifact::Artifact,
    model::{TargetResolution, Visibility},
    query::{DEFAULT_CLASS_COMPARISON_DEPTH, validate_class_comparison_depth},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ClassGraph {
    pub root: String,
    pub nodes: Vec<ClassNode>,
    pub links: Vec<ClassLink>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ClassNode {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_visibility: Option<Visibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_visibility: Option<Visibility>,
    pub before: bool,
    pub after: bool,
    pub status: GraphChange,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ClassLink {
    pub from: String,
    pub to: String,
    pub before: BTreeMap<TargetResolution, u64>,
    pub after: BTreeMap<TargetResolution, u64>,
    pub status: GraphChange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphChange {
    Unchanged,
    Added,
    Removed,
    Modified,
}

impl GraphChange {
    fn presence(before: bool, after: bool) -> Self {
        match (before, after) {
            (false, true) => Self::Added,
            (true, false) => Self::Removed,
            _ => Self::Unchanged,
        }
    }
}

impl ClassGraph {
    pub fn between(before: &Artifact, after: &Artifact, id: &str) -> Result<Option<Self>> {
        Self::between_at_depth(before, after, id, DEFAULT_CLASS_COMPARISON_DEPTH)
    }

    pub fn between_at_depth(
        before: &Artifact,
        after: &Artifact,
        id: &str,
        depth: u32,
    ) -> Result<Option<Self>> {
        validate_class_comparison_depth(depth)?;
        with_snapshots(before, after, |before, after| {
            graph_at_depth(before, after, id, depth)
        })
    }
}

pub(super) fn with_snapshots<T>(
    before: &Artifact,
    after: &Artifact,
    project: impl FnOnce(&Snapshot<'_>, &Snapshot<'_>) -> T,
) -> Result<T> {
    let (old, old_ids) = before.validated_portable()?;
    let (new, new_ids) = after.validated_portable()?;
    let old = old.into_runtime(&old_ids)?;
    let new = new.into_runtime(&new_ids)?;
    let old_missing = missing_source(before);
    let new_missing = missing_source(after);
    let old = Snapshot::new(&old, &old_missing)?;
    let new = Snapshot::new(&new, &new_missing)?;
    Ok(project(&old, &new))
}

#[cfg(test)]
pub(super) fn graph(before: &Snapshot<'_>, after: &Snapshot<'_>, id: &str) -> Option<ClassGraph> {
    graph_at_depth(before, after, id, DEFAULT_CLASS_COMPARISON_DEPTH)
}

pub(super) fn graph_at_depth(
    before: &Snapshot<'_>,
    after: &Snapshot<'_>,
    id: &str,
    depth: u32,
) -> Option<ClassGraph> {
    let class = class_for_id(before, id).or_else(|| class_for_id(after, id))?;
    let old = Neighborhood::new(before, &class, depth);
    let new = Neighborhood::new(after, &class, depth);
    Some(ClassGraph {
        root: id.to_owned(),
        nodes: nodes(&old, &new),
        links: links(&old, &new),
    })
}

pub(super) fn class_for_id(snapshot: &Snapshot<'_>, id: &str) -> Option<Class> {
    snapshot
        .classes
        .keys()
        .find(|class| class.id() == id)
        .cloned()
        .or_else(|| {
            snapshot
                .files
                .keys()
                .filter_map(|file| snapshot.single_class(*file))
                .find(|class| class.id() == id)
        })
}

fn nodes(old: &Neighborhood, new: &Neighborhood) -> Vec<ClassNode> {
    let ids: BTreeSet<_> = old.names.keys().chain(new.names.keys()).collect();
    ids.into_iter()
        .map(|id| {
            let before = old.names.get(id);
            let after = new.names.get(id);
            let details = after
                .or(before)
                .expect("the class is present on at least one side");
            ClassNode {
                id: id.clone(),
                name: details.description.clone(),
                short_name: Some(details.label.clone()),
                before_visibility: before.and_then(|name| name.visibility),
                after_visibility: after.and_then(|name| name.visibility),
                before: before.is_some(),
                after: after.is_some(),
                status: GraphChange::presence(before.is_some(), after.is_some()),
            }
        })
        .collect()
}

fn links(old: &Neighborhood, new: &Neighborhood) -> Vec<ClassLink> {
    let keys: BTreeSet<_> = old.links.keys().chain(new.links.keys()).collect();
    keys.into_iter()
        .map(|key| {
            let before = old.links.get(key);
            let after = new.links.get(key);
            let status = match (before, after) {
                (Some(previous), Some(current)) if previous != current => GraphChange::Modified,
                _ => GraphChange::presence(before.is_some(), after.is_some()),
            };
            ClassLink {
                from: key.0.clone(),
                to: key.1.clone(),
                before: before.cloned().unwrap_or_default(),
                after: after.cloned().unwrap_or_default(),
                status,
            }
        })
        .collect()
}
