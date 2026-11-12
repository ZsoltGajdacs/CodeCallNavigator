//! Full portable-artifact differences, independent of retrieval and presentation.
use super::{Artifact, canonicalization::sort_graph};
use anyhow::Result;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const VERSION: u32 = 1;
pub const MAX_RESPONSE_BYTES: usize = 24 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Add,
    Remove,
    Replace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Graph,
    Artifact,
}

/// Paths are relative to the identified entity. Absent values are omitted;
/// a present JSON null is retained as `Some(Value::Null)`.
#[derive(Clone, Debug, Serialize)]
pub struct Change {
    pub scope: Scope,
    pub section: String,
    pub entity: String,
    pub operation: Operation,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<Value>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Summary {
    pub changed_entities: usize,
    pub records: usize,
    pub added: usize,
    pub removed: usize,
    pub replaced: usize,
}

#[derive(Debug, Serialize)]
pub struct Difference {
    pub graph_equal: bool,
    pub metadata_equal: bool,
    pub changes: Vec<Change>,
    pub summary: BTreeMap<String, Summary>,
}

pub fn compare(before: &Artifact, after: &Artifact) -> Result<Difference> {
    before.validate()?;
    after.validate()?;
    let mut before_graph = before.graph.clone();
    let mut after_graph = after.graph.clone();
    sort_graph(&mut before_graph)?;
    sort_graph(&mut after_graph)?;
    let mut changes = Vec::new();
    for section in keys(Some(&before_graph), Some(&after_graph)) {
        walk(
            Location {
                scope: Scope::Graph,
                section: &section,
                entity: "",
                path: "",
            },
            before_graph.get(&section),
            after_graph.get(&section),
            &mut changes,
        );
    }
    for entity in before
        .extensions
        .keys()
        .chain(after.extensions.keys())
        .collect::<BTreeSet<_>>()
    {
        walk(
            Location {
                scope: Scope::Artifact,
                section: "extensions",
                entity,
                path: "",
            },
            before.extensions.get(entity),
            after.extensions.get(entity),
            &mut changes,
        );
    }
    changes.sort_by(|a, b| {
        (a.scope, &a.section, &a.entity, &a.path, a.operation).cmp(&(
            b.scope,
            &b.section,
            &b.entity,
            &b.path,
            b.operation,
        ))
    });
    let graph_equal = changes.iter().all(metadata);
    let metadata_equal = changes.iter().all(|c| !metadata(c));
    let summary = summarize(&changes);
    Ok(Difference {
        graph_equal,
        metadata_equal,
        changes,
        summary,
    })
}

fn metadata(change: &Change) -> bool {
    change.scope == Scope::Artifact || change.section == "analysis"
}

fn keys(before: Option<&Value>, after: Option<&Value>) -> BTreeSet<String> {
    before
        .into_iter()
        .chain(after)
        .filter_map(Value::as_object)
        .flat_map(|object| object.keys().cloned())
        .collect()
}

fn pointer(path: &str, field: &str) -> String {
    format!("{path}/{}", field.replace('~', "~0").replace('/', "~1"))
}

#[derive(Clone, Copy)]
struct Location<'a> {
    scope: Scope,
    section: &'a str,
    entity: &'a str,
    path: &'a str,
}

fn walk(
    location: Location<'_>,
    before: Option<&Value>,
    after: Option<&Value>,
    changes: &mut Vec<Change>,
) {
    let Location {
        scope,
        section,
        entity,
        path,
    } = location;
    if before == after {
        return;
    }
    if let (Some(Value::Object(_)), Some(Value::Object(_))) = (before, after) {
        for key in keys(before, after) {
            walk(
                Location {
                    scope,
                    section,
                    entity,
                    path: &pointer(path, &key),
                },
                before.and_then(|v| v.get(&key)),
                after.and_then(|v| v.get(&key)),
                changes,
            );
        }
    } else if let (Some(Value::Array(old)), Some(Value::Array(new)), Some(policy)) = (
        before,
        after,
        entity
            .is_empty()
            .then(|| collection(section, path))
            .flatten(),
    ) {
        compare_collection(CollectionContext { location, policy }, old, new, changes);
    } else {
        let operation = match (before, after) {
            (None, _) => Operation::Add,
            (_, None) => Operation::Remove,
            _ => Operation::Replace,
        };
        changes.push(Change {
            scope,
            section: section.into(),
            entity: entity.into(),
            operation,
            path: path.into(),
            before: before.cloned(),
            after: after.cloned(),
        });
    }
}

