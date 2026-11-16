//! Explicit session-owned residency; consumers decide when to release flows.
use super::Loaded;
use codecallnavigator_core::{
    artifact::flow::{InventoryIndex, View},
    flow::FlowInventory,
    model::GraphData,
    query::{self, Error, FlowResourceAction, FlowResourceStatus, Query, Request, Response},
};
use std::{io::Write, sync::atomic::Ordering, time::Instant};

#[derive(Default)]
pub(super) struct Residency(Option<Resident>);

struct Resident {
    inventory: Option<FlowInventory>,
    structure: GraphData,
    index: InventoryIndex,
    retained_bytes: usize,
    load_milliseconds: f64,
}

impl Loaded {
    pub(in crate::entrypoints) fn ensure_flows(&mut self) -> query::Result<()> {
        if self.flows.0.is_some() {
            return Ok(());
        }
        let started = Instant::now();
        let inventory = self
            .captured
            .load_flows_for(&self.artifact)
            .map_err(|e| Error::new("flows_unavailable", format!("{e:#}")))?;
        // Flow ownership stays portable for saved-revision and expansion parity.
        // Only nodes and saved sources are needed; no structural edge/index copy.
        let structure = codecallnavigator_core::artifact::flow::read_structure(&self.artifact)
            .map_err(|e| Error::new("invalid_flows", e.to_string()))?;
        let index = inventory
            .as_ref()
            .map(InventoryIndex::new)
            .unwrap_or_default();
        let retained_bytes =
            estimate_bytes(inventory.as_ref())?.saturating_add(estimate_structure(&structure)?);
        if super::super::cancellation::CANCELLED.load(Ordering::Acquire) {
            return Err(Error::new("cancelled", "Flow loading cancelled"));
        }
        self.flows = Residency(Some(Resident {
            inventory,
            structure,
            index,
            retained_bytes,
            load_milliseconds: started.elapsed().as_secs_f64() * 1000.0,
        }));
        Ok(())
    }

    pub(in crate::entrypoints) fn flow_view(&self) -> query::Result<View<'_>> {
        let resident =
            self.flows.0.as_ref().ok_or_else(|| {
                Error::new("flows_unavailable", "Flow inventory has not been loaded")
            })?;
        View::from_indexed_parts(
            &resident.structure,
            resident.inventory.as_ref(),
            &resident.index,
        )
        .map_err(|e| Error::new("invalid_flows", e.to_string()))
    }

    /// Ordinary flow replies retain checkout paths; expansion retains portable ownership.
    fn hydrate_flow_reply(
        &self,
        mut result: serde_json::Value,
    ) -> query::Result<serde_json::Value> {
        let function = serde_json::from_value(result["function"].take())
            .map_err(|e| Error::new("invalid_flows", e.to_string()))?;
        let mut selected = FlowInventory {
            version: codecallnavigator_core::flow::INVENTORY_VERSION,
            functions: vec![function],
        };
        self.artifact
            .hydrate_flow_inventory(&mut selected, &self.roots)
            .map_err(|e| Error::new("invalid_flows", e.to_string()))?;
        let function = selected.functions.pop().expect("one selected function");
        result["chart"]["file"] = serde_json::json!(function.file);
        result["source"]["file"] = serde_json::json!(function.file);
        result["function"] = serde_json::to_value(function)
            .map_err(|e| Error::new("serialization_failed", e.to_string()))?;
        Ok(result)
    }

    fn flow_status(&self) -> FlowResourceStatus {
        match &self.flows.0 {
            Some(resident) => FlowResourceStatus {
                loaded: true,
                functions: resident
                    .inventory
                    .as_ref()
                    .map_or(0, |inventory| inventory.functions.len()),
                retained_bytes: resident.retained_bytes,
                load_milliseconds: resident.load_milliseconds,
            },
            None => FlowResourceStatus {
                loaded: false,
                functions: 0,
                retained_bytes: 0,
                load_milliseconds: 0.0,
            },
        }
    }

    fn resource_action(&mut self, action: FlowResourceAction) -> query::Result<serde_json::Value> {
        match action {
            FlowResourceAction::Load => self.ensure_flows()?,
            FlowResourceAction::Unload => self.flows = Residency::default(),
            FlowResourceAction::Status => (),
        }
        serde_json::to_value(self.flow_status())
            .map_err(|e| Error::new("serialization_failed", e.to_string()))
    }

    pub(in crate::entrypoints) fn flow_response(&mut self, request: &Request) -> Option<Response> {
        let result = match &request.query {
            Query::FlowResources { action } => self.resource_action(*action),
            Query::Flow { symbol } => self
                .refresh_freshness()
                .and_then(|()| self.ensure_flows())
                .and_then(|()| {
                    let view = self.flow_view()?;
                    let result = query::function_flow(&view, symbol)?;
                    self.hydrate_flow_reply(result)
                }),
            _ => return None,
        };
        Some(super::envelope(Some(self), result))
    }
}

/// Approximate IR/index storage without allocating a second serialized inventory.
/// This is a diagnostic estimate; it does not claim allocator or RSS precision.
fn estimate_bytes(inventory: Option<&FlowInventory>) -> query::Result<usize> {
    let Some(inventory) = inventory else {
        return Ok(0);
    };
    let mut bytes = ByteCount(0);
    serde_json::to_writer(&mut bytes, inventory)
        .map_err(|e| Error::new("serialization_failed", e.to_string()))?;
    let functions = inventory.functions.capacity()
        * std::mem::size_of::<codecallnavigator_core::flow::FunctionFlowIR>();
    let symbols: usize = inventory
        .functions
        .iter()
        .map(|function| function.symbol.len() + 64)
        .sum();
    Ok(bytes.0.saturating_add(functions).saturating_add(symbols))
}

struct ByteCount(usize);
impl Write for ByteCount {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn estimate_structure(structure: &GraphData) -> query::Result<usize> {
    let mut bytes = ByteCount(0);
    serde_json::to_writer(&mut bytes, structure)
        .map_err(|e| Error::new("serialization_failed", e.to_string()))?;
    Ok(bytes.0.saturating_add(
        structure.nodes.capacity() * std::mem::size_of::<codecallnavigator_core::model::Node>(),
    ))
}
