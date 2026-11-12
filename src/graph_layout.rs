//! One completion path for asynchronous and deterministic graph layouts.
use crate::{
    app::State,
    projection::{FocusTarget, GraphView, Grouping},
};

impl State {
    pub fn complete_graph_layout(&mut self, view: GraphView) {
        if self.focus_was_removed(&view) {
            self.nav.overview();
            self.auto_fit = true;
            self.visual_selection = None;
            self.graph_interaction.center_after_layout = false;
            self.changed();
            return;
        }
        self.replace_graph_view(view);
        self.reconcile_visual_selection();
        self.visible_revision = Some((self.epoch, self.revision));
        self.nav.page = self.nav.page.min(self.graph_view.pages.saturating_sub(1));
        if self.auto_fit {
            self.fit_visible_graph();
            self.auto_fit = false;
        }
        if self.graph_interaction.center_after_layout {
            if matches!(self.nav.focus_target, Some(FocusTarget::FileClasses(_))) {
                self.fit_graph_focus();
            } else {
                self.center_graph_focus();
            }
            self.graph_interaction.center_after_layout = false;
        }
    }

    pub fn fit_visible_graph(&mut self) {
        self.nav.camera.fit(self.graph_view.size, self.viewport);
    }

    fn focus_was_removed(&self, view: &GraphView) -> bool {
        if self.nav.show_path {
            return false;
        }
        // Missing assignments may mean setup, a reload, or a configuration error.
        // Resolve the target only once the layer membership is available.
        match (self.nav.grouping, self.architecture.as_ref()) {
            (Grouping::Layer, None) => false,
            _ => self.has_graph_focus() && !view.focus_valid,
        }
    }

    pub fn has_graph_focus(&self) -> bool {
        self.nav.focus_target.is_some() || self.nav.focus.is_some()
    }

    pub fn resize_graph_viewport(&mut self, viewport: [f32; 2]) {
        let resized = self.viewport != viewport;
        let centered = self.graph_focus_center().is_some_and(|center| {
            let screen = self.nav.camera.screen(center);
            (screen[0] - self.viewport[0] / 2.0).abs() < 0.1
                && (screen[1] - self.viewport[1] / 2.0).abs() < 0.1
        });
        self.viewport = viewport;
        if resized && self.file_focus_is_current() {
            self.fit_graph_focus();
            return;
        }
        if centered && self.view_is_current() {
            self.center_graph_focus();
        }
    }

    pub(crate) fn set_graph_filter_height(&mut self, height: f32) {
        if self.graph_interaction.filter_height == height {
            return;
        }
        self.graph_interaction.filter_height = height;
        if self.file_focus_is_current() {
            self.fit_graph_focus();
        }
    }

    fn file_focus_is_current(&self) -> bool {
        matches!(self.nav.focus_target, Some(FocusTarget::FileClasses(_))) && self.view_is_current()
    }

    fn center_graph_focus(&mut self) {
        if let Some(center) = self.graph_focus_center() {
            let zoom = self.nav.camera.zoom;
            self.nav.camera.pan = [
                self.viewport[0] / 2.0 - center[0] * zoom,
                self.viewport[1] / 2.0 - center[1] * zoom,
            ];
            self.dirty_session = true;
        }
    }

    fn fit_graph_focus(&mut self) {
        if let Some(bounds) = focus_bounds(&self.graph_view) {
            let top = self.graph_interaction.filter_height.min(self.viewport[1]);
            let available = [self.viewport[0], (self.viewport[1] - top).max(1.0)];
            self.nav
                .camera
                .fit([bounds[2] - bounds[0], bounds[3] - bounds[1]], available);
            self.nav.camera.pan[0] -= bounds[0] * self.nav.camera.zoom;
            self.nav.camera.pan[1] -= bounds[1] * self.nav.camera.zoom;
            self.nav.camera.pan[1] += top;
            self.dirty_session = true;
        }
    }

    fn graph_focus_center(&self) -> Option<[f32; 2]> {
        focus_bounds(&self.graph_view).map(|b| [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0])
    }
}

/// Shared world-space bounds for ordinary centering and file-class framing.
fn focus_bounds(view: &GraphView) -> Option<[f32; 4]> {
    let mut nodes = view
        .nodes
        .iter()
        .filter(|node| view.focus_keys.contains(&node.key));
    let first = nodes.next()?;
    let mut bounds = [
        first.position[0],
        first.position[1],
        first.position[0] + first.size[0],
        first.position[1] + first.size[1],
    ];
    for node in nodes {
        bounds[0] = bounds[0].min(node.position[0]);
        bounds[1] = bounds[1].min(node.position[1]);
        bounds[2] = bounds[2].max(node.position[0] + node.size[0]);
        bounds[3] = bounds[3].max(node.position[1] + node.size[1]);
    }
    Some(bounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::ViewNode;

    #[test]
    fn bounds_include_only_focus_nodes_and_handle_negative_positions() {
        let view = GraphView {
            focus_keys: vec!["a".into(), "b".into()],
            nodes: vec![
                ViewNode {
                    key: "a".into(),
                    position: [-20.0, 30.0],
                    size: [10.0, 20.0],
                    ..Default::default()
                },
                ViewNode {
                    key: "b".into(),
                    position: [50.0, -10.0],
                    size: [40.0, 60.0],
                    ..Default::default()
                },
                ViewNode {
                    key: "unrelated".into(),
                    position: [900.0, 900.0],
                    size: [100.0, 100.0],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert_eq!(focus_bounds(&view), Some([-20.0, -10.0, 90.0, 50.0]));
        assert_eq!(focus_bounds(&GraphView::default()), None);
    }
}
