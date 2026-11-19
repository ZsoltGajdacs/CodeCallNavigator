//! Validate the flow comparison envelope and selection without interpreting graph facts.
use super::{array, boolean, comparison, strings};
use anyhow::{Result, bail, ensure};
use serde_json::Value;

pub(in crate::wire) fn metadata(value: &Value) -> Result<()> {
    let head = &value["result"]["head"];
    ensure!(
        head["digest"] == value["generation"] && value["generation"].is_string(),
        "Flow comparison target generation disagrees with envelope"
    );
    ensure!(
        value["freshness"].is_null()
            || (head["kind"] == "worktree" && value["freshness"].is_object()),
        "Invalid flow comparison freshness"
    );
    Ok(())
}

pub(super) fn validate(result: &Value) -> Result<()> {
    ensure!(
        result["version"] == 1,
        "Unsupported flow comparison result version"
    );
    ensure!(
        result["comparison_id"]
            .as_str()
            .is_some_and(crate::wire::generation),
        "Malformed flow comparison binding"
    );
    for side in ["base", "head"] {
        comparison::endpoint(&result[side])?;
        ensure!(
            result[side]["schema"].is_string() || result[side]["schema"].is_null(),
            "Invalid flow comparison endpoint schema"
        );
    }
    ensure!(
        result["base"]["kind"] == "commit",
        "Flow comparison base must name a commit"
    );
    match result["kind"].as_str() {
        Some("catalog") => catalog(result),
        Some("detail") => detail(result),
        _ => bail!("Invalid flow comparison selection result"),
    }
}

fn catalog(result: &Value) -> Result<()> {
    strings(result, &["text"])?;
    boolean(result, "complete")?;
    comparison::page(result, "records")?;
    for side in ["before", "after"] {
        ensure!(
            matches!(
                result["endpoints"][side]["status"].as_str(),
                Some("available" | "unavailable")
            ),
            "Invalid flow catalog endpoint status"
        );
    }
    for record in array(result, "records")? {
        strings(record, &["symbol"])?;
        ensure!(
            matches!(
                record["membership"].as_str(),
                Some("both" | "added" | "removed" | "unknown")
            ),
            "Invalid flow catalog membership"
        );
        for side in ["before", "after"] {
            ensure!(
                matches!(
                    record[side]["status"].as_str(),
                    Some("present" | "missing" | "unknown")
                ),
                "Invalid flow catalog side status"
            );
        }
    }
    Ok(())
}

fn detail(result: &Value) -> Result<()> {
    let correspondence = &result["correspondence"];
    strings(correspondence, &["kind", "symbol", "head_symbol"])?;
    ensure!(
        matches!(correspondence["kind"].as_str(), Some("exact" | "manual")),
        "Invalid flow comparison correspondence"
    );
    for side in ["before", "after"] {
        let detail = &result[side];
        match detail["status"].as_str() {
            Some("ready") => {
                strings(&detail["source"], &["origin", "file", "symbol", "content"])?;
                for field in ["node", "function", "chart"] {
                    ensure!(
                        detail[field].is_object(),
                        "Flow comparison detail omitted {field}"
                    );
                }
                array(detail, "targets")?;
            }
            Some("missing") => {}
            Some("unavailable") => strings(detail, &["reason", "message"])?,
            _ => bail!("Invalid flow comparison detail side status"),
        }
    }
    Ok(())
}
