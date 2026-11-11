//! Roslyn-owned MediatR facts are separate from native compiler call edges.
use crate::model::{Edge, EntryPoints, GraphData, RelationshipEvidence, SourceSite};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::Path,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DispatchFact {
    from: String,
    to: String,
    dispatch: String,
    invocation: Invocation,
    payload: String,
    handler: String,
    awaited: bool,
    conditional: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Invocation {
    file: String,
    line: u32,
    column: u32,
}

#[derive(Default)]
struct InvocationTargets {
    sites: BTreeSet<SourceSite>,
    evidence: BTreeSet<RelationshipEvidence>,
}

pub(super) struct Facts {
    pub edges: Vec<DispatchFact>,
    pub entries: Vec<String>,
    pub diagnostics: Vec<String>,
}

pub(super) fn append(
    root: &Path,
    graph: &mut GraphData,
    indices: &HashMap<String, u32>,
    facts: Facts,
) -> Result<()> {
    let mut targets = BTreeMap::<(u32, u32, String), InvocationTargets>::new();
    for fact in facts.edges {
        ensure!(
            matches!(fact.dispatch.as_str(), "send" | "publish"),
            "Unsupported MediatR dispatch"
        );
        let from = *indices
            .get(&fact.from)
            .context("MediatR caller is missing")?;
        let to = *indices
            .get(&fact.to)
            .context("MediatR handler is missing")?;
        let evidence = evidence(root, &graph.nodes[to as usize].id, &fact)?;
        let target = targets.entry((from, to, fact.dispatch)).or_default();
        target.sites.insert(evidence.invocation.clone());
        target.evidence.insert(evidence);
    }
    for ((from, to, _), target) in targets {
        graph.edges.push(Edge {
            from,
            to,
            kind: "call".into(),
            certainty: "possible".into(),
            provenance: "codecallnavigator.mediatr.v1".into(),
            count: u32::try_from(target.sites.len())?,
            evidence: target.evidence.into_iter().collect(),
        });
    }
    let framework = &mut graph
        .entry_points
        .get_or_insert_with(EntryPoints::default)
        .framework_entries;
    for entry in facts.entries {
        let index = indices.get(&entry).context("MediatR entry is missing")?;
        framework.push(graph.nodes[*index as usize].id.clone());
    }
    framework.sort();
    framework.dedup();
    graph.diagnostics.extend(facts.diagnostics);
    Ok(())
}

fn evidence(root: &Path, target: &str, fact: &DispatchFact) -> Result<RelationshipEvidence> {
    ensure!(
        !fact.payload.is_empty() && !fact.handler.is_empty(),
        "Incomplete MediatR type facts"
    );
    ensure!(
        !fact.invocation.file.is_empty() && fact.invocation.line > 0 && fact.invocation.column > 0,
        "Incomplete MediatR invocation site"
    );
    let (file, line) = super::location(
        &format!("{}:{}", fact.invocation.file, fact.invocation.line),
        root,
    );
    let operation = if fact.dispatch == "send" {
        "Send"
    } else {
        "Publish"
    };
    let mut qualifiers = BTreeMap::from([
        ("mediatr.operation".into(), operation.into()),
        ("mediatr.completion".into(), "task".into()),
        ("mediatr.payload".into(), fact.payload.clone()),
        ("mediatr.handler".into(), fact.handler.clone()),
        ("mediatr.direct_await".into(), fact.awaited.to_string()),
    ]);
    if fact.conditional {
        // An enclosing conditional syntax construct, not a proven control-flow guard.
        qualifiers.insert("mediatr.conditional_context".into(), "true".into());
    }
    Ok(RelationshipEvidence {
        rule: format!("codecallnavigator.mediatr:{}", fact.dispatch),
        version: 1,
        invocation: SourceSite {
            file,
            line,
            column: Some(fact.invocation.column),
            span: None,
        },
        registration: None,
        supporting_symbols: vec![target.to_owned()],
        channel: None,
        qualifiers,
        semantics: "possible-invocation".into(),
    })
}
