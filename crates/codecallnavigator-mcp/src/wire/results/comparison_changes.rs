//! Compact comparison wire validation. The engine owns classification and diffing.
use super::{array, boolean, comparison, integers, strings};
use anyhow::{Result, bail, ensure};
use serde_json::Value;

pub(super) fn validate(result: &Value) -> Result<()> {
    ensure!(
        result["version"] == 1,
        "Unsupported change comparison version"
    );
    strings(result, &["comparison_id"])?;
    ensure!(
        crate::wire::generation(result["comparison_id"].as_str().unwrap_or_default()),
        "Malformed change comparison binding"
    );
    for side in ["base", "head"] {
        comparison::endpoint(&result[side])?;
        strings(&result[side], &["schema"])?;
    }
    ensure!(
        result["base"]["kind"] == "commit",
        "Comparison base is not committed"
    );
    match result["kind"].as_str() {
        Some("list") => list(result),
        Some("source_diff") => source_diff(result),
        _ => bail!("Unknown change comparison result kind"),
    }
}

pub(super) fn binding(request: &Value, result: &Value) -> Result<()> {
    let selection = &request["selection"];
    ensure!(
        result["kind"] == selection["kind"],
        "CLI returned a different selection kind"
    );
    for side in ["base", "head"] {
        ensure!(
            result[side]["requested_ref"] == request[side],
            "CLI returned a different {side} reference"
        );
    }
    if selection["kind"] == "source_diff" {
        ensure!(
            result["id"] == selection["id"],
            "CLI returned a different change ID"
        );
    } else {
        for key in ["offset", "limit"] {
            ensure!(
                result[key] == selection[key],
                "CLI returned a different page {key}"
            );
        }
    }
    Ok(())
}

fn list(result: &Value) -> Result<()> {
    boolean(&result["coverage"], "entry_point_search_complete")?;
    comparison::page(result, "changes")?;
    array(result, "changes")?.iter().try_for_each(change)
}

fn change(record: &Value) -> Result<()> {
    strings(record, &["id", "name", "file", "kind", "status"])?;
    match record["kind"].as_str() {
        Some("method") => {
            strings(record, &["symbol"])?;
            boolean(record, "flow_available")?;
            ensure!(
                record
                    .get("class_id")
                    .is_none_or(|value| value.is_null() || value.is_string()),
                "Malformed method class ID"
            );
        }
        Some("class" | "file") => {}
        _ => bail!("Unknown changed entity kind"),
    }
    ensure!(
        matches!(
            record["status"].as_str(),
            Some("added" | "removed" | "modified")
        ),
        "Unknown changed entity status"
    );
    ensure!(
        array(record, "changes_in")?.iter().all(Value::is_string),
        "Malformed change categories"
    );
    array(record, "entry_points")?
        .iter()
        .try_for_each(entry_point)
}

fn entry_point(route: &Value) -> Result<()> {
    strings(route, &["id", "name"])?;
    for side in ["before", "after"] {
        ensure!(
            route.get(side).is_some_and(|value| {
                value.is_null() || matches!(value.as_str(), Some("direct" | "possible"))
            }),
            "Malformed entry-point reachability"
        );
    }
    Ok(())
}

fn source_diff(result: &Value) -> Result<()> {
    strings(result, &["id", "file"])?;
    let hunks = array(result, "hunks")?;
    match result["status"].as_str() {
        Some("ready") => {}
        Some("unavailable") if hunks.is_empty() => {}
        _ => bail!("Malformed source diff availability"),
    }
    hunks.iter().try_for_each(hunk)
}

fn hunk(hunk: &Value) -> Result<()> {
    let (mut before, mut after) = (0, 0);
    for line in array(hunk, "lines")? {
        strings(line, &["text"])?;
        match line["kind"].as_str() {
            Some("context") => {
                before += 1;
                after += 1;
            }
            Some("delete") => before += 1,
            Some("insert") => after += 1,
            _ => bail!("Unknown unified diff line kind"),
        }
    }
    for (side, count) in [("before", before), ("after", after)] {
        integers(&hunk[side], &["start", "count"])?;
        ensure!(
            hunk[side]["count"].as_u64() == Some(count),
            "Unified diff line count disagrees with {side} range"
        );
    }
    Ok(())
}
