//! Read-side flow preparation; publication validates ranges without the display-size cap.
use super::{FlowChart, FunctionFlowIR};
use anyhow::{Result, anyhow, ensure};

pub const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

pub fn checked_chart(function: &FunctionFlowIR, content: &str) -> Result<FlowChart> {
    // Legacy Flow reports the outer validation message, and the complete build error chain.
    super::validate_source(function, content).map_err(|error| anyhow!(error.to_string()))?;
    ensure!(
        content.len() as u64 <= MAX_SOURCE_BYTES,
        "The analyzed source exceeds the 16 MiB flow source limit"
    );
    super::build(function)
}
