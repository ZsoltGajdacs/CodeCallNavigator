//! Operation-specific query envelope validation, independent of process transport.
use super::{
    observation,
    results::{comparison_metadata, flow_comparison_metadata, impact_preview_metadata},
};
use anyhow::{Result, ensure};
use serde_json::Value;

pub fn validate(value: &Value, operation: &str, bound: bool) -> Result<()> {
    match operation {
        "compare" | "comparison_changes" | "class_flow_comparison" => comparison_metadata(value),
        "flow_comparison" => flow_comparison_metadata(value),
        "impact_preview" => impact_preview_metadata(value),
        "git_comparison_changes" | "snapshot_status" => {
            ensure!(
                value["generation"].is_null() && value["freshness"].is_null(),
                "{operation} unexpectedly claims an analyzed graph generation"
            );
            Ok(())
        }
        "explorer" if !bound && value["generation"].is_null() => Ok(()),
        _ => analyzed_metadata(value, operation),
    }
}

fn analyzed_metadata(value: &Value, operation: &str) -> Result<()> {
    ensure!(
        value["freshness"].is_object(),
        "CLI omitted analyzed graph freshness"
    );
    ensure!(
        value["generation"].as_str().is_some(),
        "CLI omitted analyzed graph generation"
    );
    if operation == "freshness" {
        observation::validate(&value["result"], value["generation"].as_str())?;
        observation::binding(&value["result"], &value["freshness"])?;
    }
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
