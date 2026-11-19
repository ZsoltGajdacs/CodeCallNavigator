//! Potential impact from old-side Git ranges and one exact saved graph.
use super::{Artifact, flow::View};
use crate::model::{Edge, GraphData, Node, RelationshipKind, TargetResolution};
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};

const MAX_RECORDS: usize = 128;
const MAX_SYMBOLS: usize = 16;
const MAX_CALLERS: usize = 8;
const MAX_CHAIN_EDGES: usize = 12;
const MAX_VISITED_EDGES: usize = 2048;
const MAX_RECORD_BYTES: usize = 512 * 1024;

pub struct Change {
    /// Project-relative path on the baseline side.
    pub path: String,
    pub status: String,
    pub old_start: u32,
    pub old_count: u32,
    pub unresolved: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Preview {
    pub records: Vec<Record>,
    pub mapped_ranges: usize,
    pub unmapped_ranges: usize,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct Record {
    pub path: String,
    pub status: String,
    pub old_start: u32,
    pub old_count: u32,
    pub symbols: Vec<Symbol>,
    pub unresolved: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Symbol {
    pub id: String,
    pub label: String,
    pub line: u32,
    pub coverage: Coverage,
    pub entry_kind: Option<EntryKind>,
    pub callers: Vec<Caller>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    AnalyzedRange,
    DeclarationLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    ApplicationStart,
    Framework,
    Candidate,
}

#[derive(Debug, Serialize)]
pub struct Caller {
    pub id: String,
    pub label: String,
    /// Ordered from the caller through recorded invocations to the changed symbol.
    pub path: Vec<String>,
    pub possible: bool,
    pub entry_kind: Option<EntryKind>,
    /// One edge per adjacent pair in `path`, in the same order.
    pub edges: Vec<CallEvidence>,
}

#[derive(Debug, Serialize)]
pub struct CallEvidence {
    pub kind: String,
    pub certainty: String,
    pub provenance: String,
}

pub fn preview(artifact: &Artifact, changes: Vec<Change>) -> Result<Preview> {
    let mut result = Preview {
        records: Vec::new(),
        mapped_ranges: 0,
        unmapped_ranges: 0,
        truncated: changes.len() > MAX_RECORDS,
    };
    if changes.is_empty() {
        return Ok(result);
    }
    let (portable, ids) = artifact.validated_portable()?;
    let graph = portable.into_runtime(&ids)?;
    let mut sources = SourceRanges::new(&graph)?;
    let callers = CallerGraph::new(&graph);
    let mut remaining_bytes = MAX_RECORD_BYTES;
    for change in changes.into_iter().take(MAX_RECORDS) {
        let record = sources.map(change, &callers, &mut result.truncated)?;
        let bytes = serde_json::to_vec(&record)?.len();
        let Some(remaining) = remaining_bytes.checked_sub(bytes) else {
            result.truncated = true;
            break;
        };
        remaining_bytes = remaining;
        if record.symbols.is_empty() {
            result.unmapped_ranges += 1;
        } else {
            result.mapped_ranges += 1;
        }
        result.records.push(record);
    }
    Ok(result)
}

#[derive(Clone, Copy)]
struct Lines {
    first: u32,
    last: u32,
}

impl Lines {
    fn old_side(first: u32, count: u32) -> Result<Option<Self>> {
        if count == 0 {
            return Ok(None);
        }
        ensure!(first > 0, "Expected one-based Git source range");
        let last = first.checked_add(count - 1);
        ensure!(last.is_some(), "Git source range overflows");
        Ok(last.map(|last| Self { first, last }))
    }

    fn intersects(self, other: Self) -> bool {
        self.first <= other.last && other.first <= self.last
    }
}

type SourceOwner<'a> = (&'a str, &'a str, &'a str);
type SymbolRange = (usize, Lines, Coverage);

struct SourceRanges<'a> {
    graph: &'a GraphData,
    flow: View<'a>,
    files: BTreeMap<&'a str, Vec<usize>>,
    starts: BTreeMap<SourceOwner<'a>, Vec<usize>>,
    saved_ranges: BTreeMap<String, Vec<SymbolRange>>,
}

impl<'a> SourceRanges<'a> {
    fn new(graph: &'a GraphData) -> Result<Self> {
        let mut files = BTreeMap::<&str, Vec<usize>>::new();
        for (index, node) in graph.nodes.iter().enumerate() {
            if let Some(relative) = node.file.strip_prefix("@repo/") {
                files.entry(relative).or_default().push(index);
            }
        }
        for symbols in files.values_mut() {
            symbols.sort_by_key(|&index| (&graph.nodes[index].line, &graph.nodes[index].id));
        }
        Ok(Self {
            graph,
            flow: View::from_parts(graph, graph.flow_inventory.as_ref())?,
            files,
            starts: BTreeMap::new(),
            saved_ranges: BTreeMap::new(),
        })
    }

