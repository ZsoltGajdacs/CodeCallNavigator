//! Compiler-backed language adapters and shared analysis orchestration.
pub use codecallnavigator_core::{
    architecture, discovery, fingerprint, flow, graph, graphviz, method_policy, metrics, model,
    store,
};
pub mod analyzers;

/// Development resource root; packaged runs discover adjacent Resources first.
pub fn repository_root() -> std::path::PathBuf {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.canonicalize().unwrap_or(root)
}
