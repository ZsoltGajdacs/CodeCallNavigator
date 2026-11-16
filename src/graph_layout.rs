//! One completion path for asynchronous and deterministic graph layouts.
use crate::{
    app::State,
    projection::{FocusTarget, GraphView, Grouping},
};

impl State {
    pub fn complete_graph_layout(&mut self, view: GraphView) {
        if self.focus_was_removed(&view) {
            self.nav.overview();
            self.auto_fit = !self.preserve_adoption_camera;
            self.visual_selection = None;
            self.graph_interaction.center_after_layout = false;
            self.changed();
            return;
        }
        if self.preserve_adoption_camera {
            self.auto_fit = false;
            self.graph_interaction.center_after_layout = false;
        }
        let before = crate::graph_motion::Snapshot::capture(&self.graph_view, &self.nav.camera);
        let mut motion = std::mem::take(&mut self.graph_motion);
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
        if !self.deterministic && !self.scene {
            motion.begin(before, &mut self.graph_view, &mut self.nav.camera);
        }
        self.graph_motion = motion;
    }

    pub fn fit_visible_graph(&mut self) {
        self.nav
            .camera
            .fit(self.graph_view.size, self.graph_safe_viewport());
    }

    pub(crate) fn graph_safe_viewport(&self) -> [f32; 2] {
        [
            self.viewport[0],
            (self.viewport[1] - self.ui.canvas_bottom_inset).max(1.0),
        ]
    }

    /// Controls are pinned over the canvas; reserve their measured bounds only for framing.
    /// World transforms, pointer coordinates and source-popup origins remain unchanged.
    pub(crate) fn set_canvas_controls_top(&mut self, top: f32) {
        if !top.is_finite() {
            return;
        }
        let inset = (self.source_popup.canvas[1] + self.viewport[1] - top)
            .clamp(0.0, self.viewport[1].max(0.0));
        if (inset - self.ui.canvas_bottom_inset).abs() < 0.5 {
            return;
        }
        let was_centered = self.focus_is_centered();
        let was_fitted = self.camera_is_fitted();
        self.ui.canvas_bottom_inset = inset;
        if !was_fitted && !was_centered {
            return;
        }
        if !self.view_is_current() {
            return;
        }
        if self.preserve_adoption_camera {
            return;
        }
        self.graph_motion.interrupt_camera();
        if was_centered {
            self.center_graph_focus();
        } else {
            self.fit_visible_graph();
        }
        self.dirty_session = true;
    }

    fn focus_is_centered(&self) -> bool {
        self.graph_focus_center().is_some_and(|center| {
            let point = self.nav.camera.screen(center);
            let safe = self.graph_safe_viewport();
            (point[0] - safe[0] / 2.0).abs() < 0.5 && (point[1] - safe[1] / 2.0).abs() < 0.5
        })
    }

    fn camera_is_fitted(&self) -> bool {
        let mut fitted = self.nav.camera.clone();
        fitted.fit(self.graph_view.size, self.graph_safe_viewport());
        fitted
            .pan
            .iter()
            .zip(self.nav.camera.pan)
            .all(|(a, b)| (*a - b).abs() < 0.5)
            && (fitted.zoom - self.nav.camera.zoom).abs() < 0.001
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
        if resized {
            self.graph_motion.interrupt_camera();
        }
        let centered = self.graph_focus_center().is_some_and(|center| {
            let screen = self.nav.camera.screen(center);
            (screen[0] - self.viewport[0] / 2.0).abs() < 0.1
                && (screen[1] - self.graph_safe_viewport()[1] / 2.0).abs() < 0.1
        });
        self.viewport = viewport;
        if self.preserve_adoption_camera {
            return;
        }
        if resized && self.file_focus_is_current() {
            self.fit_graph_focus();
            return;
        }
        if centered && self.view_is_current() {
            self.center_graph_focus();
        }
    }