    fn map(
        &mut self,
        change: Change,
        callers: &CallerGraph<'_>,
        truncated: &mut bool,
    ) -> Result<Record> {
        let mut record = Record {
            path: change.path,
            status: change.status,
            old_start: change.old_start,
            old_count: change.old_count,
            symbols: Vec::new(),
            unresolved: change.unresolved,
        };
        if record.unresolved.is_some() {
            return Ok(record);
        }
        let Some(changed) = Lines::old_side(record.old_start, record.old_count)? else {
            record.unresolved = Some("no_baseline_lines".into());
            return Ok(record);
        };
        let mut covered = Vec::new();
        for &(index, range, coverage) in self.ranges(&record.path) {
            if !changed.intersects(range) {
                continue;
            }
            covered.push(range);
            if record.symbols.len() == MAX_SYMBOLS {
                *truncated = true;
                continue;
            }
            record
                .symbols
                .push(callers.symbol(index, coverage, truncated));
        }
        record.unresolved = coverage_gap(changed, covered).map(str::to_owned);
        Ok(record)
    }

    fn ranges(&mut self, path: &str) -> &[SymbolRange] {
        self.saved_ranges.entry(path.into()).or_insert_with(|| {
            self.files
                .get(path)
                .into_iter()
                .flatten()
                .filter_map(|&index| {
                    let (range, coverage) =
                        mapped_range(&self.flow, &self.graph.nodes[index], &mut self.starts)?;
                    Some((index, range, coverage))
                })
                .collect()
        })
    }
}

fn mapped_range<'a>(
    flow: &View<'_>,
    node: &'a Node,
    starts: &mut BTreeMap<SourceOwner<'a>, Vec<usize>>,
) -> Option<(Lines, Coverage)> {
    if let Some((content, span)) = flow.source_range(&node.id) {
        let starts = starts
            .entry((&node.build, &node.project, &node.file))
            .or_insert_with(|| line_starts(content));
        let first = line_for_offset(starts, span.start)?;
        let last = line_for_offset(starts, span.end - 1)?;
        return Some((Lines { first, last }, Coverage::AnalyzedRange));
    }
    (node.line > 0).then_some((
        Lines {
            first: node.line,
            last: node.line,
        },
        Coverage::DeclarationLine,
    ))
}

fn line_starts(content: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(
            content
                .bytes()
                .enumerate()
                .filter_map(|(offset, byte)| (byte == b'\n').then_some(offset + 1)),
        )
        .collect()
}

fn line_for_offset(starts: &[usize], offset: usize) -> Option<u32> {
    u32::try_from(starts.partition_point(|start| *start <= offset)).ok()
}

fn coverage_gap(changed: Lines, mut covered: Vec<Lines>) -> Option<&'static str> {
    if covered.is_empty() {
        return Some("no_saved_symbol_range");
    }
    covered.sort_by_key(|range| range.first);
    let mut next = changed.first;
    for range in covered {
        if range.first > next {
            break;
        }
        if range.last >= changed.last {
            return None;
        }
        next = next.max(range.last + 1);
    }
    Some("partially_mapped_range")
}

struct CallerGraph<'a> {
    graph: &'a GraphData,
    incoming: Vec<Vec<usize>>,
    entries: BTreeMap<&'a str, EntryKind>,
}

impl<'a> CallerGraph<'a> {
    fn new(graph: &'a GraphData) -> Self {
        let mut incoming = vec![Vec::new(); graph.nodes.len()];
        for (index, edge) in graph.edges.iter().enumerate() {
            if edge.classification().kind == RelationshipKind::Call {
                incoming[edge.to as usize].push(index);
            }
        }
        for edges in &mut incoming {
            edges.sort_by_key(|&index| {
                let edge = &graph.edges[index];
                (
                    &graph.nodes[edge.from as usize].id,
                    edge_possible(edge),
                    &edge.kind,
                    &edge.certainty,
                    &edge.provenance,
                )
            });
        }
        Self {
            graph,
            incoming,
            entries: entry_index(graph),
        }
    }

    fn symbol(&self, index: usize, coverage: Coverage, truncated: &mut bool) -> Symbol {
        let node = &self.graph.nodes[index];
        Symbol {
            id: node.id.clone(),
            label: node.label.clone(),
            line: node.line,
            coverage,
            entry_kind: self.entry(node),
            callers: self.callers(index, truncated),
        }
    }

