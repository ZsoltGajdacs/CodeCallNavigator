use super::{Contents, Preview};
use crate::{
    app::State,
    projection::{GroupKind, Grouping, ViewNode},
    source,
};
use rune::Tx;
use std::sync::Arc;

pub struct SourceOutput {
    pub serial: u64,
    pub result: Result<source::SourceFile, String>,
}

fn eligible(node: &ViewNode, grouping: Grouping) -> bool {
    if node.member.is_some() || node.group_kind == Some(GroupKind::Class) {
        return true;
    }
    node.group_kind.is_some()
        && matches!(
            grouping,
            Grouping::Folder | Grouping::Namespace | Grouping::Layer
        )
}

impl State {
    /// Open the inspected method through the same cancellable source lifecycle as hover.
    pub(crate) fn open_selected_source(&mut self) {
        let Some(member) = self.selected() else {
            return;
        };
        let node = self
            .graph_view
            .nodes
            .iter()
            .find(|node| {
                node.member == Some(member) || node.methods.iter().any(|row| row.member == member)
            })
            .cloned();
        if let Some(mut node) = node {
            node.member = Some(member);
            self.close_source();
            open_hover(self, node);
            // Pin the same displayed anchor as hover before the first popup layout.
            self.source_popup.interaction.rect = self.source_popup.bounds(self);
            self.pin_source();
        }
    }

    pub fn hover_source(&mut self, node: ViewNode, inside: bool, tx: &mut Tx) {
        let popup = &mut self.source_popup;
        if popup
            .active
            .as_ref()
            .is_some_and(|p| p.node.key == node.key)
        {
            popup.interaction.trigger_inside = inside;
        }
        if !popup.interaction.can_hover() {
            return;
        }
        if inside && eligible(&node, self.nav.grouping) {
            popup.hover = Some(popup.interaction.begin_hover(node, tx));
        } else if !inside
            && popup
                .hover
                .as_ref()
                .is_some_and(|h| h.target.key == node.key)
        {
            popup.hover = None;
            popup.interaction.schedule_close(tx);
        }
    }

    pub fn close_source(&mut self) {
        let popup = &mut self.source_popup;
        popup
            .cancelled
            .store(true, std::sync::atomic::Ordering::Release);
        popup.active = None;
        popup.hover = None;
        popup.interaction.reset();
        popup.serial += 1;
        self.jobs.source.cancel();
    }

    pub fn pin_source(&mut self) {
        if self.source_popup.active.is_some() {
            self.source_popup.interaction.toggle_pin();
            self.source_popup.hover = None;
        }
    }

    pub fn open_source_file(&mut self, path: String, line: u32) {
        self.source_popup
            .cancelled
            .store(true, std::sync::atomic::Ordering::Release);
        self.source_popup.cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel = self.source_popup.cancelled.clone();
        self.source_popup.serial += 1;
        let serial = self.source_popup.serial;
        if let Some(preview) = &mut self.source_popup.active {
            preview.path = Some(path.clone());
            preview.requested_line = line;
            preview.location_warning = None;
            preview.contents = Contents::Loading;
            let queries = self.graph_queries();
            self.jobs.source.request(path, move |path| SourceOutput {
                serial,
                result: queries.and_then(|queries| {
                    queries
                        .source(&path, &cancel)
                        .map_err(|error| format!("{error:#}"))
                }),
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
}

fn open_hover(state: &mut State, node: ViewNode) {
    state.jobs.source.cancel();
    state.source_popup.serial += 1;
    state.source_popup.interaction.inside = false;
    state.source_popup.interaction.trigger_inside = true;
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

pub fn poll(state: &mut State, time: f64, tx: &mut Tx) -> bool {
    state.source_popup.interaction.now = time;
    let loaded = poll_loaded(state, tx);
    let popup = &mut state.source_popup;
    if !popup.interaction.can_hover() {
        return loaded;
    }
    if let Some(hover) = &popup.hover {
        let same = popup
            .active
            .as_ref()
            .is_some_and(|p| p.node.key == hover.target.key);
        if !same && hover.ready(time, tx) {
            let node = hover.target.clone();
            open_hover(state, node);
            return true;
        }
    } else if popup.active.is_some()
        && popup
            .interaction
            .should_close(super::panel::Owner::Graph.focused(tx), tx)
    {
        state.close_source();
        return true;
    }
    loaded
}
