//! Pure operations over one immutable graph generation.
use super::{
    Category, Error, Query, Result, Selection, SourceOrigin, select, select_render, selection::node,
};
use crate::{
    flow,
    graph::{Direction, GraphIndex},
    search::{NameSearchIndex, NameSearchQuery, SearchCategory, SearchTarget},
};
use serde_json::{Value, json};

pub fn execute(graph: &GraphIndex, query: &Query) -> Result<Value> {
    match query {
        Query::Status => Ok(
            json!({"nodes":graph.data.nodes.len(),"edges":graph.data.edges.len(),"diagnostics":graph.data.diagnostics}),
        ),
        Query::Snapshot => encode(&*graph.data),
        Query::Symbol { symbol } => encode(&graph.data.nodes[node(graph, symbol)? as usize]),
        Query::Search {
            text,
            category,
            offset,
            limit,
        } => search(
            graph,
            &NameSearchQuery {
                text,
                category: match category {
                    Category::All => SearchCategory::All,
                    Category::Classes => SearchCategory::Classes,
                    Category::Methods => SearchCategory::Methods,
                },
                offset: *offset,
                limit: *limit,
            },
        ),
        Query::Selection {
            selection,
            filter,
            limits,
        } => section(graph, select(graph, selection, filter, *limits)?),
        Query::RenderSelection { selection, filter } => {
            section(graph, select_render(graph, selection, filter)?)
        }
        Query::Neighbors {
            symbol,
            filter,
            limits,
        } => {
            let selection = super::SelectionKind::Neighborhood {
                symbols: vec![symbol.clone()],
                direction: Direction::Both,
                depth: 1,
            };
            section(graph, select(graph, &selection, filter, *limits)?)
        }
        Query::Source {
            symbol,
            origin: SourceOrigin::Analyzed,
        } => source(graph, symbol),
        Query::Flow { symbol } => {
            let function =
                flow::lookup(&graph.data, symbol).map_err(|e| Error::new("flow_unavailable", e))?;
            let chart = flow::build(function)
                .map_err(|e| Error::new("flow_unavailable", format!("{e:#}")))?;
            Ok(
                json!({"function":function,"chart":chart_value(&chart),"source":source(graph,symbol)?}),
            )
        }
        Query::Source {
            origin: SourceOrigin::Live,
            ..
        }
        | Query::Explorer { .. } => Err(Error::new(
            "invalid_request",
            "Live filesystem queries require engine handling",
        )),
    }
}

pub(super) fn encode(value: &impl serde::Serialize) -> Result<Value> {
    serde_json::to_value(value).map_err(|e| Error::new("serialization_failed", e.to_string()))
}

fn search(graph: &GraphIndex, request: &NameSearchQuery<'_>) -> Result<Value> {
    if !(1..=200).contains(&request.limit) {
        return Err(Error::new(
            "invalid_request",
            "Search limit must be 1..=200",
        ));
    }
    let page = NameSearchIndex::new(&graph.data).search(request);
    let hits=page.hits.iter().map(|hit| {
        let target=match &hit.target {
            SearchTarget::Method { identity,.. }=>json!({"kind":"method","symbol":identity}),
            SearchTarget::Class { identity,members,file_container }=>json!({"kind":"class","build":identity.build,"project":identity.project,"container":identity.container,"file_container":file_container,"member_count":members.len()}),
        };
        json!({"name":hit.name,"detail":hit.detail,"target":target})
    }).collect::<Vec<_>>();
    Ok(json!({"hits":hits,"more":page.more,"offset":request.offset,"limit":request.limit}))
}

fn section(graph: &GraphIndex, selection: Selection) -> Result<Value> {
    let nodes = selection
        .nodes
        .iter()
        .map(|id| &graph.data.nodes[*id as usize])
        .collect::<Vec<_>>();
    let edges=selection.edges.iter().map(|id| {
        let edge=&graph.data.edges[*id as usize];
        json!({"from":graph.data.nodes[edge.from as usize].id,"to":graph.data.nodes[edge.to as usize].id,"kind":edge.kind,"certainty":edge.certainty,"provenance":edge.provenance,"count":edge.count,"evidence":edge.evidence,"classification":edge.classification()})
    }).collect::<Vec<_>>();
    Ok(
        json!({"nodes":nodes,"edges":edges,"roots":selection.roots.iter().map(|id|&graph.data.nodes[*id as usize].id).collect::<Vec<_>>(),"diagnostics":graph.data.diagnostics,"omitted_nodes":selection.omitted_nodes,"omitted_edges":selection.omitted_edges,"truncated":selection.omitted_nodes>0 || selection.omitted_edges>0}),
    )
}

fn source(graph: &GraphIndex, symbol: &str) -> Result<Value> {
    let node = &graph.data.nodes[node(graph, symbol)? as usize];
    let file = graph
        .data
        .architecture
        .as_ref()
        .and_then(|facts| {
            facts.files.iter().find(|file| {
                file.file == node.file && file.build == node.build && file.project == node.project
            })
        })
        .ok_or_else(|| {
            Error::new(
                "source_unavailable",
                "Refresh to collect analyzed source for this declaration",
            )
        })?;
    Ok(
        json!({"origin":"analyzed","file":node.file,"content":file.content,"line":node.line,"symbol":node.id}),
    )
}

fn chart_value(chart: &flow::FlowChart) -> Value {
    json!({"symbol":chart.symbol,"file":chart.file,"span":chart.span,"entry":chart.entry,"partial":chart.partial,"diagnostics":chart.diagnostics,
        "steps":chart.steps.iter().map(|s|json!({"id":s.id,"kind":s.kind,"label":s.label,"span":s.span,"calls":s.calls})).collect::<Vec<_>>(),
        "links":chart.links.iter().map(|l|json!({"from":l.from,"to":l.to,"label":l.label,"kind":l.kind})).collect::<Vec<_>>(),
        "regions":chart.regions.iter().map(|r|json!({"label":r.label,"deferred":r.deferred,"entry":r.entry,"steps":r.steps})).collect::<Vec<_>>()})
}
