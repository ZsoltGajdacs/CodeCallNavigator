use super::{Contents, Hover, Preview};
use crate::{
    app::State,
    projection::{Grouping, ViewNode},
    source,
};
use rune::{Drag, Phase, Tx};
use std::{sync::Arc, time::Duration};

pub struct SourceOutput {
    pub serial: u64,
    pub result: Result<source::SourceFile, String>,
}

fn eligible(node: &ViewNode, grouping: Grouping) -> bool {
    if node.member.is_some() {
        return true;
    }
    node.group_kind.is_some() && matches!(grouping, Grouping::Folder | Grouping::Namespace)
}

impl State {
    pub fn hover_source(&mut self, node: ViewNode, inside: bool, tx: &mut Tx) {
        if self
            .source_popup
            .active
            .as_ref()
            .is_some_and(|p| p.node.key == node.key)
        {
            self.source_popup.trigger_inside = inside;
        }
        if self.source_popup.active.as_ref().is_some_and(|p| p.pinned) {
            return;
        }
        if inside && self.source_popup.inside {
            return;
        }
        if inside {
            let eligible = eligible(&node, self.nav.grouping);
            if eligible {
                self.source_popup.hover = Some(Hover {
                    node,
                    deadline: Some(self.source_popup.now + 0.4),
                });
                self.source_popup.close_at = None;
                tx.wake_after(Duration::from_millis(400));
            }
        } else if self
            .source_popup
            .hover
            .as_ref()
            .is_some_and(|h| h.node.key == node.key)
        {
            self.source_popup.hover = None;
            self.source_popup.close_at = Some(self.source_popup.now + 0.3);
            tx.wake_after(Duration::from_millis(300));
        }
    }

    pub fn close_source(&mut self) {
        self.source_popup.active = None;
        self.source_popup.hover = None;
        self.source_popup.inside = false;
        self.source_popup.trigger_inside = false;
        self.source_popup.close_at = None;
        self.source_popup.resizing = false;
        self.source_popup.serial += 1;
        self.jobs.source.cancel();
    }

    pub fn pin_source(&mut self) {
        if let Some(preview) = &mut self.source_popup.active {
            preview.pinned = !preview.pinned;
            self.source_popup.hover = None;
            self.source_popup.close_at = None;
        }
    }

    pub fn open_source_file(&mut self, path: String, line: u32) {
        self.source_popup.serial += 1;
        let serial = self.source_popup.serial;
        if let Some(preview) = &mut self.source_popup.active {
            preview.path = Some(path.clone());
            preview.requested_line = line;
            preview.location_warning = None;
            preview.contents = Contents::Loading;
            self.jobs.source.request(path, move |path| SourceOutput {
                serial,
                result: source::load(&path),
            });
        }
    }

    pub fn source_files_back(&mut self) {
        self.jobs.source.cancel();
        self.source_popup.serial += 1;
        if let Some(preview) = &mut self.source_popup.active {
            preview.contents = Contents::Files;
            preview.path = None;
            preview.location_warning = None;
        }
    }

    pub fn resize_source(&mut self, drag: &Drag) {
        if drag.phase == Phase::Down {
            self.source_popup.resize_saved = self.source_popup.size;
            self.source_popup.resize_start = [self.source_popup.rect[2], self.source_popup.rect[3]];
            self.source_popup.resizing = true;
        }
        let start = self.source_popup.resize_start;
        let size = if drag.cancelled() {
            self.source_popup.resize_saved
        } else {
            [start[0] + drag.dx, start[1] + drag.dy]
        };
        self.source_popup.size = self.source_popup.constrained_size(size);
        if drag.phase == Phase::Up {
            self.source_popup.resizing = false;
        }
    }
}

fn open_hover(state: &mut State, node: ViewNode) {
    state.jobs.source.cancel();
    state.source_popup.serial += 1;
    state.source_popup.inside = false;
    state.source_popup.trigger_inside = true;
    let method = node
        .member
        .and_then(|member| state.graph.as_ref()?.data.nodes.get(member as usize));
    let location = method.map(|n| (n.file.clone(), n.line));
    state.source_popup.active = Some(Preview {
        node,
        contents: Contents::Files,
        path: None,
        requested_line: 0,
        target: 0,
        location_warning: None,
        pinned: false,
    });
    if let Some((path, line)) = location {
        state.open_source_file(path, line);
    }
}

fn poll_loaded(state: &mut State, tx: &mut Tx) -> bool {
    let Some(output) = state.jobs.source.take() else {
        return false;
    };
    if output.serial != state.source_popup.serial {
        return false;
    }
    let Some(preview) = &mut state.source_popup.active else {
        return false;
    };
    preview.contents = match output.result {
        Ok(file) => {
            let (target, warning) = source::target_line(preview.requested_line, file.lines.len());
            preview.target = target;
            preview.location_warning = warning;
            tx.scroll_center_in_instant("source-scroll", format!("source-line-{target}"));
            Contents::Source(Arc::new(file))
        }
        Err(error) => Contents::Error(error),
    };
    true
}

fn poll_hover(state: &mut State, time: f64, tx: &mut Tx) -> bool {
    let hover = state.source_popup.hover.as_mut().unwrap();
    let deadline = *hover.deadline.get_or_insert(time + 0.4);
    let same = state
        .source_popup
        .active
        .as_ref()
        .is_some_and(|p| p.node.key == hover.node.key);
    state.source_popup.close_at = None;
    if same {
        return false;
    }
    if time < deadline {
        tx.wake_after(Duration::from_secs_f64(deadline - time));
        return false;
    }
    let node = hover.node.clone();
    open_hover(state, node);
    true
}

fn poll_close(state: &mut State, time: f64, tx: &mut Tx) -> bool {
    let popup = &mut state.source_popup;
    let keep = popup.active.is_none() || popup.inside || popup.trigger_inside || popup.resizing;
    if keep {
        popup.close_at = None;
        return false;
    }
    let deadline = *popup.close_at.get_or_insert(time + 0.3);
    if time < deadline {
        tx.wake_after(Duration::from_secs_f64(deadline - time));
        return false;
    }
    state.close_source();
    true
}

pub fn poll(state: &mut State, time: f64, tx: &mut Tx) -> bool {
    state.source_popup.now = time;
    let loaded = poll_loaded(state, tx);
    if state.source_popup.active.as_ref().is_some_and(|p| p.pinned) {
        return loaded;
    }
    if state.source_popup.inside {
        state.source_popup.close_at = None;
        return loaded;
    }
    let lifecycle = if state.source_popup.hover.is_some() {
        poll_hover(state, time, tx)
    } else {
        poll_close(state, time, tx)
    };
    loaded | lifecycle
}
