//! Independent wire DTOs: no CLI library imports or graph interpretation.
mod freshness;
mod observation;
pub use observation::Support as FreshnessSupport;
pub mod metadata;
pub mod results;
pub mod snapshots;
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
struct WireCapabilities {
    query_version: u32,
    generation_binding: bool,
    operations: Vec<String>,
    comparison: ComparisonCapabilities,
}
#[derive(Clone)]
pub struct Capabilities {
    pub freshness: FreshnessSupport,
    pub analysis_session: AnalysisSessionSupport,
    pub local_snapshots: bool,
    pub flow_comparison: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnalysisSessionSupport {
    Supported,
    Unsupported,
}

#[derive(Deserialize)]
struct AnalysisSessionCapabilities {
    version: u32,
    command: String,
    activity_version: u32,
    cancel_command: String,
}

fn analysis_session(value: Option<&Value>) -> AnalysisSessionSupport {
    let Some(value) = value else {
        return AnalysisSessionSupport::Unsupported;
    };
    let Ok(capability) = serde_json::from_value::<AnalysisSessionCapabilities>(value.clone())
    else {
        return AnalysisSessionSupport::Unsupported;
    };
    match capability {
        AnalysisSessionCapabilities {
            version: 1,
            activity_version: 1,
            command,
            cancel_command,
        } if command == "analysis-session" && cancel_command == "cancel_refresh" => {
            AnalysisSessionSupport::Supported
        }
        _ => AnalysisSessionSupport::Unsupported,
    }
}

#[derive(Deserialize)]
struct ComparisonCapabilities {
    version: u32,
    snapshot_binding: bool,
}
#[derive(Deserialize)]
struct LocalSnapshotCapabilities {
    version: u32,
    status_query: String,
    prepare_command: String,
    cancel_command: String,
}

fn supports_local_snapshots(value: &Value, operations: &[String]) -> bool {
    serde_json::from_value::<LocalSnapshotCapabilities>(value["local_snapshots"].clone()).is_ok_and(
        |snapshot| {
            snapshot.version == 1
                && snapshot.status_query == "snapshot_status"
                && snapshot.prepare_command == "prepare_snapshot"
                && snapshot.cancel_command == "cancel"
                && operations.iter().any(|op| op == &snapshot.status_query)
        },
    )
}

pub fn capabilities(value: &Value) -> Result<Capabilities> {
    let capabilities: WireCapabilities =
        serde_json::from_value(value.clone()).context("Malformed CLI capabilities")?;
    ensure!(
        capabilities.query_version == 2 && capabilities.generation_binding,
        "Incompatible CLI query contract: version 2 with generation binding is required"
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
    Ok(Capabilities {
        freshness: observation::negotiate(value.get("freshness"), &capabilities.operations),
        analysis_session: analysis_session(value.get("analysis_session")),
        local_snapshots: supports_local_snapshots(value, &capabilities.operations),
        flow_comparison: capabilities
            .operations
            .iter()
            .any(|op| op == "flow_comparison"),
    })
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
    ensure!(response.version == 2, "Unsupported CLI response version");
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
            json!({"version":999,"generation":null,"freshness":null,"result":{},"error":null}),
            json!({"version":2,"generation":"bad","freshness":null,"result":{},"error":null}),
            json!({"version":2,"generation":null,"freshness":null,"result":{},"error":{"code":"x","message":"x"}}),
        ] {
            assert!(response(&value, None).is_err());
        }
        assert!(
            capabilities(
                &json!({"query_version":999,"generation_binding":true,"operations":OPERATIONS})
            )
            .is_err()
        );
        let mut value = json!({"query_version":2,"generation_binding":true,"comparison":{"version":1,"snapshot_binding":true},"operations":OPERATIONS});
        assert_eq!(
            capabilities(&value).unwrap().analysis_session,
            AnalysisSessionSupport::Unsupported
        );
        for capability in [
            json!(null),
            json!({}),
            json!("unknown"),
            json!({"version":2,"command":"analysis-session","activity_version":1,"cancel_command":"cancel_refresh"}),
            json!({"version":1,"command":"unknown","activity_version":1,"cancel_command":"cancel_refresh"}),
            json!({"version":1,"command":"analysis-session","activity_version":1}),
        ] {
            value["analysis_session"] = capability;
            assert_eq!(
                capabilities(&value).unwrap().analysis_session,
                AnalysisSessionSupport::Unsupported
            );
        }
        value["analysis_session"] = json!({"version":1,"command":"analysis-session","activity_version":1,"cancel_command":"cancel_refresh"});
        assert_eq!(
            capabilities(&value).unwrap().analysis_session,
            AnalysisSessionSupport::Supported
        );
    }
    #[test]
    fn structured_cli_failures_keep_original_codes_and_generation_metadata() {
        let value = json!({"version":2,"generation":null,"freshness":null,"result":null,"error":{"code":"graph_missing","message":"Refresh explicitly"}});
        assert!(response(&value, None).unwrap());
        assert_eq!(value["error"]["code"], "graph_missing");
    }
}
