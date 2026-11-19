//! Public process-query contract. Graph semantics live here; loading belongs to the engine.
mod contract;
mod diagnostics;
mod expansion;
mod operations;
mod selection;
pub use contract::*;
pub use expansion::{expand_flow, expand_flow_view};
pub use operations::{execute, function_flow, render_section};
pub use selection::{Selection, select, select_render};

pub const VERSION: u32 = 2;
pub const MAX_SOURCE_BYTES: u64 = crate::flow::MAX_SOURCE_BYTES;
pub const MAX_FRESHNESS_REASONS: usize = 50;
pub const MAX_FRESHNESS_REASON_BYTES: usize = 8192;

pub fn capabilities() -> serde_json::Value {
    serde_json::json!({
        "query_version": VERSION,
        "operations": ["flow_resources", "status", "freshness", "snapshot", "inspect", "live_source", "architecture", "search", "symbol", "source", "explorer", "selection", "render_selection", "neighbors", "flow", "compare", "flow_comparison", "flow_expansion", "snapshot_status"],
        "local_snapshots": {"version":1,"status_query":"snapshot_status","prepare_command":"prepare_snapshot","cancel_command":"cancel"},
        "automatic_refresh": {"version":crate::refresh::VERSION,"activity_version":crate::refresh::VERSION,"observe_flag":"--auto-refresh","initial_session_version":crate::refresh::OBSERVE_SESSION_VERSION,"debounce_milliseconds":crate::refresh::DEBOUNCE_MILLISECONDS,"cancel_command":"cancel_refresh"},
        "analysis_session": {"version":crate::refresh::SESSION_VERSION,"command":"analysis-session","activity_version":crate::refresh::VERSION,"cancel_command":"cancel_refresh"},
        "flow_comparison": {"version":1,"selections":["catalog","detail"],"default_limit":50,"maximum_limit":500,"maximum_text_bytes":1024,"maximum_symbol_bytes":16384,"maximum_response_bytes":crate::artifact::difference::MAX_RESPONSE_BYTES,"maximum_source_bytes":MAX_SOURCE_BYTES,"snapshot_binding":true,"correspondence":"exact_identity","manual_counterpart":true},
        "comparison": {"version":1,"default_limit":100,"maximum_limit":500,"maximum_response_bytes":24*1024*1024,"snapshot_binding":true},
        "freshness": {"version":crate::freshness::VERSION,"streaming":true,"mode":"verified_polling","limited":true,"reconcile_milliseconds":crate::freshness::RECONCILE_MILLISECONDS,"heartbeat_milliseconds":crate::freshness::HEARTBEAT_MILLISECONDS},
        "flow_expansion": {"version":1,"expected_generation":true,"maximum_selections":128,"maximum_occurrence_depth":32,"maximum_identity_bytes":1024*1024,"maximum_response_bytes":crate::artifact::difference::MAX_RESPONSE_BYTES,"default_limits":crate::flow::ExpansionLimits::default(),"maximum_limits":{"depth":32,"requests":128,"steps":crate::flow::MAX_STEPS,"links":crate::flow::MAX_LINKS,"source_bytes":MAX_SOURCE_BYTES}},
        "flow_languages": ["java", "typescript", "csharp"],
        "source_origins": ["analyzed", "live"],
        "generation_binding": true,
        "flow_resources": {"version":1,"actions":["load","unload","status"],"expected_generation":true},
        "generation_required": ["flow_resources","render_selection", "inspect", "flow_expansion"],
        "default_limits": {"nodes":1500,"edges":6000,"search":50},
        "maximum_limits": {"nodes":1500,"edges":6000,"search":200,"diagnostics":diagnostics::COUNT,"diagnostic_characters":diagnostics::CHARACTERS,"freshness_reasons":MAX_FRESHNESS_REASONS,"freshness_reason_bytes":MAX_FRESHNESS_REASON_BYTES}
    })
}
