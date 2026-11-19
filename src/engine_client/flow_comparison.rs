//! Revision-bound desktop requests, independent of the currently installed graph generation.
mod client;
mod model;
mod snapshots;
mod validation;
pub use client::{BoundComparison, CatalogPage, Error};
pub use model::*;
pub use snapshots::{
    ComparisonRefs, GraphChange, GraphCursor, GraphPage, SnapshotPreparation, SnapshotStatus,
};
