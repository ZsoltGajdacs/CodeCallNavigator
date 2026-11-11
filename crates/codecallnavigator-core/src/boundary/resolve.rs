//! Infer from current compiler facts only; previous inferred edges are never evidence.
use super::PROVENANCE;
use super::{BoundaryUse, Configuration, RouteFact, matcher};
use crate::model::{Edge, GraphData, RelationshipEvidence};
use anyhow::Result;
use std::collections::{BTreeMap, HashMap};

pub fn resolve(graph: &mut GraphData, configuration: &Configuration) -> Result<()> {
    run(graph, configuration, None)
}
pub fn resolve_cached(
    graph: &mut GraphData,
    configuration: &Configuration,
    cache: &mut super::DecisionCache,
) -> Result<()> {
    run(graph, configuration, Some(cache))
}
fn run(
    graph: &mut GraphData,
    configuration: &Configuration,
    mut cache: Option<&mut super::DecisionCache>,
) -> Result<()> {
    configuration.validate()?;
    graph.validate()?;
    validate_scopes(graph, configuration)?;
    graph.edges.retain(|edge| edge.provenance != PROVENANCE);
    graph.diagnostics.retain(|text| {
        !text.starts_with("http boundary unresolved at ")
            && !text.starts_with("navigation boundary unresolved at ")
    });
    super::flow::clear(graph);
    let Some(inventory) = graph.boundaries.clone() else {
        return Ok(());
    };
    for usage in &inventory.uses {
        let path = match request_path(usage) {
            Ok(path) => path,
            Err(reason) => {
                unresolved(graph, usage, reason);
                continue;
            }
        };
        let candidates: Vec<_> = inventory
            .routes
            .iter()
            .filter(|route| {
                in_scope(route, usage, configuration)
                    && matcher::potential(route, usage.method.as_deref(), path)
            })
            .collect();
        let context = super::cache_key::Context {
            configuration,
            engine_versions: &graph.engine_versions,
        };
        let decision = decision(usage, &candidates, &context, cache.as_deref_mut())?;
        if let Some(route) = decision {
            attach(graph, usage, route)?;
        } else {
            unresolved(
                graph,
                usage,
                "route is missing, ambiguous or blocked by unsupported routing behavior",
            );
        }
    }
    graph.validate()
}
fn request_path(usage: &BoundaryUse) -> std::result::Result<&str, &'static str> {
    if usage.kind == "http" && usage.method.is_none() {
        return Err("unknown request method");
    }
    matcher::path(&usage.path).ok_or("unsupported or ambiguous URL")
}

fn in_scope(route: &RouteFact, usage: &BoundaryUse, configuration: &Configuration) -> bool {
    if route.kind != usage.kind {
        return false;
    }
    if usage.kind == "navigation" || own_next(route, usage) {
        return own_next(route, usage);
    }
    configuration
        .http
        .iter()
        .find(|scope| client_matches(scope, usage))
        .map_or_else(
            || false,
            |scope| {
                scope
                    .servers
                    .iter()
                    .any(|server| server_matches(server, route))
            },
        )
}
fn own_next(route: &RouteFact, usage: &BoundaryUse) -> bool {
    route.framework == "next"
        && route.build == usage.build
        && route.application == usage.application
}
fn client_matches(scope: &super::HttpScope, usage: &BoundaryUse) -> bool {
    scope.client == usage.build
        && scope
            .client_application
            .as_ref()
            .is_none_or(|app| app == &usage.application)
}
fn server_matches(server: &super::Server, route: &RouteFact) -> bool {
    server.target() == route.build
        && server
            .application()
            .is_none_or(|app| app == route.application)
}

