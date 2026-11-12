//! Public process-query contract. Graph semantics live here; loading belongs to the engine.
mod contract;
mod operations;
mod selection;
pub use contract::*;
pub use operations::execute;
pub use selection::{Selection, select, select_render};

pub const VERSION: u32 = 1;

pub fn capabilities() -> serde_json::Value {
    serde_json::json!({
        "query_version": VERSION,
        "operations": ["status", "snapshot", "search", "symbol", "source", "explorer", "selection", "render_selection", "neighbors", "flow"],
        "flow_languages": ["java", "typescript"],
        "source_origins": ["analyzed", "live"],
        "generation_binding": true,
        "generation_required": ["render_selection"],
        "default_limits": {"nodes":1500,"edges":6000,"search":50},
        "maximum_limits": {"nodes":1500,"edges":6000,"search":200}
    })
}
