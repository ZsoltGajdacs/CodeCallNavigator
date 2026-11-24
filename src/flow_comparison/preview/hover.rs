//! Connect Compare canvas triggers to the shared source-popup interaction.
use super::{Panel, PreviewTarget, Previews, SideId, Workspace};
use crate::source_popup::Owner;
use rune::Tx;

pub(super) struct CanvasPreview {
    anchor: String,
    target: CanvasTarget,
}

enum CanvasTarget {
    Step(SideId, usize),
    Class(SideId, String),
}

impl Previews {
    fn queue_canvas_hover(&mut self, anchor: &str, tx: &mut Tx) {
        if let Some(target) = canvas_target(anchor) {
            let preview = CanvasPreview {
                anchor: anchor.into(),
                target,
            };
            self.hover = Some(self.interaction.begin_hover(preview, tx));
        }
    }

    fn leave_canvas_trigger(&mut self, anchor: &str, active: bool, tx: &mut Tx) {
        let pending = self
            .hover
            .as_ref()
            .is_some_and(|hover| hover.target.anchor == anchor);
        if active || pending {
            self.clear_hover();
            self.interaction.schedule_close(tx);
        }
    }

    fn dismiss_hover_if_expired(&mut self, tx: &mut Tx) -> bool {
        let Some(preview) = &self.active else {
            return false;
        };
        if matches!(preview.target, PreviewTarget::Row(_)) {
            return false;
        }
        let focused =
            self.focus_key.as_deref() == Some(&preview.anchor) || Owner::Compare.focused(tx);
        if self.interaction.should_close(focused, tx) {
            self.dismiss();
            return true;
        }
        false
    }
}

impl Workspace {
    pub fn hover_canvas_source(&mut self, anchor: &str, inside: bool, tx: &mut Tx) {
        let previews = &mut self.previews;
        let active = previews
            .active
            .as_ref()
            .is_some_and(|preview| preview.anchor == anchor);
        if active {
            previews.interaction.trigger_inside = inside;
        }
        if !previews.interaction.can_hover() {
            return;
        }
        match (inside, active) {
            (true, false) => previews.queue_canvas_hover(anchor, tx),
            (true, true) => {}
            (false, _) => previews.leave_canvas_trigger(anchor, active, tx),
        }
    }

    pub fn poll_hover_preview(&mut self, time: f64, tx: &mut Tx) -> bool {
        let previews = &mut self.previews;
        previews.interaction.now = time;
        if !self.controls.visible || self.controls.panel != Panel::Flows {
            return false;
        }
        if previews.hover.is_none() {
            return previews.dismiss_hover_if_expired(tx);
        }
        if let Some(hover) = previews.hover.take_if(|hover| hover.ready(time, tx)) {
            let preview = hover.target;
            self.open_canvas_preview(preview.target, preview.anchor, true);
            return true;
        }
        false
    }

    pub fn sync_change_focus(&mut self, focused: Option<&str>) -> bool {
        if !self.controls.visible || self.controls.panel != Panel::Flows {
            return false;
        }
        if focused.is_some_and(|key| Owner::Compare.owns_focus(key)) {
            return false;
        }
        let focused =
            focused.filter(|_| self.changes.show_outline() || self.class_selection().is_some());
        let focus = focused.and_then(canvas_target);
        let key = focus.as_ref().and(focused);
        if self.previews.focus_key.as_deref() == key {
            return false;
        }
        self.previews.focus_key = key.map(str::to_owned);
        if let Some((key, target)) = key.zip(focus) {
            self.open_canvas_preview(target, key.into(), false);
        }
        true
    }

    pub fn focus_canvas_source(&mut self, anchor: &str) {
        if let Some(target) = canvas_target(anchor) {
            self.open_canvas_preview(target, anchor.into(), false);
        }
    }

    fn open_canvas_preview(&mut self, target: CanvasTarget, anchor: String, hovered: bool) {
        if self.previews.interaction.pinned
            || self
                .previews
                .active
                .as_ref()
                .is_some_and(|preview| preview.anchor == anchor)
        {
            return;
        }
        self.previews.clear_hover();
        match target {
            CanvasTarget::Step(side, index) => self.show_step_preview(side, index, anchor.clone()),
            CanvasTarget::Class(side, id) => self.show_class_preview(side, id, anchor.clone()),
        }
        if self
            .previews
            .active
            .as_ref()
            .is_some_and(|preview| preview.anchor == anchor)
        {
            self.previews.interaction.trigger_inside = hovered;
            self.previews.interaction.close_at = None;
        }
    }
}

fn canvas_target(key: &str) -> Option<CanvasTarget> {
    let (side, key) = if let Some(key) = key.strip_prefix("comparison-before-") {
        (SideId::Before, key)
    } else {
        (SideId::After, key.strip_prefix("comparison-after-")?)
    };
    if let Some(index) = key.strip_prefix("step-") {
        return Some(CanvasTarget::Step(side, index.parse().ok()?));
    }
    Some(CanvasTarget::Class(
        side,
        key.strip_prefix("class-")?.into(),
    ))
}