    fn entry(&self, node: &Node) -> Option<EntryKind> {
        self.entries.get(node.id.as_str()).copied()
    }

    fn callers(&self, target: usize, truncated: &mut bool) -> Vec<Caller> {
        let search = self.search(target);
        let entries = search
            .discovered
            .iter()
            .copied()
            .filter(|&caller| self.entry(&self.graph.nodes[caller]).is_some());
        let direct = search.discovered.iter().copied().filter(|&caller| {
            self.entry(&self.graph.nodes[caller]).is_none()
                && search
                    .parent(caller)
                    .is_some_and(|index| self.graph.edges[index].to as usize == target)
        });
        let mut callers = entries
            .chain(direct)
            .take(MAX_CALLERS + 1)
            .collect::<Vec<_>>();
        *truncated |= search.truncated || callers.len() > MAX_CALLERS;
        callers.truncate(MAX_CALLERS);
        callers
            .into_iter()
            .map(|caller| self.chain(caller, &search))
            .collect()
    }

    fn search(&self, target: usize) -> Search {
        let mut search = Search::new(target);
        let mut remaining = MAX_VISITED_EDGES;
        while let Some((current, distance)) = search.queue.pop_front() {
            let incoming = &self.incoming[current];
            if distance == MAX_CHAIN_EDGES {
                search.truncated |= !incoming.is_empty();
                continue;
            }
            for &edge_index in incoming.iter().take(remaining) {
                let caller = self.graph.edges[edge_index].from as usize;
                search.observe(caller, edge_index, distance + 1);
            }
            let Some(next) = remaining.checked_sub(incoming.len()) else {
                search.truncated = true;
                break;
            };
            remaining = next;
        }
        search
    }

    fn chain(&self, caller: usize, search: &Search) -> Caller {
        let node = &self.graph.nodes[caller];
        let entry_kind = self.entry(node);
        let mut possible = entry_kind == Some(EntryKind::Candidate);
        let mut path = vec![node.id.clone()];
        let mut edges = Vec::new();
        let mut cursor = caller;
        while let Some(index) = search.parent(cursor) {
            let edge = &self.graph.edges[index];
            cursor = edge.to as usize;
            path.push(self.graph.nodes[cursor].id.clone());
            possible |= edge_possible(edge);
            edges.push(CallEvidence {
                kind: edge.kind.clone(),
                certainty: edge.certainty.clone(),
                provenance: edge.provenance.clone(),
            });
        }
        Caller {
            id: node.id.clone(),
            label: node.label.clone(),
            path,
            possible,
            entry_kind,
            edges,
        }
    }
}

fn entry_index(graph: &GraphData) -> BTreeMap<&str, EntryKind> {
    let Some(points) = &graph.entry_points else {
        return BTreeMap::new();
    };
    let mut entries = BTreeMap::new();
    for (kind, symbols) in [
        (EntryKind::ApplicationStart, &points.application_starts),
        (EntryKind::Framework, &points.framework_entries),
        (EntryKind::Candidate, &points.candidates),
    ] {
        for id in symbols {
            entries.entry(id.as_str()).or_insert(kind);
        }
    }
    entries
}

struct Search {
    queue: VecDeque<(usize, usize)>,
    edges: BTreeMap<usize, Option<usize>>,
    discovered: Vec<usize>,
    truncated: bool,
}

impl Search {
    fn new(target: usize) -> Self {
        Self {
            queue: VecDeque::from([(target, 0)]),
            edges: BTreeMap::from([(target, None)]),
            discovered: Vec::new(),
            truncated: false,
        }
    }

    fn observe(&mut self, caller: usize, edge: usize, distance: usize) {
        if let std::collections::btree_map::Entry::Vacant(path) = self.edges.entry(caller) {
            path.insert(Some(edge));
            self.discovered.push(caller);
            self.queue.push_back((caller, distance));
        }
    }

    fn parent(&self, node: usize) -> Option<usize> {
        self.edges.get(&node).copied().flatten()
    }
}

