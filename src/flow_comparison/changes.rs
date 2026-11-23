//! Change outline and source previews use the same captured endpoints as the charts.
use super::{
    Selection, Workspace,
    preview::mismatch,
    requests::{Clock, StringError},
    work::Tracked,
};
use crate::engine_client::flow_comparison::{Change, ChangeKind, ChangesPage, Error};
use rune::{Latest, Waker};

type ListResult = Result<Option<ChangesPage>, Error>;

pub struct Changes {
    pub outline_height: f32,
    pub expert: bool,
    pub rows: Vec<Change>,
    pub complete: bool,
    pub message: String,
    pub next_offset: Option<usize>,
    pub loaded: bool,
    automatic_selection: bool,
    list: Latest<Tracked<ListResult>>,
    list_clock: Clock,
}

impl Default for Changes {
    fn default() -> Self {
        Self {
            outline_height: 220.0,
            expert: false,
            rows: Vec::new(),
            complete: false,
            message: String::new(),
            next_offset: None,
            loaded: false,
            automatic_selection: false,
            list: Latest::default(),
            list_clock: Clock::default(),
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

    pub(super) fn clear(&mut self) {
        self.cancel();
        self.rows.clear();
        self.complete = false;
        self.loaded = false;
        self.automatic_selection = false;
        self.message.clear();
        self.next_offset = None;
        self.expert = false;
    }

    pub fn show_outline(&self) -> bool {
        let has_content = self.loaded || self.listing() || !self.message.is_empty();
        has_content && !self.expert
    }

    pub fn listing(&self) -> bool {
        self.list.is_pending()
    }
}

impl Workspace {
    pub(super) fn load_change_list(&mut self, offset: usize) {
        if self.worktree_refresh.pending() {
            return;
        }
        let Some(bound) = self.binding.clone() else {
            return;
        };
        let cancel = self.changes.list_clock.restart();
        let work = self.workers.track((bound, cancel));
        self.changes.list.request(work, move |work| {
            work.map(|(bound, cancel)| {
                if !bound.changes_supported(&cancel)? {
                    return Ok(None);
                }
                bound.changes(offset, &cancel).map(Some)
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
            job.map(|result| match result {
                Ok(Some(page)) => self.accept_changes(page),
                Ok(None) => self.controls.catalog_open = true,
                Err(error) if mismatch(&error) => self.failed(StringError::Query(error)),
                Err(error) => self.changes.message = error.to_string(),
            })
        });
        listed.is_some()
    }

    fn accept_changes(&mut self, page: ChangesPage) {
        self.changes.loaded = true;
        self.changes.complete = page.coverage.entry_point_search_complete;
        self.changes.next_offset = page.next_offset;
        self.changes.message.clear();
        self.changes.rows.extend(page.changes);
        if !self.can_choose_first_method() {
            return;
        }
        if let Some(symbol) = self
            .changes
            .rows
            .iter()
            .find(|row| row.flow_available)
            .and_then(|row| row.symbol.clone())
        {
            self.choose_change_method(symbol);
        } else if self.changes.next_offset.is_some() {
            self.next_changes();
        } else if let Some(id) = self
            .changes
            .rows
            .iter()
            .find_map(|row| (row.kind == ChangeKind::Class).then(|| row.id.clone()))
        {
            self.choose_change_class(id);
        }
    }

    fn can_choose_first_method(&self) -> bool {
        !self.changes.expert
            && !self.changes.automatic_selection
            && self.current.is_none()
            && self.class_selection().is_none()
            && !self.detail_pending()
    }

    pub fn choose_change_class(&mut self, id: String) {
        self.changes.automatic_selection = true;
        self.changes.expert = false;
        if let Err(error) = self.select_class(id) {
            self.changes.message = error;
        }
    }

    pub fn choose_change_method(&mut self, symbol: String) {
        self.changes.automatic_selection = true;
        self.changes.expert = false;
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
