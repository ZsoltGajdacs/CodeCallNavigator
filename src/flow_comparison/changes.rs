//! The source outline can load before a saved graph catalog is available.
use super::{
    Selection, Workspace,
    identity::Installation,
    preview::mismatch,
    requests::{Clock, StringError},
    work::{Tracked, Workers},
};
use crate::engine_client::{
    QueryContext,
    flow_comparison::{
        BoundComparison, BoundSourceComparison, Change, ChangeKind, ChangeList, Error, SourceDiff,
    },
};
use rune::{Latest, Live, Waker};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

enum ListAnswer {
    Opened(PathBuf, Option<Box<(BoundSourceComparison, ChangeList)>>),
    Page(Option<ChangeList>),
}

#[derive(Default)]
enum Outline {
    #[default]
    Discovering,
    Git(Arc<BoundSourceComparison>),
    Legacy,
}

#[derive(Clone)]
pub(super) enum OutlineBinding {
    Git(Arc<BoundSourceComparison>),
    Legacy(Box<BoundComparison>),
}

impl OutlineBinding {
    pub(super) fn source_diff(&self, id: String, cancel: &AtomicBool) -> Result<SourceDiff, Error> {
        match self {
            Self::Git(bound) => bound.source_diff(id, cancel),
            Self::Legacy(bound) => bound.source_diff(id, cancel),
        }
    }
}

#[derive(Default)]
pub struct Changes {
    pub(crate) sidebar: Sidebar,
    pub rows: Vec<Change>,
    pub message: String,
    pub next_offset: Option<usize>,
    pub loaded: bool,
    pub reload_required: bool,
    automatic_selection: bool,
    pending_class: Option<String>,
    outline: Outline,
    context: Option<QueryContext>,
    list: Latest<Tracked<Result<ListAnswer, Error>>>,
    list_clock: Clock,
}

pub(crate) struct Sidebar {
    pub(crate) collapsed: bool,
    pub(crate) selection: Option<String>,
    pub(crate) scroll: Live<f32>,
    closed_branches: HashSet<String>,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self {
            collapsed: false,
            selection: None,
            scroll: Live::new(0.0),
            closed_branches: HashSet::new(),
        }
    }
}

impl Sidebar {
    pub(crate) fn expanded(&self, branch: &str) -> bool {
        !self.closed_branches.contains(branch)
    }

    pub(crate) fn toggle(&mut self, branch: &str) {
        if !self.closed_branches.remove(branch) {
            self.closed_branches.insert(branch.to_owned());
        }
    }
}

impl Changes {
    pub(super) fn set_waker(&self, waker: Waker) {
        self.list.set_waker(waker);
    }

    pub(super) fn pending(&self) -> bool {
        self.list.is_pending()
    }

    pub(super) fn cancel(&mut self) {
        self.list_clock.cancel();
        self.list.cancel();
    }

    pub(super) fn clear(&mut self, workers: &Arc<Workers>) {
        self.cancel();
        self.rows.clear();
        self.sidebar.selection = None;
        self.sidebar.closed_branches.clear();
        self.sidebar.scroll.set(0.0);
        self.loaded = false;
        self.reload_required = false;
        self.outline = Outline::Discovering;
        if let Some(context) = self.context.take() {
            let retired = workers.track(context);
            std::thread::spawn(move || drop(retired));
        }
        self.automatic_selection = false;
        self.pending_class = None;
        self.message.clear();
        self.next_offset = None;
    }

    pub fn show_outline(&self) -> bool {
        self.loaded || self.listing() || !self.message.is_empty()
    }

    pub fn listing(&self) -> bool {
        self.list.is_pending()
    }

    pub(super) fn uses_legacy_graph(&self) -> bool {
        matches!(self.outline, Outline::Legacy)
    }
}

#[derive(Clone)]
pub(crate) enum ChangeAction {
    Class(String),
    Method {
        id: String,
        symbol: Option<String>,
        flow_available: bool,
        anchor: String,
    },
    File {
        id: String,
        anchor: String,
    },
}

impl ChangeAction {
    pub(crate) fn new(change: &Change, anchor: &str) -> Self {
        let id = change.id.clone();
        match change.kind {
            ChangeKind::Class => Self::Class(id),
            ChangeKind::Method => Self::Method {
                id,
                symbol: change.symbol.clone(),
                flow_available: change.flow_available,
                anchor: anchor.into(),
            },
            ChangeKind::File => Self::File {
                id,
                anchor: anchor.into(),
            },
        }
    }
}

