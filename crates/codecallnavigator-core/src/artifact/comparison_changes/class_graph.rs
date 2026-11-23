//! A selected class and its immediate incoming and outgoing saved calls.
use super::{Class, Snapshot, missing_source};
use crate::{
    artifact::Artifact,
    model::{RelationshipKind, TargetResolution},
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
        with_snapshots(before, after, |before, after| graph(before, after, id))
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

pub(super) fn graph(before: &Snapshot<'_>, after: &Snapshot<'_>, id: &str) -> Option<ClassGraph> {
    let class = class_for_id(before, id).or_else(|| class_for_id(after, id))?;
    let old = Neighborhood::new(before, &class, id);
    let new = Neighborhood::new(after, &class, id);
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

type LinkKey = (String, String);
type Calls = BTreeMap<TargetResolution, u64>;

#[derive(Default)]
struct Neighborhood {
    names: BTreeMap<String, String>,
    links: BTreeMap<LinkKey, Calls>,
}

impl Neighborhood {
    fn new(snapshot: &Snapshot<'_>, root: &Class, id: &str) -> Self {
        let mut neighborhood = Self::default();
        if snapshot.has_class(root) {
            neighborhood
                .names
                .insert(id.to_owned(), root.owner.container.clone());
        }
        let Some(members) = snapshot.classes.get(root) else {
            return neighborhood;
        };
        let members: BTreeSet<_> = members.iter().copied().collect();
        for edge in &snapshot.graph.edges {
            let from = edge.from as usize;
            let to = edge.to as usize;
            let (neighbor, incoming) = match (members.contains(&from), members.contains(&to)) {
                (true, false) => (to, false),
                (false, true) => (from, true),
                _ => continue,
            };
            let classification = edge.classification();
            if classification.kind != RelationshipKind::Call {
                continue;
            }
            let Some(class) = Class::from_node(&snapshot.graph.nodes[neighbor]) else {
                continue;
            };
            let class_id = class.id();
            neighborhood
                .names
                .entry(class_id.clone())
                .or_insert(class.owner.container);
            let link = if incoming {
                (class_id, id.to_owned())
            } else {
                (id.to_owned(), class_id)
            };
            *neighborhood
                .links
                .entry(link)
                .or_default()
                .entry(classification.resolution)
                .or_default() += u64::from(edge.count);
        }
        neighborhood
    }
}

fn nodes(old: &Neighborhood, new: &Neighborhood) -> Vec<ClassNode> {
    let ids: BTreeSet<_> = old.names.keys().chain(new.names.keys()).collect();
    ids.into_iter()
        .map(|id| {
            let before = old.names.get(id);
            let after = new.names.get(id);
            ClassNode {
                id: id.clone(),
                name: after.or(before).cloned().unwrap_or_default(),
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
