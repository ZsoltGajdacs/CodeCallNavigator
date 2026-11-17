//! Query metadata stays bounded; the explicit verify command retains every reason.
use codecallnavigator_core::{
    artifact::verification::{Reason, Report},
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
    Reason::normalize(&mut report.reasons);
    report.reasons.iter_mut().for_each(bound_reason);
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

fn bound_reason(reason: &mut Reason) {
    bound_text(&mut reason.code, 128);
    for (field, limit) in [
        (&mut reason.path, 1024),
        (&mut reason.operation, 256),
        (&mut reason.detail, 1536),
        (&mut reason.recovery, 768),
    ] {
        if let Some(text) = field {
            bound_text(text, limit);
        }
    }
}

fn bound_text(text: &mut String, limit: usize) {
    if text.len() <= limit {
        return;
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push_str(" … [truncated; run verify for full text]");
}

fn serialization_error(error: serde_json::Error) -> Error {
    Error::new("serialization_failed", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_staleness_cannot_hide_a_required_input_blocker() {
        let changed = Reason::new("input_changed", Some("@repo/generated.java".into()));
        let mut reasons = vec![changed; 100];
        reasons.push(Reason::new("unavailable_root", Some("@linked/".into())));
        let report = Report {
            version: 1,
            contract_valid: true,
            input_status: codecallnavigator_core::artifact::verification::InputStatus::Stale,
            digest: "a".repeat(64),
            reasons,
        };
        let result = bounded(report).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.omitted, 0);
        assert_eq!(result.report.reasons[0].code, "unavailable_root");
        let mut report = result.report;
        report.reasons[0].detail = Some("é".repeat(9000));
        let bounded = bounded(report).unwrap();
        assert_eq!(bounded.report.reasons[0].code, "unavailable_root");
        assert!(
            bounded.report.reasons[0]
                .detail
                .as_ref()
                .unwrap()
                .contains("truncated")
        );
    }
}
