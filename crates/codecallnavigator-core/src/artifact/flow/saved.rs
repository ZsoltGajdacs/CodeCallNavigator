use super::{Owner, Reason, Unavailable, ownership};
use serde_json::Value;

pub(super) fn function<'a>(graph: &'a Value, symbol: &str) -> Result<&'a Value, Unavailable> {
    let inventory = graph
        .get("flow_inventory")
        .filter(|value| !value.is_null())
        .ok_or_else(|| {
            Unavailable::new(Reason::MissingInventory, crate::flow::INVENTORY_UNAVAILABLE)
        })?;
    if !inventory
        .get("version")
        .and_then(Value::as_u64)
        .is_some_and(crate::flow::inventory_supported)
    {
        return Err(Unavailable::new(
            Reason::InventoryVersion,
            crate::flow::INVENTORY_UNAVAILABLE,
        ));
    }
    let functions = inventory
        .get("functions")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            Unavailable::new(
                Reason::InvalidFlow,
                "Saved function flow inventory has no function collection",
            )
        })?;
    unique(
        functions
            .iter()
            .filter(|value| value.get("symbol").and_then(Value::as_str) == Some(symbol)),
        Reason::DuplicateFunction,
        "Saved symbol has multiple function flows",
    )?
    .ok_or_else(|| Unavailable::new(Reason::MissingFunction, crate::flow::FUNCTION_UNAVAILABLE))
}

pub(super) fn owner(function: &Value, expected: Owner<'_>) -> Result<(), Unavailable> {
    let actual = ownership::owner(function)
        .map_err(|error| Unavailable::new(Reason::FunctionOwnership, error.message))?;
    if actual != expected {
        return Err(Unavailable::new(
            Reason::FunctionOwnership,
            "Saved function does not match its symbol's build, project, file and language",
        ));
    }
    Ok(())
}

pub(super) fn source<'a>(graph: &'a Value, owner: Owner<'_>) -> Result<&'a str, Unavailable> {
    let missing = || {
        Unavailable::new(
            Reason::MissingSource,
            "Refresh to collect analyzed source for this declaration",
        )
    };
    if owner.file.is_empty() {
        return Err(missing());
    }
    let facts = graph
        .get("architecture")
        .filter(|value| !value.is_null())
        .ok_or_else(missing)?;
    if facts.get("version").and_then(Value::as_u64)
        != Some(u64::from(crate::architecture::FACTS_VERSION))
    {
        return Err(Unavailable::new(
            Reason::InvalidSource,
            "Unsupported saved analyzed source facts version",
        ));
    }
    let files = facts
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(missing)?;
    let file = unique(
        files.iter().filter(|file| same_source(file, owner)),
        Reason::DuplicateSource,
        "Analyzed source has multiple owners for this build, project and file",
    )?
    .ok_or_else(missing)?;
    file.get("content")
        .ok_or_else(missing)?
        .as_str()
        .ok_or_else(|| {
            Unavailable::new(
                Reason::InvalidSource,
                "Analyzed source content is not UTF-8 text",
            )
        })
}
fn same_source(file: &Value, owner: Owner<'_>) -> bool {
    [
        ("build", owner.build),
        ("project", owner.project),
        ("file", owner.file),
    ]
    .into_iter()
    .all(|(key, expected)| file.get(key).and_then(Value::as_str) == Some(expected))
}
fn unique<'a>(
    mut values: impl Iterator<Item = &'a Value>,
    reason: Reason,
    message: &str,
) -> Result<Option<&'a Value>, Unavailable> {
    let first = values.next();
    if values.next().is_some() {
        return Err(Unavailable::new(reason, message));
    }
    Ok(first)
}
