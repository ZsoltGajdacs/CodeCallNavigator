use crate::app::{RESULT_PAGE_SIZE, State};
use codegraph::{
    graph::Filter,
    store::{SqliteStore, SymbolSearch},
};
use std::sync::Arc;
type Fallible<T> = Result<T, String>;

pub struct SearchOutput {
    pub epoch: u64,
    pub generation: String,
    pub query: String,
    pub page: usize,
    pub result: Fallible<Vec<u32>>,
}

struct SearchRequest {
    store: Arc<SqliteStore>,
    generation: String,
    epoch: u64,
    query: String,
    page: usize,
}

fn search_worker(request: SearchRequest) -> SearchOutput {
    let filter = Filter {
        include_tests: true,
        include_generated: true,
        include_unavailable: true,
        ..Default::default()
    };
    let result = request
        .store
        .search_generation(
            &request.generation,
            &SymbolSearch {
                prefix: &request.query,
                filter: &filter,
                limit: RESULT_PAGE_SIZE + 1,
                offset: request.page * RESULT_PAGE_SIZE,
            },
        )
        .map_err(|error| error.to_string());
    SearchOutput {
        epoch: request.epoch,
        generation: request.generation,
        query: request.query,
        page: request.page,
        result,
    }
}
impl State {
    pub fn request_search(&mut self) {
        self.search_page = 0;
        self.request_search_page();
    }

    pub fn step_search(&mut self, forward: bool) {
        if forward && !self.search_more {
            return;
        }
        self.search_page = if forward {
            self.search_page + 1
        } else {
            self.search_page.saturating_sub(1)
        };
        self.request_search_page();
    }

    fn request_search_page(&mut self) {
        let query = self.query.text.clone();
        self.last_query = query.clone();
        self.hits.clear();
        self.search_more = false;
        if query.trim().is_empty() {
            self.jobs.search.cancel();
            return;
        }
        if self.scene {
            self.search_fixture();
            return;
        }
        let (Some(store), Some(generation)) = (self.store.clone(), self.generation.clone()) else {
            return;
        };
        let request = SearchRequest {
            store,
            generation,
            epoch: self.epoch,
            page: self.search_page,
            query,
        };
        self.jobs.search.request(request, search_worker);
    }

    fn search_fixture(&mut self) {
        let Some(graph) = &self.graph else {
            return;
        };
        let query = self.query.text.to_lowercase();
        let mut hits: Vec<_> = graph
            .data
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                node.label.to_lowercase().contains(&query)
                    || node.signature.to_lowercase().contains(&query)
            })
            .map(|(index, _)| index as u32)
            .skip(self.search_page * RESULT_PAGE_SIZE)
            .take(RESULT_PAGE_SIZE + 1)
            .collect();
        self.search_more = hits.len() > RESULT_PAGE_SIZE;
        hits.truncate(RESULT_PAGE_SIZE);
        self.hits = hits;
    }
}

pub fn poll(state: &mut State) -> bool {
    let Some(output) = state.jobs.search.take() else {
        return false;
    };
    if output.epoch != state.epoch {
        return false;
    }
    if output.query != state.query.text {
        return false;
    }
    if output.page != state.search_page {
        return false;
    }
    if state.generation.as_deref() != Some(output.generation.as_str()) {
        return false;
    }
    match output.result {
        Ok(mut hits) => {
            state.search_more = hits.len() > RESULT_PAGE_SIZE;
            hits.truncate(RESULT_PAGE_SIZE);
            state.hits = hits;
        }
        Err(error) => state.warning = error,
    }
    true
}
