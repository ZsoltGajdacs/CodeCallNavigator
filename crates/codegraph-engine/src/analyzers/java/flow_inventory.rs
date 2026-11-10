//! Bind detached javac source IR through existing bytecode identities, never by display names.
use super::{
    JavaInput,
    lambda_binding::{self, DeclarationIndex},
    normalize::identity_prefix,
    source_inventory::Inventories,
};
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
    pub(super) lambda_owner: String,
    #[serde(default)]
    pub(super) export_identity: String,
    #[serde(default)]
    lambda_start_line: u32,
    #[serde(default)]
    lambda_end_line: u32,
}

impl SourceFlow {
    pub fn is_lambda(&self) -> bool {
        !self.lambda_owner.is_empty()
    }

    pub fn contains_line(&self, line: u32) -> bool {
        line > 0 && line >= self.lambda_start_line && line <= self.lambda_end_line
    }
}

pub(super) struct BindingContext<'a> {
    pub build: &'a BuildTarget,
    pub inputs: &'a [JavaInput],
    pub owner: Option<&'a str>,
}

pub(super) struct BindingResult {
    pub inventory: Option<FlowInventory>,
    pub ambiguous_callbacks: usize,
    pub shared_callbacks: usize,
    pub export_identities: HashMap<String, String>,
    pub unstable_identities: Vec<String>,
    pub correlation_diagnostics: Vec<String>,
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
    let mut shared_callbacks = 0;
    let mut correlation_diagnostics = Vec::new();
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
                shared_callbacks: 0,
                export_identities: HashMap::new(),
                unstable_identities: Vec::new(),
                correlation_diagnostics: Vec::new(),
            };
        }
        let declarations = DeclarationIndex::new(graph, &prefix);
        let lambdas = lambda_binding::resolve(inventory, &prefix, &declarations);
        ambiguous_callbacks += lambdas.unresolved;
        shared_callbacks += lambdas.shared;
        export_identities.extend(lambdas.export_identities);
        correlation_diagnostics.extend(inventory.lambda_correlations.diagnostics());
        for (index, raw) in inventory.flows.iter().enumerate() {
            let node = if raw.is_lambda() {
                lambdas.flows.get(&index).copied()
            } else {
                declarations.declaration(&format!("{prefix}{}", raw.function.symbol))
            };
            let Some(node) = node else {
                continue;
            };
            functions.push(bind_function(&raw.function, node, &targets));
        }
    }
    functions.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    correlation_diagnostics.sort();
    let unstable_identities = unstable_identities(graph, &export_identities);
    BindingResult {
        export_identities,
        unstable_identities,
        inventory: Some(FlowInventory {
            version: INVENTORY_VERSION,
            functions,
        }),
        ambiguous_callbacks,
        shared_callbacks,
        correlation_diagnostics,
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
