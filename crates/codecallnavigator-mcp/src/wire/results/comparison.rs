//! Comparison wire shape checks; diff semantics remain entirely in the CLI.
use super::{array, boolean, integers, strings};
use anyhow::{Result, ensure};
use serde_json::Value;

pub(in crate::wire) fn metadata(value: &Value) -> Result<()> {
    let head = &value["result"]["head"];
    ensure!(
        head["digest"] == value["generation"] && value["generation"].is_string(),
        "Comparison target generation disagrees with envelope"
    );
    match head["kind"].as_str() {
        Some("commit") => ensure!(
            value["freshness"].is_null(),
            "Committed comparison must not claim live freshness"
        ),
        Some("worktree") => ensure!(
            value["freshness"].is_object(),
            "Working comparison omitted freshness"
        ),
        _ => anyhow::bail!("Unknown comparison target kind"),
    }
    Ok(())
}

pub(super) fn validate(result: &Value) -> Result<()> {
    ensure!(
        result["version"] == 1,
        "Unsupported comparison result version"
    );
    strings(result, &["comparison_id"])?;
    ensure!(
        crate::wire::generation(result["comparison_id"].as_str().unwrap_or_default()),
        "Malformed comparison binding"
    );
    for side in ["base", "head"] {
        endpoint(&result[side])?;
    }
    ensure!(
        result["base"]["kind"] == "commit",
        "Comparison base is not committed"
    );
    boolean(result, "graph_equal")?;
    boolean(result, "metadata_equal")?;
    ensure!(result["summary"].is_object(), "Comparison omitted summary");
    page(result)?;
    for change in array(result, "changes")? {
        record(change)?;
    }
    Ok(())
}

fn page(result: &Value) -> Result<()> {
    boolean(result, "more")?;
    integers(result, &["offset", "limit", "returned", "total"])?;
    let changes = array(result, "changes")?;
    ensure!(
        result["returned"].as_u64() == Some(changes.len() as u64),
        "Comparison page count mismatch"
    );
    let offset = result["offset"].as_u64().unwrap_or_default();
    let limit = result["limit"].as_u64().unwrap_or_default();
    let total = result["total"].as_u64().unwrap_or_default();
    let next = offset.saturating_add(changes.len() as u64);
    ensure!(
        (1..=500).contains(&limit) && changes.len() as u64 <= limit,
        "Invalid comparison page limit"
    );
    ensure!(
        result["more"] == (next < total),
        "Invalid comparison continuation"
    );
    if next < total {
        ensure!(
            !changes.is_empty() && result["next_offset"].as_u64() == Some(next),
            "Comparison page does not advance"
        );
    } else {
        ensure!(
            result["next_offset"].is_null(),
            "Unexpected comparison continuation"
        );
    }
    Ok(())
}

fn record(change: &Value) -> Result<()> {
    strings(change, &["scope", "section", "entity", "operation", "path"])?;
    ensure!(
        matches!(change["scope"].as_str(), Some("graph" | "artifact")),
        "Unknown comparison scope"
    );
    match change["operation"].as_str() {
        Some("add") => ensure!(
            change.get("before").is_none() && change.get("after").is_some(),
            "Malformed addition"
        ),
        Some("remove") => ensure!(
            change.get("before").is_some() && change.get("after").is_none(),
            "Malformed removal"
        ),
        Some("replace") => ensure!(
            change.get("before").is_some() && change.get("after").is_some(),
            "Malformed replacement"
        ),
        _ => anyhow::bail!("Unknown comparison operation"),
    }
    Ok(())
}

fn endpoint(value: &Value) -> Result<()> {
    strings(
        value,
        &["kind", "requested_ref", "path", "schema", "digest"],
    )?;
    ensure!(
        crate::wire::generation(value["digest"].as_str().unwrap_or_default()),
        "Malformed endpoint digest"
    );
    match value["kind"].as_str() {
        Some("commit") => {
            let commit = value["commit"].as_str().unwrap_or_default();
            ensure!(
                matches!(commit.len(), 40 | 64) && commit.bytes().all(|b| b.is_ascii_hexdigit()),
                "Malformed resolved commit"
            );
        }
        Some("worktree") => ensure!(
            value["commit"].is_null(),
            "Working graph has a resolved commit"
        ),
        _ => anyhow::bail!("Unknown endpoint kind"),
    }
    Ok(())
}
