//! Operation-specific query envelope validation, independent of process transport.
use super::results::comparison_metadata;
use anyhow::{Result, ensure};
use serde_json::Value;

pub fn validate(value: &Value, operation: &str, bound: bool) -> Result<()> {
    if operation == "compare" {
        return comparison_metadata(value);
    }
    if let ("explorer", false, true) = (operation, bound, value["generation"].is_null()) {
        return Ok(());
    }
    ensure!(
        value["freshness"].is_object(),
        "CLI omitted analyzed graph freshness"
    );
    ensure!(
        value["generation"].as_str().is_some(),
        "CLI omitted analyzed graph generation"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn successful_bound_queries_require_nonnull_freshness() {
        let value = json!({"generation":"a".repeat(64),"freshness":null});
        assert!(validate(&value, "status", false).is_err());
        assert!(validate(&value, "explorer", true).is_err());
        assert!(
            validate(
                &json!({"generation":null,"freshness":null}),
                "explorer",
                false
            )
            .is_ok()
        );
    }
}
