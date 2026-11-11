//! Resolve validated source groups without changing native graph identities or edges.
use super::{flow_inventory::SourceFlow, source_inventory::SourceInventory};
use crate::model::{GraphData, Node};
use std::collections::{HashMap, HashSet};

pub(super) struct DeclarationIndex<'a> {
    by_id: HashMap<&'a str, &'a Node>,
    lambdas: HashMap<(&'a str, &'a str), Vec<&'a Node>>,
}

impl<'a> DeclarationIndex<'a> {
    pub fn new(graph: &'a GraphData, prefix: &str) -> Self {
        let mut by_id = HashMap::new();
        let mut lambdas: HashMap<_, Vec<_>> = HashMap::new();
        for node in graph
            .nodes
            .iter()
            .filter(|node| node.id.starts_with(prefix))
        {
            by_id.insert(node.id.as_str(), node);
            if let Some(owner) = lambda_owner(node) {
                lambdas
                    .entry((owner, node.file.as_str()))
                    .or_default()
                    .push(node);
            }
        }
        Self { by_id, lambdas }
    }

    pub fn declaration(&self, identity: &str) -> Option<&'a Node> {
        self.by_id.get(identity).copied()
    }

    fn legacy(&self, raw: &SourceFlow, peers: &[&SourceFlow]) -> Option<&'a Node> {
        let nodes = self
            .lambdas
            .get(&(raw.lambda_owner.as_str(), raw.function.file.as_str()))?;
        let mut matches = nodes.iter().filter(|node| raw.contains_line(node.line));
        let candidate = *matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        (peers
            .iter()
            .filter(|other| other.contains_line(candidate.line))
            .count()
            == 1)
            .then_some(candidate)
    }
}

#[derive(Default)]
pub(super) struct ResolvedLambdas<'a> {
    pub flows: HashMap<usize, &'a Node>,
    pub export_identities: HashMap<String, String>,
    pub unresolved: usize,
    pub shared: usize,
}

fn lambda_owner(node: &Node) -> Option<&str> {
    if node.visibility.is_some() {
        return None;
    }
    let (owner, method) = node.signature.split_once(':')?;
    method.starts_with("lambda$").then_some(owner)
}

pub(super) fn resolve<'a>(
    inventory: &SourceInventory,
    prefix: &str,
    nodes: &DeclarationIndex<'a>,
) -> ResolvedLambdas<'a> {
    let mut result = ResolvedLambdas::default();
    for binding in &inventory.lambda_correlations.bindings {
        let members: Vec<_> = binding
            .sources
            .iter()
            .map(|index| &inventory.flows[*index])
            .collect();
        let node = nodes
            .declaration(&format!("{prefix}{}", binding.implementation))
            .filter(|node| {
                node.file.is_empty()
                    || members
                        .iter()
                        .all(|source| source.function.file == node.file)
            });
        let Some(node) = node else {
            result.unresolved += members.len();
            continue;
        };
        record_identity(&members, node, prefix, &mut result.export_identities);
        if binding.sources.len() == 1 {
            result.flows.insert(binding.sources[0], node);
        } else {
            result.shared += binding.sources.len();
        }
    }
    resolve_legacy(inventory, prefix, nodes, &mut result);
    result
}

fn resolve_legacy<'a>(
    inventory: &SourceInventory,
    prefix: &str,
    nodes: &DeclarationIndex<'a>,
    result: &mut ResolvedLambdas<'a>,
) {
    let claimed: HashSet<_> = inventory
        .lambda_correlations
        .bindings
        .iter()
        .flat_map(|binding| binding.sources.iter().copied())
        .collect();
    let mut peers: HashMap<_, Vec<_>> = HashMap::new();
    for raw in inventory.flows.iter().filter(|flow| flow.is_lambda()) {
        peers
            .entry((raw.lambda_owner.as_str(), raw.function.file.as_str()))
            .or_default()
            .push(raw);
    }
    for (index, raw) in inventory
        .flows
        .iter()
        .enumerate()
        .filter(|(_, flow)| flow.is_lambda())
    {
        if claimed.contains(&index) {
            continue;
        }
        let matching = &peers[&(raw.lambda_owner.as_str(), raw.function.file.as_str())];
        let node = inventory
            .lambda_correlations
            .permits_line_fallback(&raw.lambda_owner)
            .then(|| nodes.legacy(raw, matching))
            .flatten();
        if let Some(node) = node {
            result.flows.insert(index, node);
            record_identity(&[raw], node, prefix, &mut result.export_identities);
        } else {
            result.unresolved += 1;
        }
    }
}

fn record_identity(
    members: &[&SourceFlow],
    node: &Node,
    prefix: &str,
    keys: &mut HashMap<String, String>,
) {
    if let Some(identity) = portable_identity(members) {
        keys.insert(node.id.clone(), format!("{prefix}{identity}"));
    }
}

fn portable_identity(members: &[&SourceFlow]) -> Option<String> {
    let mut keys: Vec<_> = members
        .iter()
        .map(|source| source.export_identity.as_str())
        .collect();
    if keys.is_empty() || keys.iter().any(|key| key.is_empty()) {
        return None;
    }
    if keys.len() == 1 {
        return Some(keys[0].to_owned());
    }
    keys.sort_unstable();
    Some(serde_json::json!({"source_lambda_group": keys}).to_string())
}
