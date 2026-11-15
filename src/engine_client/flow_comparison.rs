//! Revision-bound desktop requests, independent of the currently installed graph generation.
mod client;
mod model;
mod validation;
pub use client::{BoundComparison, CatalogPage, Error};
pub use model::*;
