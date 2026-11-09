//! C# inclusion decisions, independent of Roslyn extraction and node construction.
use super::AtlasNode;
#[cfg(test)]
use super::DeclarationFacts;
use crate::model::GraphData;
use std::collections::HashSet;

pub(super) fn excluded(node: &AtlasNode) -> bool {
    let Some(facts) = &node.declaration else {
        return true; // Fields and delegate targets are structural compiler symbols.
    };
    !facts.has_source_declaration
        || facts.is_implicit
        || facts.is_object_member
        || matches!(
            facts.callable_kind.as_str(),
            "Constructor" | "StaticConstructor"
        )
        || matches!(
            node.kind.as_str(),
            "constructor" | "cctor" | "fieldinit" | "field"
        )
}

pub(super) fn finalize_entries(graph: &mut GraphData) {
    let called: HashSet<_> = graph.edges.iter().map(|edge| edge.to).collect();
    let Some(entries) = &mut graph.entry_points else {
        return;
    };
    entries.candidates = graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, node)| node.kind != "toplevel" && !called.contains(&(*index as u32)))
        .map(|(_, node)| node.id.clone())
        .collect();
    entries.candidates.sort();
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(kind: &str, callable: &str) -> AtlasNode {
        AtlasNode {
            id: "id".into(),
            kind: kind.into(),
            container: "Type".into(),
            name: "member".into(),
            location: "file:1".into(),
            declaration: Some(DeclarationFacts {
                has_source_declaration: true,
                is_implicit: false,
                callable_kind: callable.into(),
                is_object_member: false,
            }),
        }
    }
    #[test]
    fn constructors_and_object_members_are_excluded_but_source_accessors_remain() {
        assert!(excluded(&node("constructor", "Constructor")));
        assert!(excluded(&node("cctor", "StaticConstructor")));
        let mut method = node("method", "Ordinary");
        assert!(!excluded(&method));
        method.declaration.as_mut().unwrap().is_object_member = true;
        assert!(excluded(&method));
        assert!(!excluded(&node("accessor", "PropertyGet")));
        assert!(!excluded(&node("toplevel", "TopLevel")));
    }
}
