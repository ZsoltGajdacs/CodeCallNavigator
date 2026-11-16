use super::{Owner, Reason, Unavailable};
use crate::{
    architecture::ArchitectureFacts,
    flow::{FlowInventory, FunctionFlowIR},
};
use std::collections::BTreeMap;

pub(super) fn index(inventory: Option<&FlowInventory>) -> BTreeMap<String, Option<usize>> {
    let mut index = BTreeMap::new();
    if let Some(inventory) = inventory {
        for (position, function) in inventory.functions.iter().enumerate() {
            index
                .entry(function.symbol.clone())
                .and_modify(|position| *position = None)
                .or_insert(Some(position));
        }
    }
    index
}

pub(super) fn function<'a>(
    inventory: Option<&'a FlowInventory>,
    index: &BTreeMap<String, Option<usize>>,
    symbol: &str,
) -> Result<&'a FunctionFlowIR, Unavailable> {
    let inventory = inventory.ok_or_else(|| {
        Unavailable::new(Reason::MissingInventory, crate::flow::INVENTORY_UNAVAILABLE)
    })?;
    if !crate::flow::inventory_supported(u64::from(inventory.version)) {
        return Err(Unavailable::new(
            Reason::InventoryVersion,
            crate::flow::INVENTORY_UNAVAILABLE,
        ));
    }
    match index.get(symbol) {
        None => Err(Unavailable::new(
            Reason::MissingFunction,
            crate::flow::FUNCTION_UNAVAILABLE,
        )),
        Some(None) => Err(Unavailable::new(
            Reason::DuplicateFunction,
            "Saved symbol has multiple function flows",
        )),
        Some(Some(position)) => Ok(&inventory.functions[*position]),
    }
}

pub(super) fn owner(function: &FunctionFlowIR, expected: Owner<'_>) -> Result<(), Unavailable> {
    let actual = Owner {
        build: &function.build,
        project: &function.project,
        file: &function.file,
        language: &function.language,
    };
    if actual != expected {
        return Err(Unavailable::new(
            Reason::FunctionOwnership,
            "Saved function does not match its symbol's build, project, file and language",
        ));
    }
    Ok(())
}

pub(super) fn source<'a>(
    facts: Option<&'a ArchitectureFacts>,
    owner: Owner<'_>,
) -> Result<&'a str, Unavailable> {
    let missing = || {
        Unavailable::new(
            Reason::MissingSource,
            "Refresh to collect analyzed source for this declaration",
        )
    };
    if owner.file.is_empty() {
        return Err(missing());
    }
    let facts = facts.ok_or_else(missing)?;
    if facts.version != crate::architecture::FACTS_VERSION {
        return Err(Unavailable::new(
            Reason::InvalidSource,
            "Unsupported saved analyzed source facts version",
        ));
    }
    let mut files = facts.files.iter().filter(|file| {
        file.build == owner.build && file.project == owner.project && file.file == owner.file
    });
    let first = files.next().ok_or_else(missing)?;
    if files.next().is_some() {
        return Err(Unavailable::new(
            Reason::DuplicateSource,
            "Analyzed source has multiple owners for this build, project and file",
        ));
    }
    Ok(&first.content)
}

/// FileFacts uses defaults for analyzer ingestion; saved source absence is explicit.
pub(super) fn decode_sources(
    value: &serde_json::Value,
) -> Result<Option<ArchitectureFacts>, Unavailable> {
    use serde::Deserialize;
    if let Some(files) = value.get("files").and_then(serde_json::Value::as_array)
        && files.iter().any(|file| file.get("content").is_none())
    {
        return Err(Unavailable::new(
            Reason::MissingSource,
            "Refresh to collect analyzed source for this declaration",
        ));
    }
    Option::<ArchitectureFacts>::deserialize(value)
        .map_err(|error| Unavailable::new(Reason::InvalidSource, error.to_string()))
}