fn edge_possible(edge: &Edge) -> bool {
    edge.classification().resolution != TargetResolution::Direct
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        architecture::{ArchitectureFacts, FileFacts},
        flow::{FlowInventory, FunctionFlowIR, SourceSpan},
        model::EntryPoints,
    };

    const CODE: &str =
        "function worker() {\n  const name = 'Å';\n  return name;\n}\n\nfunction other() {}\n";
    const SOURCE: &str = "@repo/main.ts";
    const BUILD: &str = "@repo/tsconfig.json";

    fn node(id: &str, line: u32) -> Node {
        Node {
            id: id.into(),
            label: id.into(),
            project: "app".into(),
            build: BUILD.into(),
            language: "typescript".into(),
            file: SOURCE.into(),
            line,
            ..Default::default()
        }
    }

    fn edge(from: u32, to: u32, kind: &str, certainty: &str) -> Edge {
        Edge {
            from,
            to,
            kind: kind.into(),
            certainty: certainty.into(),
            provenance: "recorded".into(),
            count: 1,
            evidence: Vec::new(),
        }
    }

    fn artifact() -> Artifact {
        let graph = GraphData {
            nodes: vec![
                node("worker", 1),
                node("helper", 6),
                node("start", 7),
                node("reference", 8),
            ],
            edges: vec![
                edge(1, 0, "call", "direct"),
                edge(2, 1, "call", "possible"),
                edge(3, 0, "reference", "direct"),
                edge(0, 1, "call", "direct"),
            ],
            entry_points: Some(EntryPoints {
                application_starts: vec!["start".into()],
                framework_entries: vec!["reference".into()],
                ..Default::default()
            }),
            architecture: Some(ArchitectureFacts {
                version: crate::architecture::FACTS_VERSION,
                files: vec![FileFacts {
                    file: SOURCE.into(),
                    project: "app".into(),
                    build: BUILD.into(),
                    content: CODE.into(),
                    ..Default::default()
                }],
            }),
            flow_inventory: Some(FlowInventory {
                version: crate::flow::INVENTORY_VERSION,
                functions: vec![FunctionFlowIR {
                    symbol: "worker".into(),
                    file: SOURCE.into(),
                    project: "app".into(),
                    build: BUILD.into(),
                    language: "typescript".into(),
                    span: SourceSpan {
                        start: 0,
                        end: CODE.find("\n\n").unwrap(),
                    },
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };
        let identities = graph
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.id.clone()))
            .collect();
        let portable =
            super::super::portable::PortableGraph::from_runtime(&graph, &identities).unwrap();
        Artifact {
            schema: super::super::SCHEMA.into(),
            graph: serde_json::to_value(portable).unwrap(),
            extensions: Default::default(),
        }
    }

    fn change(old_start: u32, old_count: u32) -> Change {
        Change {
            path: "main.ts".into(),
            status: "modified".into(),
            old_start,
            old_count,
            unresolved: None,
        }
    }

    #[test]
    fn old_body_lines_find_saved_invocation_chains() {
        let result = preview(&artifact(), vec![change(3, 1)]).unwrap();
        let record = &result.records[0];
        assert_eq!(result.mapped_ranges, 1);
        assert_eq!(record.unresolved, None);
        let worker = &record.symbols[0];
        assert_eq!(worker.id, "worker");
        assert_eq!(worker.coverage, Coverage::AnalyzedRange);
        assert_eq!(worker.callers.len(), 2);
        let start = &worker.callers[0];
        assert_eq!(start.path, ["start", "helper", "worker"]);
        assert_eq!(start.entry_kind, Some(EntryKind::ApplicationStart));
        assert!(start.possible);
        assert_eq!(start.edges[0].provenance, "recorded");
        let direct = &worker.callers[1];
        assert_eq!(direct.path, ["helper", "worker"]);
        assert!(!direct.possible);
    }

    #[test]
    fn partially_covered_and_inserted_lines_remain_unresolved() {
        let result = preview(
            &artifact(),
            vec![change(4, 2), change(6, 0), change(6, 1), change(5, 1)],
        )
        .unwrap();
        assert_eq!((result.mapped_ranges, result.unmapped_ranges), (2, 2));
        let partial = &result.records[0];
        assert_eq!(partial.symbols[0].id, "worker");
        assert_eq!(
            partial.unresolved.as_deref(),
            Some("partially_mapped_range")
        );
        let inserted = &result.records[1];
        assert_eq!(inserted.unresolved.as_deref(), Some("no_baseline_lines"));
        assert!(inserted.symbols.is_empty());
        let declaration = &result.records[2].symbols[0];
        assert_eq!(declaration.coverage, Coverage::DeclarationLine);
        let gap = &result.records[3];
        assert_eq!(gap.unresolved.as_deref(), Some("no_saved_symbol_range"));
        assert!(gap.symbols.is_empty());
    }

    #[test]
    fn bounds_report_when_saved_matches_are_omitted() {
        let changes = (0..MAX_RECORDS + 1).map(|_| change(3, 1)).collect();
        let result = preview(&artifact(), changes).unwrap();
        assert!(result.truncated);
        assert_eq!(result.records.len(), MAX_RECORDS);
    }
}
