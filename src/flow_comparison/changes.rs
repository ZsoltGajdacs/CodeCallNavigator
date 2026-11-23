//! Change outline and source previews use the same captured endpoints as the charts.
use super::{
    Selection, Side, SideId, Workspace,
    requests::{Clock, StringError},
    work::Tracked,
};
use crate::engine_client::flow_comparison::{
    Change, ChangesPage, Error, Hunk, SourceDiff, SourceStatus,
};
use rune::{Latest, Waker};
use std::{ops::RangeInclusive, sync::Arc};

type ListResult = Result<Option<ChangesPage>, Error>;
type DiffResult = (String, Result<SourceDiff, Error>);

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
    source: Latest<Tracked<DiffResult>>,
    source_clock: Clock,
    method_diff: Option<DiffCache>,
    row_diff: Option<DiffCache>,
    source_id: Option<String>,
    pub preview: Option<Preview>,
    focus_key: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preview {
    pub anchor: String,
    pub target: PreviewTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreviewTarget {
    Step(SideId, usize),
    Row(String),
}

struct DiffCache {
    id: String,
    result: Result<Arc<SourceDiff>, String>,
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
            source: Latest::default(),
            source_clock: Clock::default(),
            method_diff: None,
            row_diff: None,
            source_id: None,
            preview: None,
            focus_key: None,
        }
    }
}

impl Changes {
    pub(super) fn set_waker(&self, waker: Waker) {
        self.list.set_waker(waker.clone());
        self.source.set_waker(waker);
    }

    pub(super) fn pending(&self) -> bool {
        self.list.is_pending() || self.source.is_pending()
    }

    pub(super) fn cancel(&mut self) {
        self.list_clock.cancel();
        self.source_clock.cancel();
        self.list.cancel();
        self.source.cancel();
        self.preview = None;
    }

    pub(super) fn clear(&mut self) {
        self.cancel();
        self.rows.clear();
        self.complete = false;
        self.loaded = false;
        self.automatic_selection = false;
        self.message.clear();
        self.next_offset = None;
        self.method_diff = None;
        self.row_diff = None;
        self.source_id = None;
        self.focus_key = None;
        self.expert = false;
    }

    pub fn show_outline(&self) -> bool {
        let has_content = self.loaded || self.listing() || !self.message.is_empty();
        has_content && !self.expert
    }

    pub fn listing(&self) -> bool {
        self.list.is_pending()
    }

    pub fn diff(&self, id: &str) -> Option<Result<&SourceDiff, &str>> {
        [self.method_diff.as_ref(), self.row_diff.as_ref()]
            .into_iter()
            .flatten()
            .find(|cache| cache.id == id)
            .map(|cache| cache.result.as_deref().map_err(String::as_str))
    }
}

impl Workspace {
    pub(super) fn load_change_list(&mut self, offset: usize) {
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
        let diff = self.changes.source.take().map(|job| {
            job.map(|(id, result)| {
                let result = match result {
                    Err(error) if mismatch(&error) => {
                        self.failed(StringError::Query(error));
                        return;
                    }
                    result => result,
                };
                let for_method = self.selected_change_id() == Some(id.as_str());
                self.changes.source_id = None;
                let cache = Some(DiffCache {
                    id,
                    result: result.map(Arc::new).map_err(|error| error.to_string()),
                });
                if for_method {
                    self.changes.method_diff = cache;
                } else {
                    self.changes.row_diff = cache;
                    self.prefetch_method_hunks();
                }
            })
        });
        listed.is_some() || diff.is_some()
    }

    fn accept_changes(&mut self, page: ChangesPage) {
        self.changes.loaded = true;
        self.changes.complete = page.coverage.entry_point_search_complete;
        self.changes.next_offset = page.next_offset;
        self.changes.message.clear();
        self.changes.rows.extend(page.changes);
        if self.changes.automatic_selection || self.has_method_selection() {
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
        } else {
            self.next_changes();
        }
    }

