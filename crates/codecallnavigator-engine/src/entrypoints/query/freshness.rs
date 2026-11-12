//! Query metadata stays bounded; the explicit verify command retains every reason.
use codecallnavigator_core::{
    artifact::verification::Report,
    query::{self, Error},
};
use serde_json::Value;

pub(in crate::entrypoints) fn summary(mut report: Report) -> query::Result<Value> {
    let total = report.reasons.len();
    let mut bytes = 2; // JSON array brackets.
    let mut retained = 0;
    for reason in report.reasons.iter().take(query::MAX_FRESHNESS_REASONS) {
        let size = serde_json::to_vec(reason)
            .map_err(serialization_error)?
            .len();
        let separator = usize::from(retained > 0);
        if bytes + separator + size > query::MAX_FRESHNESS_REASON_BYTES {
            break;
        }
        bytes += separator + size;
        retained += 1;
    }
    report.reasons.truncate(retained);
    let mut value = serde_json::to_value(report).map_err(serialization_error)?;
    value["reasons_total"] = total.into();
    value["omitted_reasons"] = (total - retained).into();
    Ok(value)
}

fn serialization_error(error: serde_json::Error) -> Error {
    Error::new("serialization_failed", error.to_string())
}
