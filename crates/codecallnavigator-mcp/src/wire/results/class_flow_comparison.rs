//! Class graph wire integrity. Class ownership and aggregation belong to the CLI.
use super::{array, boolean, comparison, strings};
use anyhow::{Result, ensure};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(super) fn validate(result: &Value) -> Result<()> {
    comparison::header(result)?;
    let graph = &result["graph"];
    let root = field(graph, "root")?;
    let nodes = node_presence(graph)?;
    ensure!(nodes.contains_key(root), "Graph root is missing");
    validate_links(graph, root, &nodes)
}

fn node_presence(graph: &Value) -> Result<HashMap<&str, (bool, bool)>> {
    let mut nodes = HashMap::new();
    for node in array(graph, "nodes")? {
        strings(node, &["name"])?;
        let id = field(node, "id")?;
        let presence = (
            boolean_value(node, "before")?,
            boolean_value(node, "after")?,
        );
        ensure!(presence.0 || presence.1, "Class missing on both sides");
        check_status(node)?;
        ensure!(
            nodes.insert(id, presence).is_none(),
            "Duplicate graph class"
        );
    }
    Ok(nodes)
}

fn validate_links(graph: &Value, root: &str, nodes: &HashMap<&str, (bool, bool)>) -> Result<()> {
    let mut links = HashSet::new();
    for link in array(graph, "links")? {
        let (from, to) = (field(link, "from")?, field(link, "to")?);
        ensure!(
            from != to && (from == root || to == root),
            "Class link must connect the root to a neighbor"
        );
        let from_sides = nodes
            .get(from)
            .ok_or_else(|| anyhow::anyhow!("Unknown caller class"))?;
        let to_sides = nodes
            .get(to)
            .ok_or_else(|| anyhow::anyhow!("Unknown callee class"))?;
        let before = calls(link, "before")?;
        let after = calls(link, "after")?;
        ensure!(before || after, "Class link missing on both sides");
        check_link_presence(before, from_sides.0, to_sides.0)?;
        check_link_presence(after, from_sides.1, to_sides.1)?;
        check_status(link)?;
        ensure!(links.insert((from, to)), "Duplicate class link");
    }
    Ok(())
}

fn check_link_presence(link: bool, from: bool, to: bool) -> Result<()> {
    match (link, from, to) {
        (false, _, _) | (true, true, true) => Ok(()),
        _ => anyhow::bail!("Class link endpoints are missing on the recorded side"),
    }
}

pub(super) fn binding(request: &Value, result: &Value) -> Result<()> {
    for side in ["base", "head"] {
        ensure!(
            result[side]["requested_ref"] == request[side],
            "CLI returned a different {side} reference"
        );
    }
    ensure!(
        result["graph"]["root"] == request["id"],
        "CLI returned a different class"
    );
    Ok(())
}

fn field<'a>(value: &'a Value, name: &str) -> Result<&'a str> {
    value[name]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Missing class graph field {name}"))
}

fn boolean_value(value: &Value, name: &str) -> Result<bool> {
    boolean(value, name)?;
    Ok(value[name] == true)
}

fn calls(link: &Value, side: &str) -> Result<bool> {
    let counts = link[side]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Missing class link counts for {side}"))?;
    ensure!(
        counts.iter().all(|(resolution, count)| matches!(
            resolution.as_str(),
            "direct" | "virtual_dispatch" | "possible" | "other"
        ) && count.is_u64()),
        "Invalid class link resolution counts"
    );
    Ok(!counts.is_empty())
}

fn check_status(item: &Value) -> Result<()> {
    ensure!(
        matches!(
            item["status"].as_str(),
            Some("unchanged" | "added" | "removed" | "modified")
        ),
        "Unknown class change status"
    );
    Ok(())
}
