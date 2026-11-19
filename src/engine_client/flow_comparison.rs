//! Revision-bound desktop requests, independent of the currently installed graph generation.
mod client;
mod impact;
mod model;
mod snapshots;
mod validation;
pub use client::{BoundComparison, CatalogPage, Error};
pub use impact::{ImpactPreview, ImpactRefs, ImpactRow, installed_impact_preview};
pub use model::*;
pub use snapshots::{
    ComparisonRefs, GraphChange, GraphCursor, GraphPage, SnapshotPreparation, SnapshotStatus,
};
