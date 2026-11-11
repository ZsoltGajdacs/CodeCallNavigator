//! Compiler-owned HTTP and navigation facts, including unresolved route blockers.
mod cache;
mod cache_clear;
mod cache_key;
mod cache_paths;
mod cache_store;
mod config;
pub use cache::{CacheStats, DecisionCache};
pub use cache_clear::clear_inference_cache;
mod contracts;
mod flow;
mod matcher;
const PROVENANCE: &str = "codegraph.boundary.v1";
mod resolve;
pub use config::{Configuration, HttpScope, Server};
pub use contracts::merge;
pub use contracts::{BoundaryInventory, BoundaryUse, RouteFact};
pub use resolve::resolve;
pub use resolve::resolve_cached;
