//! Transient search lifecycle; matching belongs to core and navigation to graph interaction.
use crate::{
    app::{RESULT_PAGE_SIZE, State},
    search_shortcut::DoubleShift,
};
use codecallnavigator::{
    graph::GraphIndex,
    search::{NameSearchIndex, NameSearchQuery, SearchCategory, SearchHit, SearchPage},
};
use rune::{Key, KeyEvent, TextEdit, Tx};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Request {
    serial: u64,
    epoch: u64,
    generation: Option<String>,
    query: String,
    category: SearchCategory,
    page: usize,
}

pub enum Results {
    Idle,
    Loading,
    Ready(SearchPage),
    Failed(String),
}

pub struct Workspace {
    pub open: bool,
    pub input: TextEdit,
    pub category: SearchCategory,
    pub page: usize,
    pub selected: usize,
    pub results: Results,
    pub shortcut: DoubleShift,
    source: Option<Arc<GraphIndex>>,
    index: Option<Arc<NameSearchIndex>>,
    serial: u64,
    requested: Option<Request>,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            open: false,
            input: TextEdit::new(""),
            category: SearchCategory::All,
            page: 0,
            selected: 0,
            results: Results::Idle,
            shortcut: DoubleShift::default(),
            source: None,
            index: None,
            serial: 0,
            requested: None,
        }
    }
}
impl Workspace {
    pub fn hits(&self) -> &[SearchHit] {
        match &self.results {
            Results::Ready(page) => &page.hits,
            _ => &[],
        }
    }
    pub fn more(&self) -> bool {
        matches!(&self.results, Results::Ready(page) if page.more)
    }
    fn request(&self, state: &State) -> Request {
        Request {
            serial: self.serial,
            epoch: state.epoch,
            generation: state.generation.clone(),
            query: self.input.text.clone(),
            category: self.category,
            page: self.page,
        }
    }
}

pub struct IndexOutput {
    serial: u64,
    index: Result<Arc<NameSearchIndex>, String>,
}
pub struct SearchOutput {
    request: Request,
    result: Result<SearchPage, String>,
}

impl State {
    pub fn search_window_height(&self) -> f32 {
        if self.flow_workspace.current.is_some() {
            self.flow_workspace.window[1]
        } else {
            self.source_popup.window[1]
        }
    }
    pub fn open_search(&mut self) {
        self.cancel_graph_press();
        self.close_source();
        self.ui.overlay = None;
        self.search.open = true;
        self.search.input.focused = true;
        self.search
            .input
            .set_selection(0, self.search.input.text.len());
        self.search.shortcut.reset();
    }
    pub fn close_search(&mut self) {
        self.search.open = false;
        self.search.input.focused = false;
        self.search.shortcut.reset();
    }
    pub fn set_search_category(&mut self, category: SearchCategory) {
        if self.search.category != category {
            self.search.category = category;
            self.request_search();
        }
        self.search.input.focused = true;
    }
    pub fn request_search(&mut self) {
        self.search.page = 0;
        self.search.selected = 0;
        self.search.requested = None;
        self.search.results = Results::Loading;
        self.jobs.search.cancel();
    }
    pub fn step_search(&mut self, forward: bool) {
        if forward && !self.search.more() {
            return;
        }
        self.search.page = if forward {
            self.search.page + 1
        } else {
            self.search.page.saturating_sub(1)
        };
        self.search.selected = 0;
        self.search.requested = None;
        self.search.results = Results::Loading;
        self.jobs.search.cancel();
    }
    pub fn activate_search(&mut self, index: usize) {
        if !same_graph(self.search.source.as_ref(), self.graph.as_ref())
            || self.search.requested.as_ref() != Some(&self.search.request(self))
        {
            return;
        }
        let Some(hit) = self.search.hits().get(index).cloned() else {
            return;
        };
        self.close_search();
        self.close_flow();
        self.reveal_search_target(hit.target);
    }
}

