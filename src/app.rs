pub use crate::navigation::{Navigation, SavedSession};
use crate::{
    analyzers::AnalyzerRuntime,
    discovery::BuildTarget,
    jobs::Jobs,
    projection::{FlowDirection, GraphView, Grouping, Request},
};
use codegraph::{graph::GraphIndex, store::SqliteStore};
use rune::TextEdit;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct TargetChoice {
    pub target: BuildTarget,
    pub selected: bool,
}

pub struct State {
    pub root: TextEdit,
    pub query: TextEdit,
    pub targets: Vec<TargetChoice>,
    pub saved_builds: Vec<PathBuf>,
    pub graph: Option<Arc<GraphIndex>>,
    pub store: Option<Arc<SqliteStore>>,
    pub runtime: Option<Arc<AnalyzerRuntime>>,
    pub nav: Navigation,
    pub back: Vec<Navigation>,
    pub forward: Vec<Navigation>,
    pub graph_view: GraphView,
    pub hits: Vec<u32>,
    pub search_page: usize,
    pub search_more: bool,
    pub neighbors: (Vec<u32>, Vec<u32>),
    pub neighbor_pages: [usize; 2],
    pub relationships: Vec<u32>,
    pub relationship_page: usize,
    pub selected_edge: Option<u32>,
    pub show_provenance: bool,
    pub status: String,
    pub warning: String,
    pub stale: bool,
    pub analyzing: bool,
    pub scene: bool,
    pub deterministic: bool,
    pub closing: bool,
    pub epoch: u64,
    pub generation: Option<String>,
    pub revision: u64,
    pub needs_layout: bool,
    pub dirty_session: bool,
    pub root_was_edited: bool,
    pub last_query: String,
    pub next_stale_check: f64,
    pub viewport: [f32; 2],
    pub auto_fit: bool,
    pub visible_revision: Option<(u64, u64)>,
    pub cancel: Arc<AtomicBool>,
    pub jobs: Jobs,
}

impl State {
    pub fn new() -> Self {
        Self {
            root: TextEdit::new(""),
            query: TextEdit::new(""),
            targets: Vec::new(),
            saved_builds: Vec::new(),
            graph: None,
            store: None,
            runtime: None,
            nav: Navigation::default(),
            back: Vec::new(),
            forward: Vec::new(),
            graph_view: GraphView::default(),
            hits: Vec::new(),
            search_page: 0,
            search_more: false,
            neighbors: (Vec::new(), Vec::new()),
            neighbor_pages: [0, 0],
            relationships: Vec::new(),
            relationship_page: 0,
            selected_edge: None,
            show_provenance: false,
            status: "Choose a project folder to begin".into(),
            warning: String::new(),
            stale: false,
            analyzing: false,
            scene: false,
            deterministic: false,
            closing: false,
            epoch: 0,
            generation: None,
            revision: 0,
            needs_layout: false,
            dirty_session: false,
            root_was_edited: false,
            last_query: String::new(),
            next_stale_check: 0.0,
            viewport: [720.0, 620.0],
            auto_fit: true,
            visible_revision: None,
            cancel: Arc::new(AtomicBool::new(false)),
            jobs: Jobs::default(),
        }
    }

