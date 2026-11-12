use super::Artifact;
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Clone, Debug, Serialize)]
pub struct Comparison {
    pub equal: bool,
    pub nodes: Changes,
    pub edges: Changes,
    pub sections: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Changes {
    pub added: Vec<Value>,
    pub removed: Vec<Value>,
    pub changed: Vec<Value>,
}

impl Changes {
    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

pub fn compare(before: &Artifact, after: &Artifact) -> Result<Comparison> {
    before.validate()?;
    after.validate()?;
    let nodes = changes(&before.graph["nodes"], &after.graph["nodes"], |v| {
        v["id"].to_string()
    })?;
    let edges = changes(&before.graph["edges"], &after.graph["edges"], |v| {
        json!([v["from"], v["to"], v["kind"]]).to_string()
    })?;
    let keys: BTreeSet<_> = before
        .graph
        .as_object()
        .unwrap()
        .keys()
        .chain(after.graph.as_object().unwrap().keys())
        .collect();
    let sections: Vec<_> = keys
        .into_iter()
        .filter(|key| !matches!(key.as_str(), "nodes" | "edges"))
        .filter(|key| before.graph[*key] != after.graph[*key])
        .cloned()
        .collect();
    Ok(Comparison {
        equal: nodes.is_empty() && edges.is_empty() && sections.is_empty(),
        nodes,
        edges,
        sections,
    })
}

fn changes(before: &Value, after: &Value, key: impl Fn(&Value) -> String) -> Result<Changes> {
    let before = groups(before, &key)?;
    let after = groups(after, &key)?;
    let mut changes = Changes::default();
    for (key, values) in &after {
        match before.get(key) {
            None => changes.added.extend(values.clone()),
            Some(old) if old != values => changes
                .changed
                .push(json!({"identity": key, "before": old, "after": values})),
            _ => {}
        }
    }
    changes.removed = before
        .into_iter()
        .filter(|(key, _)| !after.contains_key(key))
        .flat_map(|(_, values)| values)
        .collect();
    Ok(changes)
}

// Multiple certainty/provenance variants can share endpoint/kind identity.
fn groups(values: &Value, key: &impl Fn(&Value) -> String) -> Result<BTreeMap<String, Vec<Value>>> {
    let mut map = BTreeMap::new();
    for value in values
        .as_array()
        .context("Comparison collection must be an array")?
    {
        map.entry(key(value))
            .or_insert_with(Vec::new)
            .push(value.clone());
    }
    for values in map.values_mut() {
        values.sort_by_cached_key(Value::to_string);
    }
    Ok(map)
}

pub fn compare_git(project: &Path, base: &str, head: &str) -> Result<Comparison> {
    let before = super::revision::load(project, base)?;
    let after = super::revision::load(project, head)?;
    compare(&before.artifact, &after.artifact)
}