#[derive(Clone, Copy)]
enum Collection {
    Nodes,
    Edges,
    Functions,
    Files,
    Values,
}

fn collection(section: &str, path: &str) -> Option<Collection> {
    match (section, path) {
        ("nodes", "") => Some(Collection::Nodes),
        ("edges", "") => Some(Collection::Edges),
        ("flow_inventory", "/functions") => Some(Collection::Functions),
        ("architecture", "/files") => Some(Collection::Files),
        ("entry_points", "/candidates" | "/application_starts" | "/framework_entries")
        | ("boundaries", "/routes" | "/uses")
        | ("diagnostics" | "roots" | "identity_roots" | "folder_project_builds", "") => {
            Some(Collection::Values)
        }
        _ => None,
    }
}

fn identity(value: &Value, policy: Collection) -> String {
    match policy {
        Collection::Nodes => value["id"].as_str().unwrap_or_default().into(),
        Collection::Edges => json!([value["from"], value["to"], value["kind"]]).to_string(),
        Collection::Functions => value["symbol"].as_str().unwrap_or_default().into(),
        Collection::Files => json!([value["build"], value["project"], value["file"]]).to_string(),
        Collection::Values => value.to_string(),
    }
}

fn groups(values: &[Value], policy: Collection) -> BTreeMap<String, Value> {
    let mut grouped = BTreeMap::<String, Vec<Value>>::new();
    for value in values {
        grouped
            .entry(identity(value, policy))
            .or_default()
            .push(value.clone());
    }
    grouped
        .into_iter()
        .map(|(key, mut values)| {
            values.sort_by_cached_key(Value::to_string);
            let value = if matches!(
                policy,
                Collection::Nodes | Collection::Functions | Collection::Files
            ) && values.len() == 1
            {
                values.remove(0)
            } else {
                Value::Array(values)
            };
            (key, value)
        })
        .collect()
}

struct CollectionContext<'a> {
    location: Location<'a>,
    policy: Collection,
}

fn compare_collection(
    context: CollectionContext<'_>,
    before: &[Value],
    after: &[Value],
    changes: &mut Vec<Change>,
) {
    let CollectionContext {
        location:
            Location {
                scope,
                section,
                path,
                ..
            },
        policy,
    } = context;
    let before = groups(before, policy);
    let after = groups(after, policy);
    for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        let entity = if matches!(policy, Collection::Values) {
            format!("{path}:{key}")
        } else {
            key.clone()
        };
        walk(
            Location {
                scope,
                section,
                entity: &entity,
                path: "",
            },
            before.get(key),
            after.get(key),
            changes,
        );
    }
}

fn summarize(changes: &[Change]) -> BTreeMap<String, Summary> {
    let mut summary = BTreeMap::<String, Summary>::new();
    let mut entities = BTreeSet::new();
    for change in changes {
        let section_path = match change.scope {
            Scope::Graph => pointer("/graph", &change.section),
            Scope::Artifact => pointer("", &change.entity),
        };
        let counts = summary.entry(section_path).or_default();
        counts.records += 1;
        if entities.insert((change.scope, &change.section, &change.entity)) {
            counts.changed_entities += 1;
        }
        match change.operation {
            Operation::Add => counts.added += 1,
            Operation::Remove => counts.removed += 1,
            Operation::Replace => counts.replaced += 1,
        }
    }
    summary
}

#[cfg(test)]
mod tests;
