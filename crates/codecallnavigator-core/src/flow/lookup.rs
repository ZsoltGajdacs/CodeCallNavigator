//! Shared availability policy for source-derived function flows.
use super::FunctionFlowIR;
use crate::model::GraphData;

pub fn lookup<'a>(graph: &'a GraphData, symbol: &str) -> Result<&'a FunctionFlowIR, String> {
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == symbol)
        .ok_or("The selected method is no longer in this graph.")?;
    super::check_body(&node.language, node.body_unavailable)
        .map_err(|issue| issue.message().to_owned())?;
    let inventory = graph
        .flow_inventory
        .as_ref()
        .filter(|i| super::inventory_supported(u64::from(i.version)))
        .ok_or(super::INVENTORY_UNAVAILABLE)?;
    inventory
        .functions
        .iter()
        .find(|function| function.symbol == symbol)
        .ok_or_else(|| super::FUNCTION_UNAVAILABLE.into())
}
