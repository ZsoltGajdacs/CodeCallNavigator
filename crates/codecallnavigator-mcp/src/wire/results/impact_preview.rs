//! Validate the CLI's impact preview contract without interpreting graph facts.
use super::{array, boolean, comparison, integers, strings};
use crate::wire::snapshots;
use anyhow::{Result, ensure};
use serde_json::Value;

pub(in crate::wire) fn metadata(value: &Value) -> Result<()> {
    ensure!(
        value["generation"].is_null()
            && value["graph_schema"].is_null()
            && value["freshness"].is_null(),
        "Impact preview unexpectedly claims a working generation"
    );
    Ok(())
}

pub(super) fn validate(result: &Value) -> Result<()> {
    ensure!(
        result["version"] == 1,
        "Unsupported impact preview result version"
    );
    comparison::endpoint(&result["base"])?;
    ensure!(
        result["base"]["kind"] == "commit",
        "Impact preview base must name a commit"
    );
    strings(&result["base"], &["schema"])?;
    head(&result["head"])?;
    coverage(&result["coverage"])?;
    for record in array(result, "records")? {
        validate_record(record)?;
    }
    Ok(())
}

fn head(value: &Value) -> Result<()> {
    strings(value, &["requested_ref"])?;
    if value["requested_ref"] == "WORKTREE" {
        ensure!(
            value["commit"].is_null(),
            "Impact preview working tree has a commit"
        );
    } else {
        ensure!(
            value["commit"].as_str().is_some_and(snapshots::commit),
            "Impact preview head omitted a resolved commit"
        );
    }
    Ok(())
}

fn coverage(value: &Value) -> Result<()> {
    boolean(value, "tracked_only")?;
    boolean(value, "truncated")?;
    integers(
        value,
        &["changed_paths", "mapped_ranges", "unmapped_ranges"],
    )
}

fn validate_record(value: &Value) -> Result<()> {
    strings(value, &["path", "status"])?;
    integers(value, &["old_start", "old_count"])?;
    ensure!(
        value["unresolved"].is_null() || value["unresolved"].is_string(),
        "Invalid impact preview unresolved reason"
    );
    for symbol in array(value, "symbols")? {
        strings(symbol, &["id", "label"])?;
        integers(symbol, &["line"])?;
        for caller in array(symbol, "callers")? {
            validate_caller(caller)?;
        }
    }
    Ok(())
}

fn validate_caller(value: &Value) -> Result<()> {
    strings(value, &["id", "label"])?;
    boolean(value, "possible")?;
    ensure!(
        array(value, "path")?.iter().all(Value::is_string),
        "Invalid impact preview caller path"
    );
    Ok(())
}

pub(super) fn binding(request: &Value, result: &Value) -> Result<()> {
    for side in ["base", "head"] {
        ensure!(
            request[side] == result[side]["requested_ref"],
            "CLI returned a different impact preview {side} reference"
        );
    }
    if let Some(expected) = request["base_snapshot"].as_str() {
        ensure!(
            result["base"]["snapshot"]["id"] == expected,
            "CLI returned a different impact preview snapshot selection"
        );
    }
    Ok(())
}
