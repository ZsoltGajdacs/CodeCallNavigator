use super::{Certainty, EntryPoint, stable_id};
use crate::{artifact::callers, model::GraphData};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_VISITED_EDGES_PER_SYMBOL: usize = 2048;
const MAX_VISITED_EDGES_PER_SIDE: usize = 100_000;

pub(super) type Matches = BTreeMap<String, (String, Certainty)>;

pub(super) struct Search<'a> {
    graph: &'a GraphData,
    incoming: Vec<Vec<usize>>,
    entries: BTreeMap<&'a str, String>,
    cache: BTreeMap<usize, Matches>,
    remaining: usize,
    pub complete: bool,
}

impl<'a> Search<'a> {
    pub fn new(graph: &'a GraphData) -> Self {
        let mut entries = BTreeMap::new();
        let labels: BTreeMap<_, _> = graph
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), &node.label))
            .collect();
        let mut routes = route_names(graph);
        if let Some(points) = &graph.entry_points {
            for symbol in points
                .application_starts
                .iter()
                .chain(&points.framework_entries)
            {
                if let Some(label) = labels.get(symbol.as_str()) {
                    entries.insert(
                        symbol.as_str(),
                        routes
                            .remove(symbol.as_str())
                            .unwrap_or_else(|| (*label).clone()),
                    );
                }
            }
        }
        Self {
            graph,
            incoming: callers::incoming(graph),
            entries,
            cache: BTreeMap::new(),
            remaining: MAX_VISITED_EDGES_PER_SIDE,
            complete: graph.entry_points.is_some() || graph.nodes.is_empty(),
        }
    }

    pub fn extend(&mut self, matches: &mut Matches, target: usize) {
        if self.entries.is_empty() {
            return;
        }
        if !self.cache.contains_key(&target) {
            let found = self.trace(target);
            self.cache.insert(target, found);
        }
        for (id, (name, certainty)) in &self.cache[&target] {
            matches
                .entry(id.clone())
                .and_modify(|(_, existing)| *existing = (*existing).max(*certainty))
                .or_insert_with(|| (name.clone(), *certainty));
        }
    }

    fn trace(&mut self, target: usize) -> Matches {
        let mut frontier = Frontier::new(target);
        let mut result = Matches::new();
        let mut budget = self.remaining.min(MAX_VISITED_EDGES_PER_SYMBOL);
        while let Some((index, certainty)) = frontier.next() {
            let node = &self.graph.nodes[index];
            if let Some(name) = self.entries.get(node.id.as_str()) {
                result.insert(node.id.clone(), (name.clone(), certainty));
            }
            let incoming = &self.incoming[index];
            if incoming.len() > budget {
                self.complete = false;
            }
            frontier.enqueue_callers(
                self.graph,
                &incoming[..incoming.len().min(budget)],
                certainty,
            );
            let used = incoming.len().min(budget);
            budget -= used;
            self.remaining -= used;
        }
        result
    }
}

struct Frontier {
    visited: BTreeMap<usize, Certainty>,
    queue: VecDeque<(usize, Certainty)>,
}

impl Frontier {
    fn new(target: usize) -> Self {
        Self {
            visited: BTreeMap::from([(target, Certainty::Direct)]),
            queue: VecDeque::from([(target, Certainty::Direct)]),
        }
    }

    fn next(&mut self) -> Option<(usize, Certainty)> {
        while let Some((index, certainty)) = self.queue.pop_front() {
            if self.visited[&index] == certainty {
                return Some((index, certainty));
            }
        }
        None
    }

    fn enqueue_callers(&mut self, graph: &GraphData, incoming: &[usize], current: Certainty) {
        for &edge_index in incoming {
            let edge = &graph.edges[edge_index];
            let caller = edge.from as usize;
            let certainty = if callers::possible(edge) {
                Certainty::Possible
            } else {
                current
            };
            if self.visited.get(&caller).is_none_or(|old| *old < certainty) {
                self.visited.insert(caller, certainty);
                self.queue.push_back((caller, certainty));
            }
        }
    }
}

fn route_names(graph: &GraphData) -> BTreeMap<&str, String> {
    let mut names = BTreeMap::<&str, BTreeSet<String>>::new();
    let routes = graph
        .boundaries
        .iter()
        .flat_map(|inventory| &inventory.routes);
    for route in routes.filter(|route| route.kind == "http") {
        if let Some((symbol, path)) = route.symbol.as_deref().zip(route.path.as_ref()) {
            let methods = route.methods.join("/");
            let name = if methods.is_empty() {
                path.clone()
            } else {
                format!("{methods} {path}")
            };
            names.entry(symbol).or_default().insert(name);
        }
    }
    names
        .into_iter()
        .map(|(symbol, names)| (symbol, names.into_iter().collect::<Vec<_>>().join(", ")))
        .collect()
}

pub(super) fn combine(before: &Matches, after: &Matches) -> Vec<EntryPoint> {
    let symbols: BTreeSet<_> = before.keys().chain(after.keys()).collect();
    let mut entries: Vec<_> = symbols
        .into_iter()
        .map(|symbol| {
            let old = before.get(symbol);
            let new = after.get(symbol);
            EntryPoint {
                id: stable_id("entry", &[symbol]),
                name: new
                    .or(old)
                    .expect("the union contains this symbol")
                    .0
                    .clone(),
                before: old.map(|(_, certainty)| *certainty),
                after: new.map(|(_, certainty)| *certainty),
            }
        })
        .collect();
    entries.sort_by(|left, right| (&left.name, &left.id).cmp(&(&right.name, &right.id)));
    entries
}

pub(super) fn routes(
    before: Option<&mut Search<'_>>,
    before_members: impl IntoIterator<Item = usize>,
    after: Option<&mut Search<'_>>,
    after_members: impl IntoIterator<Item = usize>,
) -> Vec<EntryPoint> {
    fn collect(
        search: Option<&mut Search<'_>>,
        members: impl IntoIterator<Item = usize>,
    ) -> Matches {
        let mut matches = Matches::new();
        if let Some(search) = search {
            for member in members {
                search.extend(&mut matches, member);
            }
        }
        matches
    }
    combine(
        &collect(before, before_members),
        &collect(after, after_members),
    )
}
