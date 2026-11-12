//! Independent wire DTOs: no CLI library imports or graph interpretation.
mod freshness;
pub mod metadata;
pub mod results;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const OPERATIONS: &[&str] = &[
    "compare",
    "status",
    "search",
    "symbol",
    "source",
    "explorer",
    "selection",
    "neighbors",
    "flow",
];

pub fn generation(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn validate_generation(value: Option<&str>) -> Result<()> {
    ensure!(
        value.is_none_or(generation),
        "expected_generation must be a 64-character hexadecimal digest"
    );
    Ok(())
}

#[derive(Deserialize)]
struct Capabilities {
    query_version: u32,
    generation_binding: bool,
    operations: Vec<String>,
    comparison: ComparisonCapabilities,
}
#[derive(Deserialize)]
struct ComparisonCapabilities {
    version: u32,
    snapshot_binding: bool,
}
pub fn capabilities(value: &Value) -> Result<()> {
    let capabilities: Capabilities =
        serde_json::from_value(value.clone()).context("Malformed CLI capabilities")?;
    ensure!(
        capabilities.query_version == 1 && capabilities.generation_binding,
        "Incompatible CLI query contract: version 1 with generation binding is required"
    );
    ensure!(
        capabilities.comparison.version == 1 && capabilities.comparison.snapshot_binding,
        "Incompatible CLI comparison contract"
    );
    for operation in OPERATIONS {
        ensure!(
            capabilities.operations.iter().any(|v| v == operation),
            "Incompatible CLI: missing operation {operation}"
        );
    }
    Ok(())
}

#[derive(Deserialize)]
struct Response {
    version: u32,
    generation: Option<String>,
    freshness: Option<Value>,
    result: Option<Value>,
    error: Option<Failure>,
}
#[derive(Deserialize)]
struct Failure {
    code: String,
    message: String,
}

pub fn response(value: &Value, expected: Option<&str>) -> Result<bool> {
    // Require the full envelope, even nullable fields; serde Option alone accepts omissions.
    for key in ["version", "generation", "freshness", "result", "error"] {
        ensure!(value.get(key).is_some(), "CLI response omitted {key}");
    }
    let response: Response =
        serde_json::from_value(value.clone()).context("Malformed CLI query response")?;
    ensure!(response.version == 1, "Unsupported CLI response version");
    validate_generation(response.generation.as_deref())?;
    ensure!(
        response.result.is_some() != response.error.is_some(),
        "CLI response must contain exactly one result or error"
    );
    if let Some(freshness) = &response.freshness {
        freshness::validate(freshness, response.generation.as_deref())?;
    }
    if let Some(error) = response.error {
        ensure!(
            !error.code.is_empty() && !error.message.is_empty(),
            "Malformed CLI error"
        );
        return Ok(true);
    }
    if let Some(expected) = expected {
        ensure!(
            response.generation.as_deref() == Some(expected),
            "CLI returned a different graph generation"
        );
    }
    Ok(false)
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub path: String,
    pub language: String,
    pub kind: String,
    pub label: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    path: String,
    kind: String,
}
#[derive(Deserialize)]
pub struct Discovery {
    pub targets: Vec<Target>,
    pub default_selection: Vec<Identity>,
}
impl Discovery {
    pub fn selected(&self) -> Result<Vec<Target>> {
        let mut selected = Vec::new();
        for identity in &self.default_selection {
            let matches: Vec<_> = self
                .targets
                .iter()
                .filter(|t| t.path == identity.path && t.kind == identity.kind)
                .collect();
            ensure!(
                matches.len() == 1,
                "CLI discovery returned an unknown or ambiguous default identity"
            );
            selected.push(matches[0].clone());
        }
        ensure!(
            !selected.is_empty(),
            "CLI discovery returned no default build targets"
        );
        Ok(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn malformed_or_incompatible_envelopes_are_rejected() {
        for value in [
            json!({}),
            json!({"version":2,"generation":null,"freshness":null,"result":{},"error":null}),
            json!({"version":1,"generation":"bad","freshness":null,"result":{},"error":null}),
            json!({"version":1,"generation":null,"freshness":null,"result":{},"error":{"code":"x","message":"x"}}),
        ] {
            assert!(response(&value, None).is_err());
        }
        assert!(
            capabilities(
                &json!({"query_version":2,"generation_binding":true,"operations":OPERATIONS})
            )
            .is_err()
        );
    }
    #[test]
    fn structured_cli_failures_keep_original_codes_and_generation_metadata() {
        let value = json!({"version":1,"generation":null,"freshness":null,"result":null,"error":{"code":"graph_missing","message":"Refresh explicitly"}});
        assert!(response(&value, None).unwrap());
        assert_eq!(value["error"]["code"], "graph_missing");
    }
}
