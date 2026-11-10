//! Bind detached javac source IR through existing bytecode identities, never by display names.
use super::{JavaInput, normalize::identity_prefix, source_inventory::Inventories};
use crate::{
    discovery::BuildTarget,
    flow::{FlowInventory, FunctionFlowIR, INVENTORY_VERSION, visit_calls_mut},
    model::{GraphData, Node},
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Deserialize)]
pub(super) struct SourceFlow {
    #[serde(flatten)]
    pub function: FunctionFlowIR,
    #[serde(default)]
    lambda_owner: String,
    #[serde(default)]
    export_identity: String,
    #[serde(default)]
    lambda_start_line: u32,
    #[serde(default)]
    lambda_end_line: u32,
}

pub(super) struct BindingContext<'a> {
    pub build: &'a BuildTarget,
    pub inputs: &'a [JavaInput],
    pub owner: Option<&'a str>,
}

pub(super) struct BindingResult {
    pub inventory: Option<FlowInventory>,
    pub ambiguous_callbacks: usize,
    pub export_identities: HashMap<String, String>,
    pub unstable_identities: Vec<String>,
}

pub(super) fn bind(
    context: BindingContext<'_>,
    graph: &GraphData,
    inventories: &Inventories,
) -> BindingResult {
    let BindingContext {
        build,
        inputs,
        owner,
    } = context;
    let mut functions = Vec::new();
    let mut ambiguous_callbacks = 0;
    let mut export_identities = HashMap::new();
    let targets = target_index(graph);
    for (&input, inventory) in inventories {
        let Some(source) = inputs[input - 1].source_set.as_ref() else {
            continue;
        };
        let prefix = identity_prefix(build, source);
        if owner.is_some_and(|owner| owner != prefix) {
            continue;
        }
        if !inventory.has_flow {
            return BindingResult {
                inventory: None,
                ambiguous_callbacks: 0,
                export_identities: HashMap::new(),
                unstable_identities: Vec::new(),
            };
        }
        for raw in &inventory.flows {
            let Some(node) = declaration(raw, &prefix, graph, &inventory.flows) else {
                ambiguous_callbacks += usize::from(!raw.lambda_owner.is_empty());
                continue;
            };
            record_identity(raw, node, &prefix, &mut export_identities);
            functions.push(bind_function(&raw.function, node, &targets));
        }
    }
    functions.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    let unstable_identities = unstable_identities(graph, &export_identities);
    BindingResult {
        export_identities,
        unstable_identities,
        inventory: Some(FlowInventory {
            version: INVENTORY_VERSION,
            functions,
        }),
        ambiguous_callbacks,
    }
}

fn record_identity(
    raw: &SourceFlow,
    node: &Node,
    prefix: &str,
    keys: &mut HashMap<String, String>,
) {
    if !raw.lambda_owner.is_empty() && !raw.export_identity.is_empty() {
        keys.insert(node.id.clone(), format!("{prefix}{}", raw.export_identity));
    }
}
fn unstable_identities(graph: &GraphData, keys: &HashMap<String, String>) -> Vec<String> {
    graph.nodes.iter().filter(|node|node.visibility.is_none() && node.signature.contains(":lambda$") && !keys.contains_key(&node.id)).map(|node|format!("Java anonymous portable identity remains compiler-native because source attribution is ambiguous, unlocated or has an anonymous/local type owner: {}",node.id)).collect()
}

fn bind_function(
    raw: &FunctionFlowIR,
    node: &Node,
    targets: &HashMap<String, &str>,
) -> FunctionFlowIR {
    let mut function = raw.clone();
    function.symbol = node.id.clone();
    function.build = node.build.clone();
    function.project = node.project.clone();
    visit_calls_mut(&mut function, &mut |call| {
        let mut missing = false;
        call.targets.retain_mut(|target| {
            if let Some(symbol) = targets.get(&target.symbol) {
                target.symbol = (*symbol).to_owned();
                true
            } else {
                missing = true;
                false
            }
        });
        if missing {
            call.diagnostic = Some(
                "Compiler target is external, excluded or ambiguous in this compile context".into(),
            );
        }
    });
    function
}

fn target_index(graph: &GraphData) -> HashMap<String, &str> {
    let mut targets = HashMap::new();
    for node in &graph.nodes {
        if let Some((signature, returns)) = node.signature.rsplit_once(" -> ") {
            targets.insert(format!("{signature}|{returns}"), node.id.as_str());
        }
    }
    targets
}

fn declaration<'a>(
    raw: &SourceFlow,
    prefix: &str,
    graph: &'a GraphData,
    flows: &[SourceFlow],
) -> Option<&'a Node> {
    if raw.lambda_owner.is_empty() {
        let identity = format!("{prefix}{}", raw.function.symbol);
        return graph.nodes.iter().find(|node| node.id == identity);
    }
    let mut matches = graph
        .nodes
        .iter()
        .filter(|node| lambda_matches(raw, prefix, node));
    let candidate = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let overlapping = flows
        .iter()
        .filter(|other| lambda_matches(other, prefix, candidate))
        .count();
    (overlapping == 1).then_some(candidate)
}

fn lambda_matches(raw: &SourceFlow, prefix: &str, node: &Node) -> bool {
    !raw.lambda_owner.is_empty()
        && node.id.starts_with(prefix)
        && node.file == raw.function.file
        && node
            .signature
            .starts_with(&format!("{}:lambda$", raw.lambda_owner))
        && node.line >= raw.lambda_start_line
        && node.line <= raw.lambda_end_line
}
