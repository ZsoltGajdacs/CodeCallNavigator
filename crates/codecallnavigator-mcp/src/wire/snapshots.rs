//! Validation of local snapshot records received from the installed CLI.
use anyhow::{Result, ensure};
use serde_json::Value;

pub fn status(value: &Value) -> Result<()> {
    ensure!(
        value["cache_path"].is_string(),
        "CLI omitted local snapshot cache path"
    );
    ensure!(
        value["commit"].as_str().is_some_and(commit),
        "Invalid snapshot revision"
    );
    let entries = value["snapshots"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("CLI omitted snapshot records"))?;
    for entry in entries {
        record(entry)?;
        ensure!(
            entry["identity"]["commit"] == value["commit"],
            "Snapshot record does not belong to the resolved revision"
        );
    }
    Ok(())
}

pub fn record(value: &Value) -> Result<()> {
    for field in ["id", "digest"] {
        ensure!(
            value[field].as_str().is_some_and(super::generation),
            "Invalid snapshot {field}"
        );
    }
    let identity = &value["identity"];
    ensure!(
        identity["commit"].as_str().is_some_and(commit),
        "Invalid snapshot commit"
    );
    for field in ["project", "analyzer", "selection"] {
        ensure!(
            identity[field].is_string(),
            "CLI omitted snapshot identity {field}"
        );
    }
    Ok(())
}

pub fn retention(value: &Value) -> Result<()> {
    if value.is_null() {
        return Ok(());
    }
    match value["status"].as_str() {
        Some("verified") => ensure!(
            value["commit"].as_str().is_some_and(commit)
                && value["snapshot"].as_str().is_some_and(super::generation),
            "Invalid verified commit snapshot in CLI completion"
        ),
        Some("working_only") => ensure!(
            value["reason"]
                .as_str()
                .is_some_and(|reason| !reason.trim().is_empty()),
            "CLI completion omitted why it saved only the working graph"
        ),
        _ => anyhow::bail!("Unknown CLI graph retention result"),
    }
    Ok(())
}

pub(super) fn commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
