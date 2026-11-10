//! Bind compiler-proven Spring pairs within one source inventory; native call edges stay intact.
use super::{normalize::identity_prefix, source_inventory::Inventories};
use crate::{
    flow::{CallTarget, visit_calls_mut},
    model::{Edge, GraphData, RelationshipEvidence, SourceSite},
};
use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;

const PROVENANCE: &str = "codegraph.spring.event.v1";

#[derive(Deserialize)]
pub(super) struct SourcePair {
    caller: String,
    handler: String,
    invocation: SourceSite,
    registration: SourceSite,
    event_type: String,
    semantics: String,
    qualifiers: BTreeMap<String, String>,
}

pub(super) fn attach(
    context: super::flow_inventory::BindingContext<'_>,
    inventories: &Inventories,
    graph: &mut GraphData,
) -> Result<()> {
    let super::flow_inventory::BindingContext {
        build,
        inputs,
        owner,
    } = context;
    for (&input, inventory) in inventories {
        let Some(source) = inputs[input - 1].source_set.as_ref() else {
            continue;
        };
        let prefix = identity_prefix(build, source);
        if owner.is_some_and(|owner| owner != prefix) {
            continue;
        }
        graph
            .diagnostics
            .extend(inventory.framework_diagnostics.iter().cloned());
        let listeners = inventory
            .framework_listeners
            .iter()
            .map(|key| format!("{prefix}{key}"))
            .collect::<std::collections::HashSet<_>>();
        let entries = graph.entry_points.get_or_insert_with(Default::default);
        entries.framework_entries.extend(
            graph
                .nodes
                .iter()
                .filter(|node| listeners.contains(&node.id))
                .map(|node| node.id.clone()),
        );
        entries.framework_entries.sort();
        entries.framework_entries.dedup();
        for pair in &inventory.framework {
            attach_pair(pair, &prefix, graph);
        }
    }
    if graph.edges.iter().any(|edge| edge.provenance == PROVENANCE) {
        graph.diagnostics.push("Spring event targets are static possibilities within one compiler source context. DI registration, event conditions, transaction state and async configuration are not proven. Cross-context and lambda publishers remain unresolved.".into());
    }
    Ok(())
}

fn attach_pair(pair: &SourcePair, prefix: &str, graph: &mut GraphData) {
    let caller = format!("{prefix}{}", pair.caller);
    let handler = format!("{prefix}{}", pair.handler);
    let Some(from) = graph.nodes.iter().position(|node| node.id == caller) else {
        return;
    };
    let Some(to) = graph.nodes.iter().position(|node| node.id == handler) else {
        return;
    };
    if !attach_flow(graph, &caller, &handler, pair) {
        graph.diagnostics.push(format!("Spring event inference deferred because its exact source flow site is unavailable: {}:{}", pair.invocation.file, pair.invocation.line));
        return;
    }
    let evidence = RelationshipEvidence {
        rule: "codegraph.spring:event".into(),
        version: 1,
        invocation: pair.invocation.clone(),
        registration: Some(pair.registration.clone()),
        supporting_symbols: vec![caller, handler],
        channel: Some(pair.event_type.clone()),
        semantics: pair.semantics.clone(),
        qualifiers: pair.qualifiers.clone(),
    };
    if let Some(edge) = graph
        .edges
        .iter_mut()
        .find(|edge| same_pair(edge, from, to))
    {
        if !edge.evidence.contains(&evidence) {
            edge.evidence.push(evidence);
            edge.evidence.sort();
            edge.count = edge.evidence.len() as u32;
        }
    } else {
        graph.edges.push(Edge {
            from: from as u32,
            to: to as u32,
            kind: "spring-event".into(),
            certainty: "possible".into(),
            provenance: PROVENANCE.into(),
            count: 1,
            evidence: vec![evidence],
        });
    }
}

fn same_pair(edge: &Edge, from: usize, to: usize) -> bool {
    (edge.from, edge.to, edge.provenance.as_str()) == (from as u32, to as u32, PROVENANCE)
}

fn attach_flow(graph: &mut GraphData, caller: &str, handler: &str, pair: &SourcePair) -> bool {
    let Some(inventory) = graph.flow_inventory.as_mut() else {
        return false;
    };
    let Some(function) = inventory
        .functions
        .iter_mut()
        .find(|function| function.symbol == caller && function.file == pair.invocation.file)
    else {
        return false;
    };
    let mut matched = false;
    visit_calls_mut(function, &mut |call| {
        if Some(call.span) != pair.invocation.span {
            return;
        }
        if !call
            .targets
            .iter()
            .any(|target| target.symbol == handler && target.provenance == PROVENANCE)
        {
            call.targets.push(CallTarget {
                kind: "call".into(),
                symbol: handler.into(),
                certainty: "possible".into(),
                provenance: PROVENANCE.into(),
            });
            call.targets
                .sort_by(|a, b| (&a.symbol, &a.provenance).cmp(&(&b.symbol, &b.provenance)));
        }
        call.diagnostic = Some("Spring event listener is a possible framework target; conditions, DI, transaction and async boundaries may defer or prevent execution".into());
        matched = true;
    });
    matched
}