    #[cfg(test)]
    pub(crate) fn set_graph_filter_height(&mut self, height: f32) {
        if self.graph_interaction.filter_height == height {
            return;
        }
        self.graph_interaction.filter_height = height;
        if self.preserve_adoption_camera {
            return;
        }
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
                self.graph_safe_viewport()[1] / 2.0 - center[1] * zoom,
            ];
            self.dirty_session = true;
        }
    }

    fn fit_graph_focus(&mut self) {
        if let Some(bounds) = focus_bounds(&self.graph_view) {
            let safe = self.graph_safe_viewport();
            let top = self.graph_interaction.filter_height.min(safe[1]);
            let available = [safe[0], (safe[1] - top).max(1.0)];
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

#[cfg(test)]
mod overlay_tests {
    use crate::app::State;

    #[test]
    fn fit_reserves_measured_controls_without_changing_canvas_coordinates() {
        let mut state = State::fixture();
        state.viewport = [1000.0, 700.0];
        state.source_popup.canvas = [50.0, 100.0];
        state.graph_view.size = [800.0, 600.0];
        state.visible_revision = Some((state.epoch, state.revision));
        state.fit_visible_graph();
        state.set_canvas_controls_top(650.0);
        assert_eq!(state.viewport, [1000.0, 700.0]);
        assert_eq!(state.ui.canvas_bottom_inset, 150.0);
        let corner = state.nav.camera.screen([800.0, 600.0]);
        assert!(corner[0] < 1000.0 && corner[1] < 550.0);
        assert_eq!(state.source_popup.canvas, [50.0, 100.0]);
    }

    #[test]
    fn changing_controls_preserves_a_manually_positioned_camera() {
        let mut state = State::fixture();
        state.viewport = [1000.0, 700.0];
        state.nav.camera.pan = [75.0, -30.0];
        state.nav.camera.zoom = 0.75;
        state.set_canvas_controls_top(550.0);
        assert_eq!(state.nav.camera.pan, [75.0, -30.0]);
        assert_eq!(state.nav.camera.zoom, 0.75);
        state.fit_visible_graph();
        assert_eq!(state.graph_safe_viewport(), [1000.0, 550.0]);
    }
    #[test]
    fn changing_controls_keeps_existing_focus_centered_without_resetting_zoom() {
        let mut state = State::fixture();
        state.viewport = [1000.0, 700.0];
        state.ui.canvas_bottom_inset = 80.0;
        state.graph_view.focus_keys = vec!["focus".into()];
        state.graph_view.nodes = vec![crate::projection::ViewNode {
            key: "focus".into(),
            position: [80.0, 120.0],
            size: [200.0, 60.0],
            ..Default::default()
        }];
        state.visible_revision = Some((state.epoch, state.revision));
        state.nav.camera.zoom = 0.75;
        state.center_graph_focus();
        assert_eq!(state.nav.camera.screen([180.0, 150.0]), [500.0, 310.0]);
        state.set_canvas_controls_top(550.0);
        assert_eq!(state.nav.camera.screen([180.0, 150.0]), [500.0, 275.0]);
        assert_eq!(state.nav.camera.zoom, 0.75);
    }

    #[test]
    fn resizing_keeps_a_focused_node_centered_above_measured_controls() {
        let mut state = State::fixture();
        state.viewport = [1000.0, 700.0];
        state.ui.canvas_bottom_inset = 150.0;
        state.graph_view.focus_keys = vec!["focus".into()];
        state.graph_view.nodes = vec![crate::projection::ViewNode {
            key: "focus".into(),
            position: [80.0, 120.0],
            size: [200.0, 60.0],
            ..Default::default()
        }];
        state.visible_revision = Some((state.epoch, state.revision));
        state.center_graph_focus();
        assert_eq!(state.nav.camera.screen([180.0, 150.0]), [500.0, 275.0]);
        state.resize_graph_viewport([1200.0, 900.0]);
        assert_eq!(state.nav.camera.screen([180.0, 150.0]), [600.0, 375.0]);
    }
}
