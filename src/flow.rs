//! Source-derived function flow, separate from the analyzer's inter-method graph.
mod builder;
mod inventory;
mod layout;
mod model;
mod prune;
pub use builder::build;
pub use inventory::{merge_inventory, visit_calls_mut};
pub use layout::layout;
pub use model::*;
pub const INVENTORY_VERSION: u32 = 1;
pub const MAX_STEPS: usize = 1_500;
pub const MAX_LINKS: usize = 6_000;
