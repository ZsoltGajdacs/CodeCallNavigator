//! Shared availability policy for source-derived function flows.
use super::{FunctionFlowIR, INVENTORY_VERSION};
use crate::model::GraphData;

pub fn lookup<'a>(graph: &'a GraphData, symbol: &str) -> Result<&'a FunctionFlowIR, String> {
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == symbol)
        .ok_or("The selected method is no longer in this graph.")?;
    if !["java", "typescript"].contains(&node.language.as_str()) {
        return Err("Function flowcharts currently support Java and TypeScript. C# support is tracked in TASK-22.".into());
    }
    if node.body_unavailable {
        return Err("No source body is available for this declaration.".into());
    }
    let inventory = graph
        .flow_inventory
        .as_ref()
        .filter(|i| i.version == INVENTORY_VERSION)
        .ok_or("Refresh this workspace to collect function flowcharts.")?;
    inventory.functions.iter().find(|f|f.symbol==symbol).ok_or_else(||"No unambiguous source flow was exported for this callable. Refresh an older graph; unsupported or ambiguous bodies remain unavailable.".into())
}
