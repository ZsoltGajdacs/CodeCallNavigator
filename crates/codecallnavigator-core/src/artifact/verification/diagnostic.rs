//! Actionable input diagnostics shared by observation, scheduling and consumers.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reason {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<String>,
}

impl Reason {
    pub fn new(code: &str, path: Option<String>) -> Self {
        let (detail, recovery) = explanation(code);
        Self {
            code: code.into(),
            path,
            operation: None,
            detail: Some(detail.into()),
            recovery: Some(recovery.into()),
        }
    }

    pub fn failure(
        code: &str,
        path: Option<String>,
        operation: &str,
        error: &anyhow::Error,
    ) -> Self {
        let mut reason = Self::new(code, path);
        reason.operation = Some(operation.into());
        reason.detail = Some(format!("{error:#}"));
        reason
    }

    pub fn is_blocker(&self) -> bool {
        !matches!(self.code.as_str(), "input_changed" | "input_missing")
    }

    pub fn describe(&self) -> String {
        let mut parts = vec![self.code.clone()];
        if let Some(path) = &self.path {
            parts.push(path.clone());
        }
        if let Some(operation) = &self.operation {
            parts.push(operation.clone());
        }
        let (detail, recovery) = explanation(&self.code);
        parts.push(self.detail.as_deref().unwrap_or(detail).into());
        parts.push(self.recovery.as_deref().unwrap_or(recovery).into());
        parts.join("\n")
    }

    /// Normalize before bounding so duplicated scans cannot crowd out blockers.
    pub fn normalize(reasons: &mut Vec<Self>) {
        reasons.sort_by(|a, b| {
            (!a.is_blocker(), &a.code, &a.path, &a.operation, &a.detail).cmp(&(
                !b.is_blocker(),
                &b.code,
                &b.path,
                &b.operation,
                &b.detail,
            ))
        });
        reasons.dedup();
    }
}

fn explanation(code: &str) -> (&'static str, &'static str) {
    match code {
        "input_changed" => (
            "This recorded input differs from the saved analysis.",
            "Refresh to analyze the current saved files.",
        ),
        "input_missing" => (
            "This previously recorded input is now absent. Its absence was observed successfully. This change makes the saved graph stale; it does not by itself make coverage incomplete.",
            "Refresh to analyze the current files. Automatic refresh may attempt analysis; if the build still requires the removed input, analysis will report that failure and preserve the previous graph.",
        ),
        "missing_required_input" => (
            "A previously recorded required input is missing. Complete coverage cannot be established.",
            "Restore the required input, or perform a manual Refresh if the build intentionally removed it. Retry freshness only checks observation.",
        ),
        "unreadable_input" | "unreadable_link" => (
            "A required input could not be read; complete coverage cannot be established.",
            "Restore the required path or link target and read/search permissions, then Retry freshness. Retry checks inputs; it does not rebuild the graph.",
        ),
        "unfollowed_directory_link" => (
            "A directory link was encountered without a recorded required-input scope for its target.",
            "Perform a manual Refresh with the updated engine to regenerate input scopes; do not remove a build-required link.",
        ),
        "unavailable_root" | "unmapped_input" => (
            "A required input root is unavailable or lacks a portable root mapping.",
            "Restore the checkout/dependency location or configure the required root mapping, then Refresh to regenerate provenance.",
        ),
        "legacy_input_policy" | "legacy_csharp_input_policy" => (
            "This graph uses an older input-observation policy. Automatic refresh requires regenerated input records.",
            "Perform one manual Refresh with the updated engine. Clear cache is not required; the previous graph remains available if analysis fails.",
        ),
        "invalid_contract" => (
            "The saved graph metadata or one of its referenced objects failed validation.",
            "Use a compatible engine and perform a manual Refresh. Preserve the existing artifact when investigating corruption.",
        ),
        "missing_analysis" | "no_observed_inputs" => (
            "The saved graph has no usable recorded input inventory.",
            "Perform a manual Refresh to record analysis inputs.",
        ),
        "unreadable_scope" => (
            "A required directory could not be traversed completely.",
            "Restore access to the directory and its required children, then Retry freshness.",
        ),
        _ => (
            "The engine cannot verify this recorded analysis condition.",
            "Inspect the detailed CLI verification report and resolve this condition before relying on automatic refresh.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_wire_reasons_gain_explanations_and_blockers_sort_first() {
        let legacy: Reason =
            serde_json::from_str(r#"{"code":"unreadable_input","path":"@repo/required"}"#).unwrap();
        assert!(legacy.describe().contains("permissions"));
        let mut reasons = vec![Reason::new("input_changed", None), legacy.clone(), legacy];
        Reason::normalize(&mut reasons);
        assert_eq!(reasons.len(), 2);
        assert_eq!(reasons[0].code, "unreadable_input");
        assert_eq!(reasons[1].code, "input_changed");
    }
}
