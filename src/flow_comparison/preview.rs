//! Source overlays and their bound asynchronous requests.
mod class;

use super::{
    Panel, Side, SideId, Workspace, changes::OutlineBinding, requests::Clock, work::Tracked,
};
use crate::engine_client::flow_comparison::{Error, Hunk, SourceDiff, SourceStatus};
use rune::{Latest, Waker};
use std::ops::RangeInclusive;

type DiffResult = (String, Result<SourceDiff, Error>);

#[derive(Default)]
pub struct Previews {
    class: class::ClassPreview,
    source: Latest<Tracked<DiffResult>>,
    source_clock: Clock,
    method_diff: Option<DiffCache>,
    row_diff: Option<DiffCache>,
    source_id: Option<String>,
    pub active: Option<Preview>,
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
    Class(SideId, String),
    ClassEdit(String),
}

struct DiffCache {
    id: String,
    result: Result<SourceDiff, String>,
}

impl Previews {
    pub(super) fn set_waker(&self, waker: Waker) {
        self.source.set_waker(waker.clone());
        self.class.set_waker(waker);
    }

    pub(super) fn pending(&self) -> bool {
        self.source.is_pending() || self.class.pending()
    }

    pub fn dismiss(&mut self) {
        self.active = None;
        self.class.cancel_request();
    }

    pub fn close(&mut self) {
        self.active = None;
        self.focus_key = None;
        self.class.clear();
        self.source_clock.cancel();
        self.source.cancel();
        self.source_id = None;
    }

    pub(super) fn clear(&mut self) {
        self.close();
        self.method_diff = None;
        self.row_diff = None;
    }

    pub fn diff(&self, id: &str) -> Option<Result<&SourceDiff, &str>> {
        [self.method_diff.as_ref(), self.row_diff.as_ref()]
            .into_iter()
            .flatten()
            .find(|cache| cache.id == id)
            .map(|cache| cache.result.as_ref().map_err(String::as_str))
    }

    pub fn preview_diff(&self, id: &str) -> Option<Result<&SourceDiff, &str>> {
        if self.source_id.as_deref() == Some(id) {
            return None;
        }
        self.diff(id)
    }

    fn needs_source(&self, id: &str, revalidate: bool) -> bool {
        if self.source_id.as_deref() == Some(id) {
            return false;
        }
        revalidate || self.diff(id).is_none()
    }
}

impl Workspace {
    pub(super) fn publish_previews(&mut self) -> bool {
        let diff = self.previews.source.take().map(|job| {
            job.map(|(id, result)| {
                let result = match result {
                    Err(error) if mismatch(&error) => {
                        self.change_binding_mismatch(error);
                        return;
                    }
                    result => result,
                };
                let for_method = self.function_change_id() == Some(id.as_str());
                self.previews.source_id = None;
                let cache = Some(DiffCache {
                    id,
                    result: result.map_err(|error| error.to_string()),
                });
                if for_method {
                    self.previews.method_diff = cache;
                } else {
                    self.previews.row_diff = cache;
                    self.prefetch_method_hunks();
                }
            })
        });
        diff.is_some() | self.publish_class_preview()
    }

    pub fn choose_change_source(&mut self, id: String, anchor: String) {
        self.previews.dismiss();
        self.previews.active = Some(Preview {
            anchor,
            target: PreviewTarget::Row(id.clone()),
        });
        self.request_change_source(id, true);
    }

    pub(super) fn prefetch_selected_source(&mut self) {
        self.previews.close();
        self.prefetch_method_hunks();
    }

    fn prefetch_method_hunks(&mut self) {
        if let Some(id) = self.function_change_id().map(str::to_owned) {
            self.request_change_source(id, false);
        }
    }

    fn request_change_source(&mut self, id: String, opening_preview: bool) {
        let Some(bound) = self.outline_binding() else {
            return;
        };
        if matches!(&bound, OutlineBinding::Legacy(_)) && self.worktree_refresh.pending() {
            return;
        }
        let revalidate = opening_preview
            && matches!(&bound, OutlineBinding::Git(source) if source.is_worktree());
        if !self.previews.needs_source(&id, revalidate) {
            return;
        }
        self.previews.source_id = Some(id.clone());
        let cancel = self.previews.source_clock.restart();
        let work = self.workers.track((bound, id, cancel));
        self.previews.source.request(work, |work| {
            work.map(|(bound, id, cancel)| {
                let result = bound.source_diff(id.clone(), &cancel);
                (id, result)
            })
        });
    }

    pub fn show_step_preview(&mut self, side: SideId, index: usize, anchor: String) {
        let Some(id) = self.function_change_id().map(str::to_owned) else {
            return;
        };
        self.previews.active = Some(Preview {
            anchor,
            target: PreviewTarget::Step(side, index),
        });
        self.request_change_source(id, true);
    }

    pub fn end_step_hover(&mut self, anchor: &str) {
        if self.previews.focus_key.as_deref() == Some(anchor) {
            return;
        }
        if self
            .previews
            .active
            .as_ref()
            .is_some_and(|preview| preview.anchor == anchor)
        {
            self.previews.dismiss();
        }
    }

    pub fn sync_change_focus(&mut self, focused: Option<&str>) -> bool {
        if !self.controls.visible || self.controls.panel != Panel::Flows {
            return false;
        }
        let focused =
            focused.filter(|_| self.changes.show_outline() || self.class_selection().is_some());
        let focus = focused.and_then(canvas_focus);
        let key = focus.as_ref().and(focused);
        if self.previews.focus_key.as_deref() == key {
            return false;
        }
        if let Some(previous) = self.previews.focus_key.take() {
            self.end_step_hover(&previous);
        }
        if let Some((key, target)) = key.zip(focus) {
            self.previews.focus_key = Some(key.into());
            match target {
                PreviewTarget::Step(side, index) => self.show_step_preview(side, index, key.into()),
                PreviewTarget::Class(side, id) => self.show_class_preview(side, id, key.into()),
                PreviewTarget::Row(_) | PreviewTarget::ClassEdit(_) => {
                    unreachable!("canvas focus resolves through the active selection")
                }
            }
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
        let id = self.function_change_id()?;
        if self.current.as_ref()?.call_changes.at(side, index) {
            return Some(StepChange::Modified);
        }
        let diff = self.previews.diff(id)?.ok()?;
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

fn canvas_focus(key: &str) -> Option<PreviewTarget> {
    let (side, key) = if let Some(key) = key.strip_prefix("comparison-before-") {
        (SideId::Before, key)
    } else {
        (SideId::After, key.strip_prefix("comparison-after-")?)
    };
    if let Some(index) = key.strip_prefix("step-") {
        return Some(PreviewTarget::Step(side, index.parse().ok()?));
    }
    Some(PreviewTarget::Class(
        side,
        key.strip_prefix("class-")?.into(),
    ))
}

pub(super) fn mismatch(error: &Error) -> bool {
    matches!(error, Error::Cli(error) if error.code == "comparison_mismatch")
}
