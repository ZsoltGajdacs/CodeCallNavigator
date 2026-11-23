use super::{
    Facet, Status,
    snapshot::{FileId, Snapshot, is_method},
};
use crate::model::Node;
use serde_json::Value;
use std::{collections::BTreeSet, ops::RangeInclusive};

pub(super) struct Method<'a> {
    pub(crate) before: Option<&'a Node>,
    pub(crate) after: Option<&'a Node>,
    pub(crate) facts: Vec<Facet>,
}

impl<'a> Method<'a> {
    pub fn node(&self) -> &'a Node {
        self.after
            .or(self.before)
            .expect("a changed method has an endpoint")
    }
    pub fn status(&self) -> Status {
        Status::between(self.before.is_some(), self.after.is_some())
    }
}

pub(super) fn changes<'a>(before: &Snapshot<'a>, after: &Snapshot<'a>) -> Vec<Method<'a>> {
    let symbols: BTreeSet<_> = before
        .symbols
        .keys()
        .chain(after.symbols.keys())
        .copied()
        .collect();
    symbols
        .into_iter()
        .filter_map(|id| {
            let old = before.node(id).filter(|node| is_method(node));
            let new = after.node(id).filter(|node| is_method(node));
            let facts = match (old, new) {
                (Some(old), Some(new)) => modification(before, after, old, new),
                (None, None) => return None,
                _ => vec![Facet::Declaration],
            };
            (!facts.is_empty()).then_some(Method {
                before: old,
                after: new,
                facts,
            })
        })
        .collect()
}

fn modification(before: &Snapshot<'_>, after: &Snapshot<'_>, old: &Node, new: &Node) -> Vec<Facet> {
    let mut facts = Vec::new();
    if declaration_changed(old, new) {
        facts.push(Facet::Declaration);
    }
    let id = old.id.as_str();
    if flow_changed(before, after, id) {
        facts.push(Facet::Flow);
    }
    if body_source(before, id)
        .zip(body_source(after, id))
        .is_some_and(|(old, new)| old != new)
    {
        facts.push(Facet::Source);
    }
    facts
}

fn body_source<'a>(snapshot: &'a Snapshot<'_>, id: &str) -> Option<&'a str> {
    let (content, span) = snapshot.flow_source(id)?;
    content.get(span.start..span.end)
}

fn declaration_changed(old: &Node, new: &Node) -> bool {
    old.signature != new.signature
        || old.label != new.label
        || old.kind != new.kind
        || old.visibility != new.visibility
        || old.is_test != new.is_test
        || old.is_generated != new.is_generated
        || old.body_unavailable != new.body_unavailable
}

fn flow_changed(before: &Snapshot<'_>, after: &Snapshot<'_>, id: &str) -> bool {
    if before.calls.get(id) != after.calls.get(id) {
        return true;
    }
    let old = before.functions.get(id);
    let new = after.functions.get(id);
    if old == new {
        return false;
    }
    match old.zip(new) {
        Some((old, new)) => match (serde_json::to_value(old), serde_json::to_value(new)) {
            (Ok(old), Ok(new)) => !equal_without_spans(&old, &new),
            _ => true,
        },
        None => true,
    }
}

fn equal_without_spans(old: &Value, new: &Value) -> bool {
    match (old, new) {
        (Value::Object(old), Value::Object(new)) => {
            old.len() == new.len()
                && old.iter().all(|(key, value)| {
                    new.get(key).is_some_and(|other| {
                        (key == "span"
                            && value.get("start").is_some()
                            && value.get("end").is_some())
                            || equal_without_spans(value, other)
                    })
                })
        }
        (Value::Array(old), Value::Array(new)) => {
            old.len() == new.len() && old.iter().zip(new).all(|(a, b)| equal_without_spans(a, b))
        }
        _ => old == new,
    }
}

pub(super) fn covered_lines(
    snapshot: &Snapshot<'_>,
    file: FileId<'_>,
    changed: &BTreeSet<&str>,
) -> Vec<RangeInclusive<usize>> {
    snapshot
        .file_nodes
        .get(&file)
        .into_iter()
        .flatten()
        .filter_map(|index| {
            let node = &snapshot.graph.nodes[*index];
            if !changed.contains(node.id.as_str()) {
                return None;
            }
            let declaration_line = node.line as usize;
            if let Some((content, span)) = snapshot.flow_source(&node.id) {
                let start = line_number(content, span.start);
                let end = line_number(content, span.end - 1);
                let start = if declaration_line > 0 {
                    start.min(declaration_line)
                } else {
                    start
                };
                return Some(start..=end);
            }
            (declaration_line > 0).then_some(declaration_line..=declaration_line)
        })
        .collect()
}

fn line_number(content: &str, byte: usize) -> usize {
    content.as_bytes()[..byte]
        .iter()
        .filter(|value| **value == b'\n')
        .count()
        + 1
}
