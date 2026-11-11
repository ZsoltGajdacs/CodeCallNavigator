//! Immutable name lookup shared by desktop search and deterministic fixtures.
use crate::model::{GraphData, Node};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClassIdentity {
    pub build: String,
    pub project: String,
    pub container: String,
}

impl ClassIdentity {
    pub fn from_node(node: &Node) -> Self {
        Self {
            build: node.build.clone(),
            project: node.project.clone(),
            container: node.container.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchCategory {
    #[default]
    All,
    Classes,
    Methods,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchTarget {
    Class {
        identity: ClassIdentity,
        members: Arc<[u32]>,
        file_container: bool,
    },
    Method {
        identity: String,
        member: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub name: String,
    pub detail: String,
    pub target: SearchTarget,
}

impl SearchHit {
    pub fn kind_label(&self) -> &'static str {
        match &self.target {
            SearchTarget::Class {
                file_container: true,
                ..
            } => "File",
            SearchTarget::Class { .. } => "Class",
            SearchTarget::Method { .. } => "Method",
        }
    }
}

pub struct NameSearchQuery<'a> {
    pub text: &'a str,
    pub category: SearchCategory,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Default)]
pub struct SearchPage {
    pub hits: Vec<SearchHit>,
    pub more: bool,
}

struct Entry {
    hit: SearchHit,
    category: SearchCategory,
    names: Vec<String>,
    order: String,
}

/// Construct once per graph generation, outside rendering and key handling.
#[derive(Default)]
pub struct NameSearchIndex {
    entries: Vec<Entry>,
}

impl NameSearchIndex {
    pub fn new(graph: &GraphData) -> Self {
        let mut entries = Vec::with_capacity(graph.nodes.len());
        let mut classes: BTreeMap<ClassIdentity, Vec<u32>> = BTreeMap::new();
        for (member, node) in graph.nodes.iter().enumerate() {
            if !node.container.is_empty() {
                classes
                    .entry(ClassIdentity::from_node(node))
                    .or_default()
                    .push(member as u32);
            }
            entries.push(method_entry(node, member as u32));
        }
        for (identity, members) in classes {
            let node = &graph.nodes[members[0] as usize];
            entries.push(class_entry(identity, members, node));
        }
        Self { entries }
    }

    pub fn search(&self, request: &NameSearchQuery<'_>) -> SearchPage {
        let NameSearchQuery {
            text,
            category,
            offset,
            limit,
        } = *request;
        let query = text.trim().to_lowercase();
        if query.is_empty() || limit == 0 {
            return SearchPage::default();
        }
        let mut matches: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| category == SearchCategory::All || entry.category == category)
            .filter_map(|entry| rank(&entry.names, &query).map(|rank| (rank, entry)))
            .collect();
        matches.sort_unstable_by(|(a_rank, a), (b_rank, b)| {
            (a_rank, &a.names[0], &a.order).cmp(&(b_rank, &b.names[0], &b.order))
        });
        let more = matches.len().saturating_sub(offset) > limit;
        let hits = matches
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|(_, entry)| entry.hit.clone())
            .collect();
        SearchPage { hits, more }
    }
}

fn rank(names: &[String], query: &str) -> Option<u8> {
    names
        .iter()
        .filter_map(|name| {
            if name == query {
                Some(0)
            } else if name.starts_with(query) {
                Some(1)
            } else if name.contains(query) {
                Some(2)
            } else {
                None
            }
        })
        .min()
}

fn method_entry(node: &Node, member: u32) -> Entry {
    let mut names = vec![node.label.to_lowercase()];
    // Signatures carry compiler names for friendly lambdas, but parameter types and
    // source paths are context, not searchable names.
    let declared = node.signature.split('(').next().unwrap_or_default();
    if !declared.is_empty() {
        names.push(declared.to_lowercase());
        if let Some(name) = declared.rsplit([':', '.']).next() {
            names.push(name.to_lowercase());
        }
    }
    if !node.container.is_empty() {
        names.push(format!("{}.{}", node.container, node.label).to_lowercase());
    }
    Entry {
        category: SearchCategory::Methods,
        names,
        order: format!("method\0{}", node.id),
        hit: SearchHit {
            name: node.label.clone(),
            detail: format!("{} · {} · {}", node.signature, node.project, node.build),
            target: SearchTarget::Method {
                identity: node.id.clone(),
                member,
            },
        },
    }
}

fn class_entry(identity: ClassIdentity, members: Vec<u32>, node: &Node) -> Entry {
    let name = class_name(node).to_owned();
    Entry {
        category: SearchCategory::Classes,
        names: vec![name.to_lowercase(), node.container.to_lowercase()],
        order: format!(
            "class\0{}\0{}\0{}",
            identity.build, identity.project, identity.container
        ),
        hit: SearchHit {
            name,
            detail: format!("{} · {} · {}", node.container, node.project, node.build),
            target: SearchTarget::Class {
                identity,
                members: members.into(),
                file_container: node.language.eq_ignore_ascii_case("typescript"),
            },
        },
    }
}

/// TypeScript currently groups callables by source file rather than declaration.
pub fn class_name(node: &Node) -> &str {
    if node.language.eq_ignore_ascii_case("typescript") {
        &node.container
    } else {
        simple_class_name(&node.container)
    }
}

pub fn simple_class_name(container: &str) -> &str {
    let mut depth = 0_u32;
    let mut start = 0;
    for (index, character) in container.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            '.' | '+' | '$' if depth == 0 => start = index + 1,
            _ => {}
        }
    }
    &container[start..]
}

#[cfg(test)]
#[path = "search/tests.rs"]
mod tests;