fn winner<'a>(candidates: &[&'a RouteFact]) -> Option<&'a RouteFact> {
    let first = *candidates.first()?;
    if candidates.iter().any(|route| {
        (route.build.as_str(), route.application.as_str())
            != (first.build.as_str(), first.application.as_str())
    }) {
        return None;
    }
    let winners: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|candidate| is_winner(candidate, candidates))
        .collect();
    (winners.len() == 1).then(|| winners[0])
}
pub(super) fn is_winner(candidate: &RouteFact, candidates: &[&RouteFact]) -> bool {
    matcher::supported(candidate)
        && candidates.iter().all(|other| {
            same_owner(candidate, other)
                && (std::ptr::eq(*other, candidate) || matcher::dominates(candidate, other))
        })
}
fn same_owner(left: &RouteFact, right: &RouteFact) -> bool {
    left.build == right.build && left.application == right.application
}
fn decision<'a>(
    usage: &BoundaryUse,
    candidates: &[&'a RouteFact],
    context: &super::cache_key::Context<'_>,
    cache: Option<&mut super::DecisionCache>,
) -> Result<Option<&'a RouteFact>> {
    let Some(cache) = cache else {
        return Ok(winner(candidates));
    };
    let key = super::cache_key::key(usage, candidates, context, cache.implementation())?;
    if let Some(decision) = cache.lookup(&key, candidates) {
        return Ok(decision);
    }
    let selected = winner(candidates);
    cache.record(key, selected)?;
    Ok(selected)
}

fn unresolved(graph: &mut GraphData, usage: &BoundaryUse, reason: &str) {
    graph.diagnostics.push(format!(
        "{} boundary unresolved at {}:{}: {reason}",
        usage.kind, usage.site.file, usage.site.line
    ));
}
fn attach(graph: &mut GraphData, usage: &BoundaryUse, route: &RouteFact) -> Result<()> {
    let target = route.symbol.as_ref().expect("supported target");
    if usage.flow && !super::flow::attach(graph, usage, target) {
        unresolved(graph, usage, "exact flow call site unavailable");
        return Ok(());
    }
    let indices: HashMap<_, _> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index as u32))
        .collect();
    let evidence = RelationshipEvidence {
        rule: usage.rule.clone(),
        version: 1,
        invocation: usage.site.clone(),
        registration: Some(route.site.clone()),
        supporting_symbols: vec![usage.symbol.clone(), target.clone()],
        channel: Some(format!(
            "{} {}",
            usage.method.as_deref().unwrap_or("NAVIGATE"),
            usage.path
        )),
        qualifiers: BTreeMap::from([("boundary.framework".into(), route.framework.clone())]),
        semantics: usage.kind.clone(),
    };
    let from = indices[usage.symbol.as_str()];
    let to = indices[target.as_str()];
    if let Some(edge) = graph.edges.iter_mut().find(|edge| {
        edge.from == from
            && edge.to == to
            && edge.kind == usage.kind
            && edge.provenance == PROVENANCE
    }) {
        if !edge.evidence.contains(&evidence) {
            edge.evidence.push(evidence);
            edge.evidence.sort();
            edge.count = edge.evidence.len() as u32;
        }
    } else {
        graph.edges.push(Edge {
            from,
            to,
            kind: usage.kind.clone(),
            certainty: "possible".into(),
            provenance: PROVENANCE.into(),
            count: 1,
            evidence: vec![evidence],
        });
    }
    Ok(())
}

fn validate_scopes(graph: &GraphData, configuration: &Configuration) -> Result<()> {
    let Some(inventory) = &graph.boundaries else {
        return Ok(());
    };
    for scope in &configuration.http {
        validate_owner(
            inventory
                .uses
                .iter()
                .filter(|usage| usage.build == scope.client)
                .map(|usage| usage.application.as_str()),
            scope.client_application.as_deref(),
        )?;
        for server in &scope.servers {
            validate_owner(
                inventory
                    .routes
                    .iter()
                    .filter(|route| route.build == server.target())
                    .map(|route| route.application.as_str()),
                server.application(),
            )?;
        }
    }
    Ok(())
}
fn validate_owner<'a>(
    applications: impl Iterator<Item = &'a str>,
    selected: Option<&str>,
) -> Result<()> {
    use anyhow::ensure;
    let owners: std::collections::HashSet<_> = applications.collect();
    if let Some(selected) = selected {
        ensure!(
            !selected.trim().is_empty() && (owners.is_empty() || owners.contains(selected)),
            "Unknown boundary application: {selected}"
        );
    } else {
        ensure!(
            owners.len() <= 1,
            "Boundary build has multiple applications; select application explicitly"
        );
    }
    Ok(())
}