fn same_graph(a: Option<&Arc<GraphIndex>>, b: Option<&Arc<GraphIndex>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

fn sync_graph(state: &mut State) -> bool {
    if same_graph(state.search.source.as_ref(), state.graph.as_ref()) {
        return false;
    }
    state.jobs.search.cancel();
    state.jobs.search_index.cancel();
    state.search.serial += 1;
    state.search.source = state.graph.clone();
    state.search.index = None;
    state.search.requested = None;
    state.search.results = Results::Idle;
    state.search.page = 0;
    state.search.selected = 0;
    if let Some(graph) = state.graph.clone() {
        let serial = state.search.serial;
        state
            .jobs
            .search_index
            .request(graph, move |graph| IndexOutput {
                serial,
                index: Ok(Arc::new(NameSearchIndex::new(&graph.data))),
            });
    }
    true
}

fn poll_index(state: &mut State) -> bool {
    let Some(output) = state.jobs.search_index.take() else {
        return false;
    };
    if output.serial != state.search.serial {
        return false;
    }
    match output.index {
        Ok(index) => state.search.index = Some(index),
        Err(error) => state.search.results = Results::Failed(error),
    }
    true
}

fn submit(state: &mut State) -> bool {
    let request = state.search.request(state);
    if state.search.requested.as_ref() == Some(&request) {
        return false;
    }
    if state
        .search
        .requested
        .as_ref()
        .is_some_and(|old| old.query != request.query || old.category != request.category)
    {
        state.search.page = 0;
        state.search.selected = 0;
    }
    let request = state.search.request(state);
    state.jobs.search.cancel();
    if request.query.trim().is_empty() {
        state.search.results = Results::Idle;
        state.search.requested = Some(request);
        return true;
    }
    if state.graph.is_none() {
        state.search.results = Results::Idle;
        state.search.requested = Some(request);
        return true;
    }
    let Some(index) = state.search.index.clone() else {
        if !matches!(state.search.results, Results::Failed(_)) {
            state.search.results = Results::Loading;
        }
        return false;
    };
    state.search.requested = Some(request.clone());
    state.search.results = Results::Loading;
    state
        .jobs
        .search
        .request((index, request), |(index, request)| SearchOutput {
            result: Ok(index.search(&NameSearchQuery {
                text: &request.query,
                category: request.category,
                offset: request.page * RESULT_PAGE_SIZE,
                limit: RESULT_PAGE_SIZE,
            })),
            request,
        });
    true
}

fn poll_results(state: &mut State) -> bool {
    let Some(output) = state.jobs.search.take() else {
        return false;
    };
    if output.request != state.search.request(state) {
        return false;
    }
    state.search.selected = 0;
    state.search.results = match output.result {
        Ok(page) => Results::Ready(page),
        Err(error) => Results::Failed(error),
    };
    true
}

pub fn poll(state: &mut State) -> bool {
    sync_graph(state) | poll_index(state) | submit(state) | poll_results(state)
}

pub fn key(state: &mut State, event: &KeyEvent, tx: &mut Tx) -> bool {
    let double_shift = state.search.shortcut.key(event, tx.clock());
    if double_shift || event.is_command('f') {
        state.open_search();
        return true;
    }
    if !state.search.open {
        return false;
    }
    match event.key {
        Key::Escape => state.close_search(),
        Key::Enter => state.activate_search(state.search.selected),
        Key::Down => {
            state.search.selected =
                (state.search.selected + 1).min(state.search.hits().len().saturating_sub(1));
            tx.scroll_into_view(format!("search-result-{}", state.search.selected));
        }
        Key::Up => {
            state.search.selected = state.search.selected.saturating_sub(1);
            tx.scroll_into_view(format!("search-result-{}", state.search.selected));
        }
        Key::Command(_) => {
            return !event.is_command('a')
                && !event.is_command('c')
                && !event.is_command('v')
                && !event.is_command('x');
        }
        _ => return false,
    }
    true
}

/// Observe interruptions before a focused editor can consume them.
pub(crate) fn key_activity(state: &mut State, event: &KeyEvent, tx: &mut Tx) {
    if !matches!(event.key, Key::Modifier { .. }) {
        state.search.shortcut.reset();
    }
    tx.live_only();
}
pub(crate) fn pointer_activity(state: &mut State, tx: &mut Tx) {
    state.search.shortcut.reset();
    tx.live_only();
}
pub(crate) fn focus_changed(state: &mut State, _: bool, tx: &mut Tx) {
    state.search.shortcut.reset();
    tx.live_only();
}
