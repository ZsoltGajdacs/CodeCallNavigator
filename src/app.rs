pub use crate::navigation::{Navigation, SavedSession};
use crate::{
    discovery::BuildTarget,
    jobs::Jobs,
    projection::{FlowDirection, GraphView, Grouping, Request},
    runtime::Runtime as AnalyzerRuntime,
};
use codecallnavigator::{graph::GraphIndex, store::SqliteStore};
use rune::TextEdit;
use std::{
    cell::RefCell,
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
    pub explorer: crate::explorer::Workspace,
    pub flow_workspace: crate::flow_workspace::Workspace,
    pub root: TextEdit,
    pub search: crate::search::Workspace,
    pub targets: Vec<TargetChoice>,
    pub build_discovery: crate::build_discovery::Workspace,
    pub saved_builds: Vec<PathBuf>,
    pub saved_targets: Option<Vec<crate::discovery::BuildIdentity>>,
    pub graph: Option<Arc<GraphIndex>>,
    pub query_context: Option<crate::engine_client::QueryContext>,
    pub presentation_cache: Option<crate::presentation_cache::PresentationCache>,
    pub browser_context: Option<crate::engine_client::QueryContext>,
    pub fixture_queries: bool,
    pub architecture: Option<Arc<codecallnavigator::architecture::LayerAssignments>>,
    pub architecture_selection: Option<codecallnavigator::architecture::Selection>,
    pub architecture_workspace: crate::architecture_workspace::Workspace,
    pub analysis_metrics: Option<codecallnavigator::metrics::Metrics>,
    pub store: Option<Arc<SqliteStore>>,
    pub runtime: Option<Arc<AnalyzerRuntime>>,
    pub nav: Navigation,
    pub ui: crate::presentation::Presentation,
    pub source_popup: crate::source_popup::Popup,
    pub back: Vec<Navigation>,
    pub forward: Vec<Navigation>,
    pub graph_view: GraphView,
    pub edge_geometry: RefCell<crate::edges::GeometryCache>,
    pub graph_emphasis: RefCell<crate::graph_emphasis::Cache>,
    pub visual_selection: Option<String>,
    pub graph_interaction: crate::graph_interaction::Interaction,
    pub neighbors: (Vec<u32>, Vec<u32>),
    pub neighbor_pages: [usize; 2],
    pub relationships: Vec<u32>,
    pub relationship_classifications:
        std::collections::HashMap<u32, codecallnavigator::model::RelationshipClassification>,
    pub relationship_page: usize,
    pub selected_edge: Option<u32>,
    pub show_provenance: bool,
    pub status: String,
    pub warning: String,
    pub stale: bool,
    pub analyzing: bool,
    pub clearing_cache: bool,
    pub scene: bool,
    pub deterministic: bool,
    pub closing: bool,
    pub epoch: u64,
    pub generation: Option<String>,
    pub revision: u64,
    pub needs_layout: bool,
    pub dirty_session: bool,
    pub root_was_edited: bool,
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
            explorer: crate::explorer::Workspace::default(),
            flow_workspace: crate::flow_workspace::Workspace::default(),
            root: TextEdit::new(""),
            search: crate::search::Workspace::default(),
            targets: Vec::new(),
            build_discovery: Default::default(),
            saved_builds: Vec::new(),
            saved_targets: None,
            graph: None,
            query_context: None,
            presentation_cache: None,
            browser_context: None,
            fixture_queries: false,
            architecture: None,
            architecture_selection: None,
            architecture_workspace: Default::default(),
            analysis_metrics: None,
            store: None,
            runtime: None,
            nav: Navigation::default(),
            ui: crate::presentation::Presentation::default(),
            source_popup: crate::source_popup::Popup::default(),
            back: Vec::new(),
            forward: Vec::new(),
            graph_view: GraphView::default(),
            edge_geometry: RefCell::default(),
            graph_emphasis: RefCell::default(),
            visual_selection: None,
            graph_interaction: Default::default(),
            neighbors: (Vec::new(), Vec::new()),
            neighbor_pages: [0, 0],
            relationships: Vec::new(),
            relationship_classifications: Default::default(),
            relationship_page: 0,
            selected_edge: None,
            show_provenance: false,
            status: "Choose a project folder to begin".into(),
            warning: String::new(),
            stale: false,
            analyzing: false,
            clearing_cache: false,
            scene: false,
            deterministic: false,
            closing: false,
            epoch: 0,
            generation: None,
            revision: 0,
            needs_layout: false,
            dirty_session: false,
            root_was_edited: false,
            next_stale_check: 0.0,
            viewport: [720.0, 620.0],
            auto_fit: true,
            visible_revision: None,
            cancel: Arc::new(AtomicBool::new(false)),
            jobs: Jobs::default(),
        }
    }

    pub fn checkpoint(&mut self) {
        self.record_graph_step(self.nav.clone());
    }

    pub fn uses_fixture_queries(&self) -> bool {
        self.fixture_queries || self.scene || self.deterministic
    }

    pub fn graph_queries(&self) -> Result<crate::graph_queries::GraphQueries, String> {
        let graph = self.graph.clone().ok_or("No analyzed graph is loaded")?;
        if self.uses_fixture_queries() {
            return Ok(crate::graph_queries::GraphQueries::fixture(graph));
        }
        let context = self
            .query_context
            .clone()
            .ok_or("The loaded graph has no CLI query context; refresh the workspace")?;
        Ok(
            crate::graph_queries::GraphQueries::cli(graph, context).with_layers(
                self.architecture_selection,
                self.architecture_workspace.signature(),
            ),
        )
    }

    pub fn flow_supported(&self, language: &str) -> bool {
        if self.uses_fixture_queries() {
            return crate::fixture_queries::flow_supported(language);
        }
        self.query_context
            .as_ref()
            .is_some_and(|context| context.flow_supported(language))
    }

    #[cfg(test)]
    pub fn fixture() -> Self {
        let mut state = Self::new();
        state.fixture_queries = true;
        state
    }

    /// Install a new layout, including same-sized replacements, before rendering it.
    pub fn replace_graph_view(&mut self, view: GraphView) {
        self.graph_view = view;
        *self.edge_geometry.get_mut() = Default::default();
        *self.graph_emphasis.get_mut() = Default::default();
    }

    pub fn changed(&mut self) {
        self.close_source();
        self.cancel_graph_press();
        self.revision += 1;
        self.needs_layout = true;
        self.graph_interaction.center_after_layout = self.has_graph_focus() && !self.nav.show_path;
        self.dirty_session = true;
        self.refresh_neighbors();
    }

    pub fn selected(&self) -> Option<u32> {
        self.graph
            .as_ref()?
            .node_index(self.nav.selected.as_deref()?)
    }

    pub fn overview(&mut self) {
        self.checkpoint();
        self.nav.overview();
        self.finish_navigation();
    }

    fn finish_navigation(&mut self) {
        self.graph_interaction.center_after_layout = false;
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
        self.visual_selection = None;
        if forward {
            self.back.push(old);
        } else {
            self.forward.push(old);
        }
        self.auto_fit = false;
        self.changed();
        self.graph_interaction.center_after_layout = false;
    }

    pub fn request(&self) -> Option<Request> {
        let graph = self.graph.as_ref()?;
        Some(Request {
            focus_target: self.nav.focus_target.clone(),
            grouping: self.nav.grouping,
            expanded: self.nav.expanded.clone(),
            scope: self.nav.scope(graph),
            direction: self.nav.direction,
            depth: self.nav.depth,
            filter: self.nav.filter.clone(),
            page: self.nav.page,
            architecture: self.architecture.clone(),
            layer_tab: self.nav.layer_tab.clone(),
        })
    }

    pub fn refresh_neighbors(&mut self) {
        self.request_neighbors();
    }

    pub fn session(&self) -> SavedSession {
        SavedSession {
            presentation_cache: self.presentation_cache.clone(),
            graph_context: self
                .query_context
                .as_ref()
                .map(|context| context.descriptor.clone()),
            root: self.root.text.clone(),
            selected_builds: self
                .targets
                .iter()
                .filter(|choice| choice.selected)
                .map(|choice| choice.target.path.clone())
                .collect(),
            selected_targets: Some(
                self.targets
                    .iter()
                    .filter(|choice| choice.selected)
                    .map(|choice| choice.target.identity())
                    .collect(),
            ),
            navigation: self.nav.clone(),
            presentation: self.ui.preferences(),
            architecture_selection: self.architecture_selection,
        }
    }

    pub fn cancel_analysis(&mut self) {
        self.cancel.store(true, Ordering::Release);
        self.status = "Cancelling analysis; the previous graph is preserved".into();
    }

    pub fn root_edited(&mut self) {
        self.jobs.file_focus_cancel.store(true, Ordering::Release);
        self.jobs.file_focus.cancel();
        self.root_was_edited = true;
        self.reset_build_discovery();
        self.close_overlay();
        self.saved_builds.clear();
        self.saved_targets = None;
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
