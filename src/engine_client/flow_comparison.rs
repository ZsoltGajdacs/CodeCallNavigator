//! Revision-bound desktop requests, independent of the currently installed graph generation.
mod changes;
mod class_graph;
mod class_source;
mod client;
mod impact;
mod model;
mod snapshots;
mod source_comparison;
mod validation;
pub use changes::{
    Change, ChangeKind, ChangeList, ChangeStatus, Hunk, LineKind, SourceDiff, SourceStatus,
};
pub use client::{BoundComparison, CatalogPage, Error};
pub use impact::{ImpactPreview, ImpactRefs, ImpactRow, installed_impact_preview};
pub use model::*;
pub use snapshots::{
    ComparisonRefs, GraphChange, GraphCursor, GraphPage, SnapshotPreparation, SnapshotStatus,
};
pub use source_comparison::BoundSourceComparison;
