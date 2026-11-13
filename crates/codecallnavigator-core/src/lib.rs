//! Shared analysis contracts and graph operations, independent of the desktop.
pub mod architecture;
pub mod artifact;
pub mod boundary;
pub mod discovery;
pub mod fingerprint;
pub mod flow;
pub mod freshness;
pub mod graph;
pub mod graphviz;
pub mod method_policy;
pub mod metrics;
pub mod model;
#[cfg(unix)]
pub mod process_ownership;
pub mod protocol;
pub mod query;
pub mod refresh;
pub mod search;
pub mod store;
