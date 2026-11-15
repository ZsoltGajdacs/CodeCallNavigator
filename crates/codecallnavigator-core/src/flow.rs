//! Source-derived function flow, separate from the analyzer's inter-method graph.
mod availability;
mod builder;
mod checked;
mod expanded;
mod inventory;
mod layout;
mod lookup;
mod metadata;
mod model;
mod parts;
pub(crate) use metadata::validate_metadata;
mod prune;
mod source;
pub(crate) use availability::{
    BodyIssue, FUNCTION_UNAVAILABLE, INVENTORY_UNAVAILABLE, check_body, inventory_supported,
};
pub use builder::build;
pub use checked::{MAX_SOURCE_BYTES, checked_chart};
pub use expanded::*;
pub use inventory::{merge_inventory, visit_calls_mut};
pub use layout::{layout, layout_cancellable};
pub use lookup::lookup;
pub use model::*;
pub use source::validate_source;
pub const INVENTORY_VERSION: u32 = 1;
pub const MAX_STEPS: usize = 1_500;
pub const MAX_LINKS: usize = 6_000;
