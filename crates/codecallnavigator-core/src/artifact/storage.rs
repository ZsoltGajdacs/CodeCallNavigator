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
pub(crate) use publication::publish;

pub const SCHEMA: &str = "codecallnavigator.graph.manifest.v2";
