//! Validate required operation payload shapes while preserving extension metadata.
mod comparison;
use anyhow::{Result, ensure};
pub(super) use comparison::metadata as comparison_metadata;
use comparison::validate as comparison;
use serde_json::Value;
pub fn validate(operation: &str, result: &Value) -> Result<()> {
    ensure!(result.is_object(), "CLI operation result must be an object");
    match operation {
        "compare" => comparison(result),
        "status" => integers(result, &["nodes", "edges"]),
        // Full typed observation and envelope binding are checked by metadata.
        "freshness" => Ok(()),
        "search" => {
            integers(result, &["offset", "limit"])?;
            boolean(result, "more")?;
            for hit in array(result, "hits")? {
                strings(hit, &["name", "detail"])?;
                let target = &hit["target"];
                match target["kind"].as_str() {
                    Some("method") => strings(target, &["symbol"]),
                    Some("class") => {
                        strings(target, &["build", "project", "container"])?;
                        integers(target, &["member_count"])
                    }
                    _ => anyhow::bail!("Malformed CLI search target"),
                }?;
            }
            Ok(())
        }
        "symbol" => strings(result, &["id", "language", "kind", "file", "label"]),
        "source" => strings(result, &["origin", "file", "content", "symbol"]),
        "explorer" => {
            strings(result, &["origin", "path"])?;
            integers(result, &["offset", "limit", "total"])?;
            boolean(result, "more")?;
            for entry in array(result, "entries")? {
                strings(entry, &["path", "label", "kind"])?;
            }
            Ok(())
        }
        "selection" | "neighbors" => section(result),
        "flow" => {
            strings(&result["function"], &["symbol", "file", "language"])?;
            strings(&result["chart"], &["symbol", "file"])?;
            for field in ["steps", "links", "regions", "diagnostics"] {
                array(&result["chart"], field)?;
            }
            boolean(&result["chart"], "partial")?;
            strings(&result["source"], &["origin", "file", "content", "symbol"])
        }
        _ => anyhow::bail!("Unsupported CLI operation response"),
    }
}
fn section(result: &Value) -> Result<()> {
    for node in array(result, "nodes")? {
        strings(node, &["id"])?;
    }
    for edge in array(result, "edges")? {
        strings(edge, &["from", "to", "kind", "certainty"])?;
    }
    ensure!(
        array(result, "roots")?.iter().all(Value::is_string),
        "Malformed CLI roots"
    );
    integers(result, &["omitted_nodes", "omitted_edges"])?;
    boolean(result, "truncated")
}
fn strings(value: &Value, fields: &[&str]) -> Result<()> {
    ensure!(
        fields.iter().all(|field| value[field].is_string()),
        "CLI result omitted a required string field: {fields:?}"
    );
    Ok(())
}
fn integers(value: &Value, fields: &[&str]) -> Result<()> {
    ensure!(
        fields.iter().all(|field| value[field].as_u64().is_some()),
        "CLI result omitted an unsigned integer: {fields:?}"
    );
    Ok(())
}
fn boolean(value: &Value, field: &str) -> Result<()> {
    ensure!(
        value[field].is_boolean(),
        "CLI result omitted boolean {field}"
    );
    Ok(())
}
fn array<'a>(value: &'a Value, field: &str) -> Result<&'a Vec<Value>> {
    value[field]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("CLI result omitted array {field}"))
}
/// Validate echoed comparison binding without interpreting graph contents.
pub fn binding(request: &Value, result: &Value) -> Result<()> {
    if request["kind"] == "compare"
        && let Some(expected) = request["expected_comparison"].as_str()
    {
        ensure!(
            result["comparison_id"] == expected,
            "CLI returned a different comparison binding"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn malformed_success_payloads_do_not_become_successful_tools() {
        assert!(validate("status", &json!("status")).is_err());
        assert!(
            validate(
                "search",
                &json!({"hits":"bad","offset":0,"limit":50,"more":false})
            )
            .is_err()
        );
        assert!(validate("status", &json!({"nodes":0,"edges":0,"future_metadata":{}})).is_ok());
    }

    #[test]
    fn comparison_binding_cannot_be_changed_by_a_successful_cli_response() {
        let expected = "a".repeat(64);
        let request = json!({"kind":"compare","expected_comparison":expected});
        assert!(binding(&request, &json!({"comparison_id":"b".repeat(64)})).is_err());
        assert!(binding(&request, &json!({"comparison_id":expected})).is_ok());
    }
}
