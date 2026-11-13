//! Query metadata stays bounded; the explicit verify command retains every reason.
use codecallnavigator_core::{
    artifact::verification::Report,
    query::{self, Error},
};
use serde_json::Value;

pub(in crate::entrypoints) struct Bounded {
    pub report: Report,
    pub total: usize,
    pub omitted: usize,
}

impl Bounded {
    pub fn value(&self) -> query::Result<Value> {
        let mut value = serde_json::to_value(&self.report).map_err(serialization_error)?;
        value["reasons_total"] = self.total.into();
        value["omitted_reasons"] = self.omitted.into();
        Ok(value)
    }
}

pub(in crate::entrypoints) fn summary(report: Report) -> query::Result<Value> {
    bounded(report)?.value()
}

pub(in crate::entrypoints) fn bounded(mut report: Report) -> query::Result<Bounded> {
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
    Ok(Bounded {
        report,
        total,
        omitted: total - retained,
    })
}

fn serialization_error(error: serde_json::Error) -> Error {
    Error::new("serialization_failed", error.to_string())
}