impl Workspace {
    pub(crate) fn activate_change(&mut self, action: &ChangeAction) {
        self.changes.automatic_selection = true;
        if self.changes.pending_class.take().is_some() {
            self.changes.message.clear();
        }
        match action {
            ChangeAction::Class(id) => self.choose_change_class(id.clone()),
            ChangeAction::Method {
                symbol: Some(symbol),
                flow_available,
                ..
            } if self.method_flow_ready(*flow_available) => {
                self.choose_change_method(symbol.clone());
            }
            ChangeAction::Method { id, anchor, .. } | ChangeAction::File { id, anchor } => {
                self.choose_change_source(id.clone(), anchor.clone());
            }
        }
    }

    fn method_flow_ready(&self, flow_available: bool) -> bool {
        let has_source = flow_available || !self.git_outline();
        has_source && self.outline_graph_ready()
    }

    pub(super) fn open_change_list(&mut self) {
        let Some(project) = &self.project else {
            return;
        };
        let context = project.context_for(self.identity.captured());
        self.changes.context = Some(context.clone());
        let refs = project.refs.clone();
        let cancel = self.changes.list_clock.restart();
        let work = self.workers.track((context, refs, cancel));
        self.changes.list.request(work, |work| {
            work.map(|(mut context, refs, cancel)| {
                let physical = context
                    .descriptor
                    .project
                    .canonicalize()
                    .map_err(anyhow::Error::from)?;
                context.descriptor.project = physical.clone();
                let result = context.open_source_comparison(refs, &cancel)?;
                Ok(ListAnswer::Opened(physical, result.map(Box::new)))
            })
        });
    }

    pub(super) fn graph_catalog_ready(&mut self) {
        if self.changes.uses_legacy_graph() {
            self.load_change_list(0);
        } else if !self.resolve_pending_class() {
            self.select_first_change();
        }
    }

    pub(super) fn resolve_pending_class(&mut self) -> bool {
        let Some(id) = self.changes.pending_class.take() else {
            return false;
        };
        self.choose_change_class(id);
        true
    }

    pub(super) fn outline_binding(&self) -> Option<OutlineBinding> {
        if self.changes.reload_required {
            return None;
        }
        match &self.changes.outline {
            Outline::Discovering => None,
            Outline::Git(binding) => Some(OutlineBinding::Git(binding.clone())),
            Outline::Legacy => self
                .binding
                .clone()
                .map(|binding| OutlineBinding::Legacy(Box::new(binding))),
        }
    }

    pub(crate) fn git_outline(&self) -> bool {
        matches!(self.changes.outline, Outline::Git(_))
    }

    pub(super) fn is_git_class_edit(&self, id: &str) -> bool {
        self.git_outline()
            && self
                .changes
                .rows
                .iter()
                .any(|row| row.kind == ChangeKind::Class && row.id == id)
    }

    pub(super) fn outline_graph_ready(&self) -> bool {
        let Some(header) = self.header().filter(|_| !self.changes.reload_required) else {
            return false;
        };
        match &self.changes.outline {
            Outline::Git(bound) => {
                bound.graph_comparison_id() == Some(header.comparison_id.as_str())
            }
            Outline::Legacy => true,
            Outline::Discovering => false,
        }
    }

    fn load_change_list(&mut self, offset: usize) {
        let Some(binding) = self.outline_binding() else {
            return;
        };
        let cancel = self.changes.list_clock.restart();
        let work = self.workers.track((binding, cancel));
        self.changes.list.request(work, move |work| {
            work.map(|(binding, cancel)| {
                let page = match binding {
                    OutlineBinding::Git(bound) => Some(bound.changes(offset, &cancel)?),
                    OutlineBinding::Legacy(bound) if bound.changes_supported(&cancel)? => {
                        Some(bound.changes(offset, &cancel)?)
                    }
                    OutlineBinding::Legacy(_) => None,
                };
                Ok(ListAnswer::Page(page))
            })
        });
    }

    pub fn next_changes(&mut self) {
        if let Some(offset) = self.changes.next_offset {
            self.load_change_list(offset);
        }
    }

