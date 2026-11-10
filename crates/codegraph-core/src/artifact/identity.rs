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
        let semantic = match supplied.get(&node.id) {
            Some(key) => paths.embedded(key),
            None if node.language == "typescript" => typescript_key(node)?,
            None => paths.embedded(&node.id),
        };
        let id = json!([
            node.language,
            paths.portable(&node.build)?,
            paths.portable(&node.file)?,
            semantic
        ])
        .to_string();
        ensure!(
            used.insert(id.clone()),
            "Portable symbol collision for {}",
            node.label
        );
        result.insert(node.id.clone(), id);
    }
    Ok(result)
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
