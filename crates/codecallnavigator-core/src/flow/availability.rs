//! Shared read-side availability facts and refresh guidance.
use super::INVENTORY_VERSION;

pub(crate) const INVENTORY_UNAVAILABLE: &str =
    "Refresh this workspace to collect function flowcharts.";
pub(crate) const FUNCTION_UNAVAILABLE: &str = "No unambiguous source flow was exported for this callable. Refresh an older graph; unsupported or ambiguous bodies remain unavailable.";

pub(crate) enum BodyIssue {
    Language,
    Body,
}
impl BodyIssue {
    pub(crate) fn message(&self) -> &'static str {
        match self {
            Self::Language => "Function flowcharts currently support Java, TypeScript and C#.",
            Self::Body => "No source body is available for this declaration.",
        }
    }
}
pub(crate) fn check_body(language: &str, body_unavailable: bool) -> Result<(), BodyIssue> {
    if !["java", "typescript", "csharp"].contains(&language) {
        return Err(BodyIssue::Language);
    }
    if body_unavailable {
        return Err(BodyIssue::Body);
    }
    Ok(())
}
pub(crate) fn inventory_supported(version: u64) -> bool {
    version == u64::from(INVENTORY_VERSION)
}