    pub(super) fn publish_changes(&mut self) -> bool {
        let listed = self.changes.list.take().map(|job| {
            job.map(|result| {
                match result {
                    Ok(ListAnswer::Opened(physical, result)) => {
                        self.accept_source_outline(physical, result)
                    }
                    Ok(ListAnswer::Page(Some(page))) => self.accept_changes(page),
                    Ok(ListAnswer::Page(None)) => {
                        self.changes.message =
                            "This engine does not support the Changes list.".into();
                    }
                    Err(error) if mismatch(&error) => self.change_binding_mismatch(error),
                    Err(error) => self.changes.message = error.to_string(),
                }
                if matches!(self.phase, super::Phase::Loading) && !self.catalog_pending() {
                    self.open_graph_catalog();
                }
            })
        });
        listed.is_some()
    }

    fn accept_source_outline(
        &mut self,
        physical: PathBuf,
        result: Option<Box<(BoundSourceComparison, ChangeList)>>,
    ) {
        if self.identity.resolved(physical) == Installation::Different {
            // Keep the source connection pinned until the discarded answer is dropped.
            drop(result);
            self.close();
            return;
        }
        if let Some(result) = result {
            let (binding, page) = *result;
            self.changes.outline = Outline::Git(Arc::new(binding));
            self.accept_changes(page);
        } else {
            self.changes.outline = Outline::Legacy;
            self.load_change_list(0);
        }
    }

    pub(super) fn change_binding_mismatch(&mut self, error: Error) {
        if self.changes.uses_legacy_graph() {
            self.failed(StringError::Query(error));
        } else {
            self.previews.clear();
            self.changes.clear(&self.workers);
            self.changes.reload_required = true;
            self.changes.message =
                "The source comparison changed. Use Find snapshot on After to see current changes."
                    .into();
        }
    }

    fn accept_changes(&mut self, page: ChangeList) {
        self.changes.loaded = true;
        self.changes.next_offset = page.next_offset;
        self.changes.message.clear();
        self.changes.rows.extend(page.changes);
        self.select_first_change();
    }

    fn select_first_change(&mut self) {
        if !self.can_choose_first_method() {
            return;
        }
        if let Some(symbol) = self
            .changes
            .rows
            .iter()
            .filter(|row| row.flow_available)
            .find_map(|row| row.symbol.clone())
        {
            self.choose_change_method(symbol);
        } else if self.changes.next_offset.is_some() {
            self.next_changes();
        } else if let Some(id) = self
            .changes
            .rows
            .iter()
            .find(|row| row.kind == ChangeKind::Class)
            .map(|row| row.id.clone())
        {
            self.choose_change_class(id);
        }
    }

    fn can_choose_first_method(&self) -> bool {
        self.outline_graph_ready()
            && !self.changes.automatic_selection
            && self.current.is_none()
            && self.class_selection().is_none()
            && !self.detail_pending()
    }

    pub fn choose_change_class(&mut self, id: String) {
        self.changes.automatic_selection = true;
        if matches!(self.phase, super::Phase::Loading) {
            self.previews.close();
            self.changes.pending_class = Some(id);
            self.changes.message = "Waiting for the saved class diagram…".into();
            return;
        }
        if self.outline_graph_ready() {
            self.changes.message = self.select_class(id).err().unwrap_or_default();
        } else {
            self.previews.close();
            self.changes.message =
                "No matching saved graph pair is available for this class diagram.".into();
        }
    }

    pub fn choose_change_method(&mut self, symbol: String) {
        self.changes.automatic_selection = true;
        if let Err(error) = self.select(Selection {
            symbol,
            head_symbol: None,
        }) {
            self.changes.message = error;
        }
    }

    pub fn selected_change_id(&self) -> Option<&str> {
        if let Some(class) = self.class_selection() {
            return Some(class);
        }
        self.function_change_id()
    }

    pub(super) fn function_change_id(&self) -> Option<&str> {
        let selection = &self.current.as_ref()?.selection;
        if selection.head_symbol.is_some() {
            return None;
        }
        self.changes
            .rows
            .iter()
            .find(|row| row.symbol.as_deref() == Some(selection.symbol.as_str()))
            .map(|row| row.id.as_str())
    }
}
