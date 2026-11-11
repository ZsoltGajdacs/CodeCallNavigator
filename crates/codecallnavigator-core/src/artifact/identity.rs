use super::paths::Paths;
use crate::model::GraphData;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub(super) fn identities(
    graph: &GraphData,
    paths: &Paths,
    supplied: &HashMap<String, String>,
) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();
    let mut used = HashSet::new();
    for node in &graph.nodes {
        let build = paths.portable(&node.build)?;
        let file = paths.portable(&node.file)?;
        let semantic = semantic_key(node, paths, supplied, (&build, &file))?;
        let id = json!([node.language, build, file, semantic]).to_string();
        ensure!(
            used.insert(id.clone()),
            "Portable symbol collision for {}",
            node.label
        );
        result.insert(node.id.clone(), id);
    }
    Ok(result)
}

fn semantic_key(
    node: &crate::model::Node,
    paths: &Paths,
    supplied: &HashMap<String, String>,
    locations: (&str, &str),
) -> Result<String> {
    if let Some(key) = supplied.get(&node.id) {
        return Ok(supplied_semantic(node, key, paths));
    }
    if let Some(key) = portable_semantic(node, locations) {
        return Ok(key);
    }
    if node.language == "typescript" {
        return typescript_key(node);
    }
    Ok(paths.embedded(&node.id))
}

fn supplied_semantic(node: &crate::model::Node, key: &str, paths: &Paths) -> String {
    // TypeScript keys use logical declaration owners; embedded strings are source types/names.
    if node.language == "typescript" {
        key.into()
    } else {
        paths.embedded(key)
    }
}

/// Only our exact canonical representation with this node's complete owner is reusable.
/// Arbitrary native JSON strings must remain native analyzer identities.
fn portable_semantic(node: &crate::model::Node, locations: (&str, &str)) -> Option<String> {
    let [_, _, _, semantic]: [String; 4] = serde_json::from_str(&node.id).ok()?;
    let expected = json!([node.language, locations.0, locations.1, semantic]).to_string();
    (node.id == expected).then_some(semantic)
}

fn typescript_key(node: &crate::model::Node) -> Result<String> {
    ensure!(
        !node.label.starts_with("<callback:"),
        "TypeScript anonymous callable {} needs an adapter semantic export identity",
        node.id
    );
    Ok(json!([node.container, node.kind, node.label, node.signature]).to_string())
}

pub(super) fn rewrite(value: &mut Value, identities: &HashMap<String, String>) -> Result<()> {
    let native = value
        .as_str()
        .context("Symbol reference must be a string")?;
    *value = identities
        .get(native)
        .with_context(|| format!("Unknown symbol reference: {native}"))?
        .clone()
        .into();
    Ok(())
}
