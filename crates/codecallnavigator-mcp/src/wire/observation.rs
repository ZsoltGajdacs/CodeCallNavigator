//! Independent freshness-v1 DTOs; the CLI owns input validity and observation policy.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;

const MAX_REASONS: usize = 50;
const MAX_REASON_BYTES: usize = 8192;

#[derive(Clone, Copy)]
pub enum Support {
    Supported,
    Unsupported(&'static str),
}

pub fn negotiate(value: Option<&Value>, operations: &[String]) -> Support {
    if !operations.iter().any(|operation| operation == "freshness") {
        return Support::Unsupported(
            "Configured CLI has no freshness operation; upgrade the CLI and restart the MCP server for graph_freshness",
        );
    }
    let Some(value) = value else {
        return Support::Unsupported("Configured CLI does not advertise freshness observation v1");
    };
    if value["version"].as_u64() != Some(1) {
        return Support::Unsupported(
            "Configured CLI freshness observation version is unsupported; version 1 is required",
        );
    }
    if value["mode"].as_str() != Some("verified_polling") {
        return Support::Unsupported("Configured CLI freshness mode is unsupported");
    }
    Support::Supported
}

#[derive(Deserialize)]
struct Observation {
    version: u32,
    generation: String,
    input_revision: Option<String>,
    contract_valid: bool,
    input_status: Status,
    reasons: Vec<Reason>,
    reasons_total: u64,
    omitted_reasons: u64,
    coverage: Coverage,
    mode: Mode,
    verification_milliseconds: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Match,
    Stale,
    Unverifiable,
}

#[derive(Deserialize)]
struct Reason {
    code: String,
    path: Option<String>,
}

#[derive(Deserialize)]
struct Coverage {
    kind: CoverageKind,
    complete: bool,
    certifies_build: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum CoverageKind {
    ObservedInputs,
}

#[derive(Deserialize)]
struct Mode {
    kind: ModeKind,
    limited: bool,
    reconcile_milliseconds: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ModeKind {
    VerifiedPolling,
}

pub fn validate(value: &Value, generation: Option<&str>) -> Result<()> {
    ensure!(
        value.get("input_revision").is_some(),
        "CLI observation omitted input_revision"
    );
    let observation: Observation =
        serde_json::from_value(value.clone()).context("Malformed CLI freshness observation")?;
    ensure!(
        observation.version == 1,
        "Unsupported CLI observation version"
    );
    ensure!(
        Some(observation.generation.as_str()) == generation,
        "CLI observation generation disagrees with envelope"
    );
    super::validate_generation(observation.input_revision.as_deref())?;
    validate_coverage(&observation)?;
    validate_reasons(&observation, &value["reasons"])?;
    ensure!(
        observation.verification_milliseconds.is_finite()
            && observation.verification_milliseconds >= 0.0,
        "Invalid CLI verification duration"
    );
    ensure!(
        observation.mode.reconcile_milliseconds > 0,
        "Invalid CLI observation interval"
    );
    // Typed facts are forwarded unchanged, never interpreted as graph policy.
    let _ = (
        observation.contract_valid,
        observation.input_status,
        observation.coverage.kind,
        observation.mode.kind,
        observation.mode.limited,
    );
    Ok(())
}

fn validate_coverage(observation: &Observation) -> Result<()> {
    ensure!(
        observation.contract_valid || matches!(observation.input_status, Status::Unverifiable),
        "Invalid CLI contract cannot claim matching or stale inputs"
    );
    ensure!(
        !observation.coverage.certifies_build,
        "Observation v1 cannot certify the build environment"
    );
    ensure!(
        !observation.coverage.complete || observation.input_revision.is_some(),
        "Complete CLI observation omitted its input revision"
    );
    Ok(())
}

pub fn binding(observation: &Value, legacy: &Value) -> Result<()> {
    for field in [
        "contract_valid",
        "input_status",
        "reasons",
        "reasons_total",
        "omitted_reasons",
    ] {
        ensure!(
            observation[field] == legacy[field],
            "CLI observation disagrees with legacy freshness field {field}"
        );
    }
    Ok(())
}

fn validate_reasons(observation: &Observation, raw: &Value) -> Result<()> {
    ensure!(
        observation
            .reasons_total
            .checked_sub(observation.omitted_reasons)
            == Some(observation.reasons.len() as u64),
        "Inconsistent CLI observation reason counts"
    );
    ensure!(
        observation.reasons.len() <= MAX_REASONS,
        "CLI observation exceeded reason count bound"
    );
    ensure!(
        serde_json::to_vec(raw)?.len() <= MAX_REASON_BYTES,
        "CLI observation exceeded reason byte bound"
    );
    for reason in &observation.reasons {
        ensure!(!reason.code.is_empty(), "Malformed CLI observation reason");
        let _ = &reason.path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({"version":1,"generation":"a".repeat(64),"input_revision":"b".repeat(64),"contract_valid":true,"input_status":"unverifiable","reasons":[{"code":"unmapped_input"}],"reasons_total":1,"omitted_reasons":0,"coverage":{"kind":"observed_inputs","complete":true,"certifies_build":false},"mode":{"kind":"verified_polling","limited":true,"reconcile_milliseconds":250},"verification_milliseconds":2.0})
    }

    #[test]
    fn typed_observation_requires_binding_shape_counts_and_finite_time() {
        let generation = "a".repeat(64);
        validate(&valid(), Some(&generation)).unwrap();
        for (path, invalid) in [
            ("/version", json!(2)),
            ("/generation", json!("c".repeat(64))),
            ("/input_revision", json!("bad")),
            ("/input_status", json!("fresh")),
            ("/reasons_total", json!(2)),
            ("/verification_milliseconds", json!(-1)),
            ("/coverage/certifies_build", json!(true)),
            ("/mode/reconcile_milliseconds", json!(0)),
            ("/mode/kind", json!("unknown")),
            ("/coverage/kind", json!("build")),
        ] {
            let mut value = valid();
            *value.pointer_mut(path).unwrap() = invalid;
            assert!(validate(&value, Some(&generation)).is_err(), "{path}");
        }
        let mut missing = valid();
        missing.as_object_mut().unwrap().remove("input_revision");
        assert!(validate(&missing, Some(&generation)).is_err());
    }

    #[test]
    fn limited_null_revision_is_supported_but_not_complete_or_unbounded_reasons() {
        let generation = "a".repeat(64);
        let mut value = valid();
        value["input_revision"] = Value::Null;
        assert!(validate(&value, Some(&generation)).is_err());
        value["coverage"]["complete"] = json!(false);
        validate(&value, Some(&generation)).unwrap();
        value["reasons"][0]["path"] = json!("x".repeat(8192));
        assert!(validate(&value, Some(&generation)).is_err());
    }

    #[test]
    fn optional_capability_absence_or_future_version_does_not_disable_old_tools() {
        assert!(matches!(negotiate(None, &[]), Support::Unsupported(_)));
        let operations = vec!["freshness".into()];
        assert!(matches!(
            negotiate(
                Some(&json!({"version":2,"mode":"verified_polling"})),
                &operations
            ),
            Support::Unsupported(_)
        ));
        assert!(matches!(
            negotiate(
                Some(&json!({"version":1,"mode":"verified_polling"})),
                &operations
            ),
            Support::Supported
        ));
    }

    #[test]
    fn invalid_contract_requires_unverifiable_and_conversion_must_agree() {
        let generation = "a".repeat(64);
        for status in ["match", "stale"] {
            let mut value = valid();
            value["contract_valid"] = json!(false);
            value["input_status"] = json!(status);
            assert!(validate(&value, Some(&generation)).is_err());
        }
        let observation = valid();
        let mut legacy = observation.clone();
        binding(&observation, &legacy).unwrap();
        legacy["input_status"] = json!("match");
        assert!(binding(&observation, &legacy).is_err());
    }
}
