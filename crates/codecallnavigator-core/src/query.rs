//! Public process-query contract. Graph semantics live here; loading belongs to the engine.
mod contract;
mod diagnostics;
mod operations;
mod selection;
pub use contract::*;
pub use operations::{execute, render_section};
pub use selection::{Selection, select, select_render};

pub const VERSION: u32 = 1;
pub const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_FRESHNESS_REASONS: usize = 50;
pub const MAX_FRESHNESS_REASON_BYTES: usize = 8192;

pub fn capabilities() -> serde_json::Value {
    serde_json::json!({
        "query_version": VERSION,
        "operations": ["status", "freshness", "snapshot", "inspect", "live_source", "architecture", "search", "symbol", "source", "explorer", "selection", "render_selection", "neighbors", "flow", "compare"],
        "comparison": {"version":1,"default_limit":100,"maximum_limit":500,"maximum_response_bytes":24*1024*1024,"snapshot_binding":true},
        "freshness": {"version":crate::freshness::VERSION,"streaming":true,"mode":"verified_polling","limited":true,"reconcile_milliseconds":crate::freshness::RECONCILE_MILLISECONDS,"heartbeat_milliseconds":crate::freshness::HEARTBEAT_MILLISECONDS},
        "flow_languages": ["java", "typescript"],
        "source_origins": ["analyzed", "live"],
        "generation_binding": true,
        "generation_required": ["render_selection", "inspect"],
        "default_limits": {"nodes":1500,"edges":6000,"search":50},
        "maximum_limits": {"nodes":1500,"edges":6000,"search":200,"diagnostics":diagnostics::COUNT,"diagnostic_characters":diagnostics::CHARACTERS,"freshness_reasons":MAX_FRESHNESS_REASONS,"freshness_reason_bytes":MAX_FRESHNESS_REASON_BYTES}
    })
}
