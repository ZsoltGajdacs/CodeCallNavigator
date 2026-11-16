use super::{Owner, Reason, Symbol, Unavailable};
use crate::model::Node;
use std::collections::BTreeMap;

pub(super) fn index(nodes: &[Node]) -> Result<BTreeMap<String, Option<usize>>, Unavailable> {
    let mut index = BTreeMap::new();
    for (position, node) in nodes.iter().enumerate() {
        if node.id.is_empty() {
            return Err(Unavailable::new(
                Reason::InvalidArtifact,
                "Saved node has no symbol identity",
            ));
        }
        index
            .entry(node.id.clone())
            .and_modify(|node| *node = None)
            .or_insert(Some(position));
    }
    Ok(index)
}

pub(super) fn symbol(node: &Node) -> Symbol<'_> {
    Symbol {
        symbol: &node.id,
        label: &node.label,
        signature: &node.signature,
        line: node.line,
        body_unavailable: node.body_unavailable,
        owner: Owner {
            build: &node.build,
            project: &node.project,
            file: &node.file,
            language: &node.language,
        },
    }
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
