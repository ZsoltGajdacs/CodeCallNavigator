//! Digests are based on fresh facts, never previously inferred relationships.
use super::{BoundaryUse, Configuration, RouteFact};
use anyhow::Result;
use serde::Serialize;

pub(super) const FORMAT: u32 = 1;
pub(super) fn implementation() -> String {
    let mut hash = blake3::Hasher::new();
    for source in [
        include_str!("../boundary.rs"),
        include_str!("resolve.rs"),
        include_str!("matcher.rs"),
        include_str!("config.rs"),
        include_str!("contracts.rs"),
        include_str!("flow.rs"),
        include_str!("cache_key.rs"),
        include_str!("cache.rs"),
        include_str!("cache_store.rs"),
        include_str!("cache_paths.rs"),
    ] {
        hash.update(source.as_bytes());
    }
    hash.finalize().to_hex().to_string()
}
pub(super) fn digest(value: &impl Serialize) -> Result<String> {
    Ok(blake3::hash(&serde_json::to_vec(value)?)
        .to_hex()
        .to_string())
}

#[derive(Serialize)]
struct Inputs<'a> {
    format: u32,
    implementation: &'a str,
    usage: &'a BoundaryUse,
    configuration: &'a Configuration,
    engine_versions: &'a str,
    /// Includes blockers and empty membership for negative lookup dependencies.
    candidates: Vec<String>,
}
pub(super) struct Context<'a> {
    pub configuration: &'a Configuration,
    pub engine_versions: &'a str,
}
pub(super) fn key(
    usage: &BoundaryUse,
    candidates: &[&RouteFact],
    context: &Context<'_>,
    implementation: &str,
) -> Result<String> {
    let mut facts = candidates
        .iter()
        .map(|route| digest(*route))
        .collect::<Result<Vec<_>>>()?;
    facts.sort();
    digest(&Inputs {
        format: FORMAT,
        implementation,
        usage,
        configuration: context.configuration,
        engine_versions: context.engine_versions,
        candidates: facts,
    })
}
