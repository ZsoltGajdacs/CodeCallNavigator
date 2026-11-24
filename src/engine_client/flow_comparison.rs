//! Revision-bound desktop requests, independent of the currently installed graph generation.
mod changes;
mod class_graph;
mod class_source;
mod client;
mod model;
mod snapshots;
mod source_comparison;
mod validation;
pub use changes::{
    Change, ChangeKind, ChangeList, ChangeStatus, Hunk, LineKind, SourceDiff, SourceStatus,
};
pub(crate) use class_graph::ClassDetail;
pub use client::{BoundComparison, Error};
pub use model::*;
pub use snapshots::{ComparisonRefs, SnapshotPreparation, SnapshotStatus};
pub use source_comparison::BoundSourceComparison;
