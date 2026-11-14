use super::{Owner, Reason, Symbol, Unavailable};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn index(graph: &Value) -> Result<BTreeMap<&str, Option<&Value>>, Unavailable> {
    let nodes = graph
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            Unavailable::new(
                Reason::InvalidArtifact,
                "Saved artifact has no node collection",
            )
        })?;
    let mut index = BTreeMap::new();
    for node in nodes {
        let symbol = text(node, "id")?;
        if symbol.is_empty() {
            return Err(Unavailable::new(
                Reason::InvalidArtifact,
                "Saved node has no symbol identity",
            ));
        }
        index
            .entry(symbol)
            .and_modify(|node| *node = None)
            .or_insert(Some(node));
    }
    Ok(index)
}

pub(super) fn owner(value: &Value) -> Result<Owner<'_>, Unavailable> {
    Ok(Owner {
        build: text(value, "build")?,
        project: text(value, "project")?,
        file: text(value, "file")?,
        language: text(value, "language")?,
    })
}
pub(super) fn symbol(value: &Value) -> Result<Symbol<'_>, Unavailable> {
    let line = match value.get("line") {
        None => 0,
        Some(line) => line
            .as_u64()
            .and_then(|line| u32::try_from(line).ok())
            .ok_or_else(|| {
                Unavailable::new(
                    Reason::InvalidArtifact,
                    "Saved node has an invalid source line",
                )
            })?,
    };
    Ok(Symbol {
        symbol: text(value, "id")?,
        label: text(value, "label")?,
        signature: text(value, "signature")?,
        line,
        body_unavailable: body_availability(value)?,
        owner: owner(value)?,
    })
}
pub(super) fn available(node: &Symbol<'_>) -> Result<(), Unavailable> {
    crate::flow::check_body(node.owner.language, node.body_unavailable).map_err(|issue| {
        let reason = match issue {
            crate::flow::BodyIssue::Language => Reason::UnsupportedLanguage,
            crate::flow::BodyIssue::Body => Reason::BodyUnavailable,
        };
        Unavailable::new(reason, issue.message())
    })
}
fn body_availability(raw: &Value) -> Result<bool, Unavailable> {
    match raw.get("body_unavailable") {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        _ => Err(Unavailable::new(
            Reason::InvalidArtifact,
            "Saved node has an invalid body availability fact",
        )),
    }
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, Unavailable> {
    value.get(key).and_then(Value::as_str).ok_or_else(|| {
        Unavailable::new(
            Reason::InvalidArtifact,
            format!("Saved owner has missing or invalid {key}"),
        )
    })
}
