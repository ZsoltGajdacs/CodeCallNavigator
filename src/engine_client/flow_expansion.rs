//! Generation-bound typed expansion; semantic decisions and source ownership remain in the CLI.
use super::QueryContext;
use anyhow::{Result, ensure};
use codecallnavigator::{
    flow::ExpandedFlow,
    query::{FlowExpansionRequest, Query},
};
use std::sync::atomic::AtomicBool;
impl QueryContext {
    pub fn flow_expansion_supported(&self) -> bool {
        self.client.flow_expansion_supported()
    }
    pub fn expand_flow(
        &self,
        request: FlowExpansionRequest,
        cancel: &AtomicBool,
    ) -> Result<ExpandedFlow> {
        ensure!(
            self.descriptor.generation.is_some(),
            "Expansion requires a loaded graph generation"
        );
        ensure!(
            self.client.expansion_supported(cancel)?,
            "Configured CLI does not support flow expansion; update the CLI"
        );
        let root = request.symbol.clone();
        let result: ExpandedFlow = self.request(Query::FlowExpansion(request), cancel)?;
        ensure!(
            result.root_symbol == root,
            "CLI returned another expansion root"
        );
        result.validate()?;
        Ok(result)
    }
}
