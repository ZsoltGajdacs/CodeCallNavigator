//! Normalize Roslyn flow facts through the exact node identity and source ownership maps.
use super::{SourceOwners, source};
use crate::{
    flow::{self, CallTarget, FlowInventory, FunctionFlowIR},
    model::{GraphData, Node},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub(super) struct BindingContext<'a> {
    pub root: &'a Path,
    pub indices: &'a HashMap<String, u32>,
    pub snapshots: &'a source::Snapshots,
    pub owners: &'a SourceOwners,
}

pub(super) fn append(
    graph: &mut GraphData,
    inventory: FlowInventory,
    context: BindingContext<'_>,
) -> Result<()> {
    let mut functions = Vec::new();
    let mut known = HashSet::new();
    for mut function in inventory.functions {
        ensure!(
            known.insert(function.symbol.clone()),
            "Duplicate C# flow identity: {}",
            function.symbol
        );
        let index = *context.indices.get(&function.symbol).with_context(|| {
            format!(
                "C# flow callable is absent from compiler graph: {}",
                function.symbol
            )
        })?;
        let node = &graph.nodes[index as usize];
        let assembly = validate_attribution(context.root, &function, node)?.to_owned();
        let file = Path::new(&node.file);
        let owned = context
            .owners
            .get(file)
            .is_some_and(|owners| owners.contains_key(&assembly));
        bind_targets(&mut function, context.indices, &graph.nodes)?;
        if !owned {
            graph.diagnostics.push(format!(
                "C# flow unavailable for {}: source has no evaluated compile owner",
                node.id
            ));
            continue;
        }
        let content = match context.snapshots.binding(file, &assembly)? {
            source::Binding::Exact(content) => content,
            source::Binding::Unavailable(reason) => {
                graph.diagnostics.push(format!(
                    "C# flow unavailable for {}: {}",
                    node.id,
                    reason.reason()
                ));
                continue;
            }
        };
        function.symbol = node.id.clone();
        function.file = node.file.clone();
        function.build = node.build.clone();
        function.project = node.project.clone();
        flow::validate_source(&function, content)
            .with_context(|| format!("Invalid C# flow source ranges for {}", node.id))?;
        functions.push(function);
    }
    graph.flow_inventory = Some(FlowInventory {
        version: flow::INVENTORY_VERSION,
        functions,
    });
    Ok(())
}

fn validate_attribution<'a>(
    root: &Path,
    function: &'a FunctionFlowIR,
    node: &Node,
) -> Result<&'a str> {
    let assembly = function
        .symbol
        .strip_prefix('[')
        .and_then(|value| value.split_once("]::"))
        .map(|(assembly, _)| assembly)
        .context("C# flow identity has no assembly qualification")?;
    ensure!(
        !assembly.is_empty(),
        "C# flow identity has an empty assembly qualification"
    );
    ensure!(
        function.project == assembly,
        "C# flow assembly does not match its compiler identity"
    );
    ensure!(
        function.language == "csharp",
        "C# flow has an incompatible language"
    );
    ensure!(
        !function.file.is_empty() && source::path(root, &function.file) == Path::new(&node.file),
        "C# flow source file does not match its compiler declaration: {}",
        function.symbol
    );
    Ok(assembly)
}

fn bind_targets(
    function: &mut FunctionFlowIR,
    indices: &HashMap<String, u32>,
    nodes: &[Node],
) -> Result<()> {
    let mut failure = None;
    flow::visit_calls_mut(function, &mut |call| {
        if failure.is_some() {
            return;
        }
        for target in &mut call.targets {
            if let Err(error) = bind_target(target, indices, nodes) {
                failure = Some(error);
                return;
            }
        }
    });
    failure.map_or(Ok(()), Err)
}

fn bind_target(
    target: &mut CallTarget,
    indices: &HashMap<String, u32>,
    nodes: &[Node],
) -> Result<()> {
    ensure!(
        !target.certainty.is_empty(),
        "C# flow target has no certainty"
    );
    ensure!(
        !target.provenance.is_empty(),
        "C# flow target has no provenance"
    );
    ensure!(
        target.kind == "call",
        "C# flow target has an invalid native kind"
    );
    let index = indices.get(&target.symbol).with_context(|| {
        format!(
            "C# flow target is absent from compiler graph: {}",
            target.symbol
        )
    })?;
    target.symbol = nodes[*index as usize].id.clone();
    Ok(())
}

#[cfg(test)]
mod tests;
