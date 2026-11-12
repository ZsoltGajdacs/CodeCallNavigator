use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;
#[derive(Deserialize)]
struct Freshness {
    version: u32,
    contract_valid: bool,
    input_status: Status,
    digest: String,
    reasons: Vec<Reason>,
    reasons_total: u64,
    omitted_reasons: u64,
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
pub fn validate(value: &Value, generation: Option<&str>) -> Result<()> {
    let report: Freshness =
        serde_json::from_value(value.clone()).context("Malformed CLI freshness summary")?;
    ensure!(report.version == 1, "Unsupported CLI freshness version");
    ensure!(
        Some(report.digest.as_str()) == generation,
        "CLI freshness digest disagrees with graph generation"
    );
    ensure!(
        report.reasons_total.checked_sub(report.omitted_reasons)
            == Some(report.reasons.len() as u64),
        "Inconsistent CLI freshness reason counts"
    );
    for reason in report.reasons {
        ensure!(!reason.code.is_empty(), "Malformed CLI freshness reason");
        let _ = reason.path;
    }
    // These are typed wire facts; their semantic policy remains CLI-owned.
    let _ = (report.contract_valid, report.input_status);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn freshness_requires_known_status_matching_digest_and_complete_counts() {
        let digest = "a".repeat(64);
        let valid = json!({"version":1,"contract_valid":true,"input_status":"match","digest":digest,"reasons":[],"reasons_total":0,"omitted_reasons":0});
        validate(&valid, Some(&digest)).unwrap();
        for value in [
            json!({}),
            {
                let mut v = valid.clone();
                v["input_status"] = json!("fresh");
                v
            },
            {
                let mut v = valid.clone();
                v["digest"] = json!("b".repeat(64));
                v
            },
            {
                let mut v = valid.clone();
                v["reasons_total"] = json!(1);
                v
            },
        ] {
            assert!(validate(&value, Some(&digest)).is_err());
        }
    }
}