    fn has_method_selection(&self) -> bool {
        self.current.is_some() || self.detail_pending()
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

    pub fn choose_change_source(&mut self, id: String, anchor: String) {
        self.changes.preview = Some(Preview {
            anchor,
            target: PreviewTarget::Row(id.clone()),
        });
        self.request_change_source(id);
    }

    pub(super) fn prefetch_selected_source(&mut self) {
        self.changes.preview = None;
        self.prefetch_method_hunks();
    }

    fn prefetch_method_hunks(&mut self) {
        if let Some(id) = self.selected_change_id().map(str::to_owned) {
            self.request_change_source(id);
        }
    }

    fn request_change_source(&mut self, id: String) {
        let Some(bound) = self.binding.clone() else {
            return;
        };
        if self.changes.source_id.as_deref() == Some(&id) || self.changes.diff(&id).is_some() {
            return;
        }
        self.changes.source_id = Some(id.clone());
        let cancel = self.changes.source_clock.restart();
        let work = self.workers.track((bound, id, cancel));
        self.changes.source.request(work, |work| {
            work.map(|(bound, id, cancel)| {
                let result = bound.source_diff(id.clone(), &cancel);
                (id, result)
            })
        });
    }

    pub fn selected_change_id(&self) -> Option<&str> {
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

    pub fn show_step_preview(&mut self, side: SideId, index: usize, anchor: String) {
        let Some(id) = self.selected_change_id().map(str::to_owned) else {
            return;
        };
        self.changes.preview = Some(Preview {
            anchor,
            target: PreviewTarget::Step(side, index),
        });
        self.request_change_source(id);
    }

    pub fn end_step_hover(&mut self, anchor: &str) {
        if self.changes.focus_key.as_deref() == Some(anchor) {
            return;
        }
        if self
            .changes
            .preview
            .as_ref()
            .is_some_and(|preview| preview.anchor == anchor)
        {
            self.changes.preview = None;
        }
    }

    pub fn sync_change_focus(&mut self, focused: Option<&str>) -> bool {
        let focus = focused.and_then(step_focus);
        let key = focus.map(|(key, _, _)| key);
        if self.changes.focus_key.as_deref() == key {
            return false;
        }
        if let Some(previous) = self.changes.focus_key.take() {
            self.end_step_hover(&previous);
        }
        if let Some((key, side, index)) = focus {
            self.changes.focus_key = Some(key.into());
            self.show_step_preview(side, index, key.into());
        }
        true
    }

    pub fn step_hunk<'a>(
        &self,
        diff: &'a SourceDiff,
        side: SideId,
        index: usize,
    ) -> Option<&'a Hunk> {
        let line_range = self.step_lines(side, index)?;
        diff.hunks.iter().find(|hunk| {
            hunk.changed_at(side == SideId::Before, &line_range)
                .is_some()
        })
    }

    pub fn step_change(&self, side: SideId, index: usize) -> Option<StepChange> {
        let id = self.selected_change_id()?;
        if self.current.as_ref()?.call_changes.at(side, index) {
            return Some(StepChange::Modified);
        }
        let diff = self.changes.diff(id)?.ok()?;
        if diff.status != SourceStatus::Ready {
            return None;
        }
        let range = self.step_lines(side, index)?;
        let changed_both = diff
            .hunks
            .iter()
            .find_map(|hunk| hunk.changed_at(side == SideId::Before, &range))?;
        Some(if changed_both {
            StepChange::Modified
        } else if side == SideId::Before {
            StepChange::Removed
        } else {
            StepChange::Added
        })
    }

    fn step_lines(&self, side: SideId, index: usize) -> Option<RangeInclusive<usize>> {
        let Side::Ready(side) = self.current.as_ref()?.side(side) else {
            return None;
        };
        let ready = side.session.ready.as_ref()?;
        let span = ready.chart.steps.get(index)?.span;
        let source = &ready.source;
        if span.start >= span.end || span.end > source.text.len() {
            return None;
        }
        let line = |offset| {
            source
                .lines
                .partition_point(|line| line.range.start <= offset)
        };
        Some(line(span.start)..=line(span.end - 1))
    }
}

#[derive(Clone, Copy)]
pub enum StepChange {
    Added,
    Removed,
    Modified,
}

fn step_focus(key: &str) -> Option<(&str, SideId, usize)> {
    let (side, index) = if let Some(index) = key.strip_prefix("comparison-before-step-") {
        (SideId::Before, index)
    } else {
        (SideId::After, key.strip_prefix("comparison-after-step-")?)
    };
    Some((key, side, index.parse().ok()?))
}

fn mismatch(error: &Error) -> bool {
    matches!(error, Error::Cli(error) if error.code == "comparison_mismatch")
}
