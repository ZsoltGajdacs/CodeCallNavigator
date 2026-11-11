//! Desktop wording for shared semantic categories; core owns their interpretation.
use codecallnavigator::model::{RelationshipKind, TargetResolution};

pub(crate) fn kind(kind: RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Call => "Calls",
        RelationshipKind::FunctionReference => "Function references",
        RelationshipKind::FrameworkLifecycle => "Framework / lifecycle",
        RelationshipKind::HttpNavigation => "HTTP / navigation",
        RelationshipKind::Other => "Other relationships",
    }
}

pub(crate) fn kind_help(kind: RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Call => {
            "Ordinary calls, construction, accessor invocations and inferred invocation targets."
        }
        RelationshipKind::FunctionReference => {
            "Functions passed, stored or registered. A reference does not establish invocation."
        }
        RelationshipKind::FrameworkLifecycle => {
            "Framework dispatch, rendering and lifecycle relationships. These are static possibilities, not observed execution."
        }
        RelationshipKind::HttpNavigation => {
            "Matched HTTP requests and navigation across application boundaries."
        }
        RelationshipKind::Other => {
            "Structural, bootstrap, field-access and unclassified analyzer relationships. These do not establish invocation."
        }
    }
}

pub(crate) fn resolution(resolution: TargetResolution) -> &'static str {
    match resolution {
        TargetResolution::Direct => "Directly resolved",
        TargetResolution::VirtualDispatch => "Virtual / interface dispatch",
        TargetResolution::Possible => "Possible targets",
        TargetResolution::Other => "Other / unspecified",
    }
}

pub(crate) fn resolution_help(resolution: TargetResolution) -> &'static str {
    match resolution {
        TargetResolution::Direct => {
            "Directly resolved targets of any relationship type. A resolved target does not prove execution."
        }
        TargetResolution::VirtualDispatch => {
            "Targets declared as virtual or interface methods, including function references. An override may execute at runtime."
        }
        TargetResolution::Possible => {
            "Possible targets reported by static analysis. Their invocation is not guaranteed."
        }
        TargetResolution::Other => {
            "Analyzer-reported, delegate and otherwise unspecified target resolution. Inspect the relationship for its original analyzer facts."
        }
    }
}
