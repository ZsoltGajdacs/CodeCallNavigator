//! Small generation-owned cache for full-graph facts used by bounded projections.
use super::{Filter, GraphData};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

const CACHE_LIMIT: usize = 4;

pub struct FilterSummary {
    pub cyclic: Vec<bool>,
    pub hidden_nodes: usize,
    pub hidden_edges: usize,
    pub(super) cycles: Vec<Vec<u32>>,
}

impl FilterSummary {
    pub(super) fn new(data: &GraphData, filter: &Filter, cycles: Vec<Vec<u32>>) -> Self {
        let mut cyclic = vec![false; data.nodes.len()];
        for &node in cycles.iter().flatten() {
            cyclic[node as usize] = true;
        }
        let hidden_nodes = data
            .nodes
            .iter()
            .filter(|node| !filter.accepts_node(node))
            .count();
        let hidden_edges = data
            .edges
            .iter()
            .filter(|edge| {
                !filter.accepts_edge(edge)
                    || !filter.accepts_node(&data.nodes[edge.from as usize])
                    || !filter.accepts_node(&data.nodes[edge.to as usize])
            })
            .count();
        Self {
            cyclic,
            hidden_nodes,
            hidden_edges,
            cycles,
        }
    }
}

#[derive(Default)]
pub(super) struct SummaryCache {
    entries: Mutex<VecDeque<(Filter, Arc<FilterSummary>)>>,
}

impl SummaryCache {
    pub(super) fn get(
        &self,
        filter: &Filter,
        compute: impl FnOnce() -> FilterSummary,
    ) -> Arc<FilterSummary> {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(index) = entries.iter().position(|(key, _)| key == filter) {
            crate::metrics::count("filter_summary.cache_hits", 1);
            let entry = entries.remove(index).expect("known cache entry");
            let result = entry.1.clone();
            entries.push_back(entry);
            return result;
        }
        crate::metrics::count("filter_summary.cache_misses", 1);
        let _stage = crate::metrics::stage("graph.filter_summary");
        let result = Arc::new(compute());
        if entries.len() == CACHE_LIMIT {
            entries.pop_front();
        }
        entries.push_back((filter.clone(), result.clone()));
        result
    }
}
