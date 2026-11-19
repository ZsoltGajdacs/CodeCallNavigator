//! Immutable compressed parts, captured together under a brief publication guard.
mod captured;
mod codec;
mod companions;
pub mod local_snapshots;
mod publication;
#[cfg(test)]
mod tests;

pub use captured::Captured;
pub use codec::{EncodedParts, Manifest, Part};
#[derive(Debug)]
pub struct MissingGraph;

impl std::fmt::Display for MissingGraph {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("No saved working graph in the local library")
    }
}

impl std::error::Error for MissingGraph {}

pub const SCHEMA: &str = "codecallnavigator.graph.manifest.v2";
