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
    let before = ComparisonInput::validated(before)?;
    let after = ComparisonInput::validated(after)?;
    compare_graphs(
        before.graph.clone(),
        after.graph.clone(),
        before.extensions,
        after.extensions,
    )
}

/// Consumes saved snapshots so validation and canonical ordering do not require
/// retaining another complete JSON graph. Unknown fields remain in the raw graph.
pub fn compare_owned(before: Artifact, after: Artifact) -> Result<Difference> {
    before.validate()?;
    after.validate()?;
    compare_graphs(
        before.graph,
        after.graph,
        &before.extensions,
        &after.extensions,
    )
}

fn compare_graphs(
    mut before: Value,
    mut after: Value,
    before_extensions: &BTreeMap<String, Value>,
    after_extensions: &BTreeMap<String, Value>,
) -> Result<Difference> {
    sort_graph(&mut before)?;
    sort_graph(&mut after)?;
    Ok(compare_canonical(
        ComparisonInput {
            graph: &before,
            extensions: before_extensions,
        },
        ComparisonInput {
            graph: &after,
            extensions: after_extensions,
        },
    ))
}

struct ComparisonInput<'a> {
    graph: &'a Value,
    extensions: &'a BTreeMap<String, Value>,
}

impl<'a> ComparisonInput<'a> {
    fn validated(artifact: &'a Artifact) -> Result<Self> {
        artifact.validate()?;
        Ok(Self {
            graph: &artifact.graph,
            extensions: &artifact.extensions,
        })
    }
}

fn compare_canonical(before: ComparisonInput<'_>, after: ComparisonInput<'_>) -> Difference {
    let mut changes = Vec::new();
    for section in keys(Some(before.graph), Some(after.graph)) {
        walk(
            Location {
                scope: Scope::Graph,
                section: &section,
                entity: "",
                path: "",
            },
            before.graph.get(&section),
            after.graph.get(&section),
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
    Difference {
        graph_equal,
        metadata_equal,
        changes,
        summary,
    }
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
        record(location, before.cloned(), after.cloned(), changes);
    }
}

fn record(
    location: Location<'_>,
    before: Option<Value>,
    after: Option<Value>,
    changes: &mut Vec<Change>,
) {
    let operation = match (&before, &after) {
        (None, _) => Operation::Add,
        (_, None) => Operation::Remove,
        _ => Operation::Replace,
    };
    changes.push(Change {
        scope: location.scope,
        section: location.section.into(),
        entity: location.entity.into(),
        operation,
        path: location.path.into(),
        before,
        after,
    });
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

#[derive(PartialEq)]
enum Group<'a> {
    Singleton(&'a Value),
    Multiple(Vec<&'a Value>),
}

impl Group<'_> {
    /// Allocate payloads only for emitted changes, never for unchanged groups.
    fn to_value(&self) -> Value {
        match self {
            Self::Singleton(value) => (*value).clone(),
            Self::Multiple(values) => {
                Value::Array(values.iter().map(|value| (*value).clone()).collect())
            }
        }
    }
}

fn groups(values: &[Value], policy: Collection) -> BTreeMap<String, Group<'_>> {
    let mut grouped = BTreeMap::<String, Vec<&Value>>::new();
    for value in values {
        grouped
            .entry(identity(value, policy))
            .or_default()
            .push(value);
    }
    grouped
        .into_iter()
        .map(|(key, mut values)| {
            // sort_graph globally orders every supported collection by serialized
            // value. Filtering into groups retains precisely that variant order.
            let value = if matches!(
                policy,
                Collection::Nodes | Collection::Functions | Collection::Files
            ) && values.len() == 1
            {
                Group::Singleton(values.remove(0))
            } else {
                Group::Multiple(values)
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
        compare_groups(
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

fn compare_groups(
    location: Location<'_>,
    before: Option<&Group<'_>>,
    after: Option<&Group<'_>>,
    changes: &mut Vec<Change>,
) {
    if before == after {
        return;
    }
    match (before, after) {
        (Some(Group::Singleton(before)), Some(Group::Singleton(after))) => {
            walk(location, Some(before), Some(after), changes);
        }
        _ => record(
            location,
            before.map(Group::to_value),
            after.map(Group::to_value),
            changes,
        ),
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