    pub fn checkpoint(&mut self) {
        self.back.push(self.nav.clone());
        if self.back.len() > 100 {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    pub fn changed(&mut self) {
        self.revision += 1;
        self.needs_layout = true;
        self.dirty_session = true;
        self.refresh_neighbors();
    }

    pub fn selected(&self) -> Option<u32> {
        self.graph
            .as_ref()?
            .node_index(self.nav.selected.as_deref()?)
    }

    pub fn select(&mut self, node: u32, focus: bool) {
        let Some(graph) = &self.graph else {
            return;
        };
        let Some(item) = graph.data.nodes.get(node as usize) else {
            return;
        };
        let identity = item.id.clone();
        self.checkpoint();
        self.set_selection(identity, focus);
    }

    fn set_selection(&mut self, identity: String, focus: bool) {
        self.nav.selected = Some(identity.clone());
        if focus {
            self.nav.focus_method(identity);
            self.auto_fit = true;
        }
        self.nav.page = 0;
        self.neighbor_pages = [0, 0];
        self.relationship_page = 0;
        self.selected_edge = None;
        self.changed();
    }

    pub fn reveal(&mut self, node: u32) {
        let Some(item) = self
            .graph
            .as_ref()
            .and_then(|graph| graph.data.nodes.get(node as usize))
        else {
            return;
        };
        let (identity, test, generated, unavailable) = (
            item.id.clone(),
            item.is_test,
            item.is_generated,
            item.body_unavailable,
        );
        self.checkpoint();
        self.nav.filter.include_tests |= test;
        self.nav.filter.include_generated |= generated;
        self.nav.filter.include_unavailable |= unavailable;
        self.set_selection(identity, true);
    }

    pub fn activate(&mut self, key: String, member: Option<u32>) {
        if let Some(node) = member {
            self.select(node, false);
            return;
        }
        self.checkpoint();
        self.nav.expanded.insert(key);
        self.nav.page = 0;
        self.auto_fit = true;
        self.changed();
    }

    pub fn overview(&mut self) {
        self.checkpoint();
        self.nav.overview();
        self.finish_navigation();
    }

    fn finish_navigation(&mut self) {
        self.auto_fit = true;
        self.changed();
    }

    pub fn set_grouping(&mut self, grouping: Grouping) {
        self.checkpoint();
        self.nav.set_grouping(grouping);
        self.finish_navigation();
    }

    pub fn set_direction(&mut self, direction: FlowDirection) {
        self.checkpoint();
        self.nav.set_direction(direction);
        self.finish_navigation();
    }

    pub fn collapse_all(&mut self) {
        self.checkpoint();
        self.nav.expanded.clear();
        self.nav.overview();
        self.finish_navigation();
    }

    pub fn history(&mut self, forward: bool) {
        let destination = if forward {
            &mut self.forward
        } else {
            &mut self.back
        };
        let Some(previous) = destination.pop() else {
            return;
        };
        let old = std::mem::replace(&mut self.nav, previous);
        if forward {
            self.back.push(old);
        } else {
            self.forward.push(old);
        }
        self.auto_fit = false;
        self.changed();
    }

    pub fn request(&self) -> Option<Request> {
        let graph = self.graph.as_ref()?;
        Some(Request {
            grouping: self.nav.grouping,
            expanded: self.nav.expanded.clone(),
            scope: self.nav.scope(graph),
            direction: self.nav.direction,
            depth: self.nav.depth,
            filter: self.nav.filter.clone(),
            page: self.nav.page,
        })
    }

    pub fn refresh_neighbors(&mut self) {
        self.request_neighbors();
    }

    pub fn session(&self) -> SavedSession {
        SavedSession {
            root: self.root.text.clone(),
            selected_builds: self
                .targets
                .iter()
                .filter(|choice| choice.selected)
                .map(|choice| choice.target.path.clone())
                .collect(),
            navigation: self.nav.clone(),
        }
    }

    pub fn cancel_analysis(&mut self) {
        self.cancel.store(true, Ordering::Release);
        self.status = "Cancelling analysis; the previous graph is preserved".into();
    }

    pub fn root_edited(&mut self) {
        self.root_was_edited = true;
        self.jobs.discovery.cancel();
        self.targets.clear();
        self.saved_builds.clear();
        self.dirty_session = true;
        self.status = if self.graph.is_some() {
            "Folder changed · the last successful graph remains until refresh"
        } else {
            "Folder changed · find builds to choose what to analyze"
        }
        .into();
    }

    pub fn view_is_current(&self) -> bool {
        self.visible_revision == Some((self.epoch, self.revision))
    }

    pub fn step_neighbors(&mut self, incoming: bool, forward: bool) {
        let index = usize::from(!incoming);
        let count = if incoming {
            self.neighbors.0.len()
        } else {
            self.neighbors.1.len()
        };
        self.neighbor_pages[index] = next_page(self.neighbor_pages[index], count, forward);
    }

    pub fn step_relationships(&mut self, forward: bool) {
        self.relationship_page =
            next_page(self.relationship_page, self.relationships.len(), forward);
    }
}

pub const RESULT_PAGE_SIZE: usize = 30;

impl Drop for State {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

fn next_page(current: usize, count: usize, forward: bool) -> usize {
    if forward {
        (current + 1).min(count.saturating_sub(1) / RESULT_PAGE_SIZE)
    } else {
        current.saturating_sub(1)
    }
}
