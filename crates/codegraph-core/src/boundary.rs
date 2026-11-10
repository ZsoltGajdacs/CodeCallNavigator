//! Compiler-owned HTTP and navigation facts, including unresolved route blockers.
mod config;
mod contracts;
mod flow;
mod matcher;
const PROVENANCE: &str = "codegraph.boundary.v1";
mod resolve;
pub use config::{Configuration, HttpScope, Server};
pub use contracts::merge;
pub use contracts::{BoundaryInventory, BoundaryUse, RouteFact};
pub use resolve::resolve;
