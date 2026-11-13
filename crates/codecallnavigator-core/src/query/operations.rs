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
        Query::Status => Ok(status(graph)),
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
        } => section(graph, &select(graph, selection, filter, *limits)?),
        Query::RenderSelection {
            selection,
            filter,
            layer: None,
        } => render_section(graph, select_render(graph, selection, filter)?, filter),
        Query::Inspect { symbol, filter } => inspect(graph, symbol, filter),
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
            section(graph, &select(graph, &selection, filter, *limits)?)
        }
        Query::Source {
            symbol,
            origin: SourceOrigin::Analyzed,
        } => source(graph, symbol),
        Query::Flow { symbol } => function_flow(graph, symbol),
        Query::Source {
            origin: SourceOrigin::Live,
            ..
        }
        | Query::Explorer { .. }
        | Query::LiveSource { .. }
        | Query::Architecture { .. }
        | Query::Freshness { .. }
        | Query::Compare { .. }
        | Query::RenderSelection { layer: Some(_), .. } => Err(Error::new(
            "invalid_request",
            "Live filesystem queries require engine handling",
        )),
    }
}

fn function_flow(graph: &GraphIndex, symbol: &str) -> Result<Value> {
    let function =
        flow::lookup(&graph.data, symbol).map_err(|e| Error::new("flow_unavailable", e))?;
    let content = analyzed_source(
        graph,
        FileLocation {
            file: &function.file,
            build: &function.build,
            project: &function.project,
        },
    )?;
    validate_flow_source(function, content)?;
    let chart =
        flow::build(function).map_err(|e| Error::new("flow_unavailable", format!("{e:#}")))?;
    let line = graph.data.nodes[node(graph, symbol)? as usize].line;
    let source = json!({"origin":"analyzed","file":function.file,"symbol":symbol,"line":line,"content":content});
    Ok(json!({"function":function,"chart":chart,"source":source}))
}

fn validate_flow_source(function: &flow::FunctionFlowIR, content: &str) -> Result<()> {
    if content
        .get(function.span.start..function.span.end)
        .is_none()
    {
        return Err(Error::new(
            "flow_unavailable",
            "The flow source range does not match its cached snapshot",
        ));
    }
    if content.len() as u64 > super::MAX_SOURCE_BYTES {
        return Err(Error::new(
            "flow_unavailable",
            "The analyzed source exceeds the 16 MiB flow source limit",
        ));
    }
    Ok(())
}

fn status(graph: &GraphIndex) -> Value {
    let mut result = json!({"nodes":graph.data.nodes.len(),"edges":graph.data.edges.len()});
    super::diagnostics::append(&mut result, &graph.data.diagnostics);
    result
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

fn section(graph: &GraphIndex, selection: &Selection) -> Result<Value> {
    let nodes = selection
        .nodes
        .iter()
        .map(|id| &graph.data.nodes[*id as usize])
        .collect::<Vec<_>>();
    let edges = selection
        .edges
        .iter()
        .map(|id| edge_value(graph, *id))
        .collect::<Vec<_>>();
    let mut result = json!({"nodes":nodes,"edges":edges,"roots":selection.roots.iter().map(|id|&graph.data.nodes[*id as usize].id).collect::<Vec<_>>(),"omitted_nodes":selection.omitted_nodes,"omitted_edges":selection.omitted_edges,"truncated":selection.omitted_nodes>0 || selection.omitted_edges>0});
    super::diagnostics::append(&mut result, &graph.data.diagnostics);
    Ok(result)
}

struct FileLocation<'a> {
    file: &'a str,
    build: &'a str,
    project: &'a str,
}

fn analyzed_source<'a>(graph: &'a GraphIndex, location: FileLocation<'_>) -> Result<&'a str> {
    graph
        .data
        .architecture
        .as_ref()
        .and_then(|facts| {
            facts.files.iter().find(|file| {
                file.file == location.file
                    && file.build == location.build
                    && file.project == location.project
            })
        })
        .map(|file| file.content.as_str())
        .ok_or_else(|| {
            Error::new(
                "source_unavailable",
                "Refresh to collect analyzed source for this declaration",
            )
        })
}

fn source(graph: &GraphIndex, symbol: &str) -> Result<Value> {
    let node = &graph.data.nodes[node(graph, symbol)? as usize];
    let content = analyzed_source(
        graph,
        FileLocation {
            file: &node.file,
            build: &node.build,
            project: &node.project,
        },
    )?;
    Ok(
        json!({"origin":"analyzed","file":node.file,"content":content,"line":node.line,"symbol":node.id}),
    )
}

fn edge_value(graph: &GraphIndex, id: u32) -> Value {
    let edge = &graph.data.edges[id as usize];
    json!({"from":graph.data.nodes[edge.from as usize].id,"to":graph.data.nodes[edge.to as usize].id,"kind":edge.kind,"certainty":edge.certainty,"provenance":edge.provenance,"count":edge.count,"evidence":edge.evidence,"classification":edge.classification()})
}

/// Snapshot-local offsets are valid only with the generation-bound render snapshot.
pub fn render_section(
    graph: &GraphIndex,
    selection: Selection,
    filter: &crate::graph::Filter,
) -> Result<Value> {
    let mut result = section(graph, &selection)?;
    result["snapshot_nodes"] = json!(selection.nodes);
    result["snapshot_edges"] = json!(selection.edges);
    let summary = graph.filter_summary(filter);
    let cyclic = graph
        .data
        .nodes
        .iter()
        .zip(&summary.cyclic)
        .filter(|(_, cyclic)| **cyclic)
        .map(|(node, _)| &node.id)
        .collect::<Vec<_>>();
    result["summary"] = json!({"cyclic_symbols":cyclic,"hidden_nodes":summary.hidden_nodes,"hidden_edges":summary.hidden_edges});
    Ok(result)
}

fn inspect(graph: &GraphIndex, symbol: &str, filter: &crate::graph::Filter) -> Result<Value> {
    let root = node(graph, symbol)?;
    let incoming = graph
        .neighbors(root, Direction::Incoming, filter)
        .into_iter()
        .map(|id| graph.data.nodes[id as usize].id.clone())
        .collect::<Vec<_>>();
    let outgoing = graph
        .neighbors(root, Direction::Outgoing, filter)
        .into_iter()
        .map(|id| graph.data.nodes[id as usize].id.clone())
        .collect::<Vec<_>>();
    let edges = graph.incident_edges(root, Direction::Both, filter);
    let relationships = edges
        .iter()
        .map(|id| edge_value(graph, *id))
        .collect::<Vec<_>>();
    let mut result = json!({"incoming":incoming,"outgoing":outgoing,"relationships":relationships,"snapshot_edges":edges});
    super::diagnostics::append(&mut result, &graph.data.diagnostics);
    Ok(result)
}
