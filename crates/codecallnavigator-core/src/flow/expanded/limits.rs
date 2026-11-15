//! Validated execution allowances, independent of the selection-envelope bounds.
use crate::flow::{MAX_LINKS, MAX_SOURCE_BYTES, MAX_STEPS};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
/// Defaults: depth 8, reached requests 64, 1500 steps, 6000 links and 16 MiB
/// of unique owned source bytes. Hard ceilings are 32/128/1500/6000/16 MiB.
/// Depth and requests may be zero; graph and source allowances must be positive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExpansionLimits {
    pub depth: usize,
    pub requests: usize,
    pub steps: usize,
    pub links: usize,
    pub source_bytes: u64,
}
impl Default for ExpansionLimits {
    fn default() -> Self {
        Self {
            depth: 8,
            requests: 64,
            steps: MAX_STEPS,
            links: MAX_LINKS,
            source_bytes: MAX_SOURCE_BYTES,
        }
    }
}
impl ExpansionLimits {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(self.depth <= 32, "Expansion depth exceeds hard ceiling 32");
        ensure!(
            self.requests <= 128,
            "Expansion requests exceed hard ceiling 128"
        );
        ensure!(
            (1..=MAX_STEPS).contains(&self.steps),
            "Expansion steps must be positive and at most {MAX_STEPS}"
        );
        ensure!(
            (1..=MAX_LINKS).contains(&self.links),
            "Expansion links must be positive and at most {MAX_LINKS}"
        );
        ensure!(
            (1..=MAX_SOURCE_BYTES).contains(&self.source_bytes),
            "Expansion source bytes must be positive and at most {MAX_SOURCE_BYTES}"
        );
        Ok(())
    }
}
