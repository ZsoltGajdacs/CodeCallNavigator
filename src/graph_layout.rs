//! One completion path for asynchronous and deterministic graph layouts.
use crate::{
    app::State,
    projection::{GraphView, Grouping},
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
        self.graph_view = view;
        self.reconcile_visual_selection();
        self.visible_revision = Some((self.epoch, self.revision));
        self.nav.page = self.nav.page.min(self.graph_view.pages.saturating_sub(1));
        if self.auto_fit {
            self.fit_visible_graph();
            self.auto_fit = false;
        }
        if self.graph_interaction.center_after_layout {
            self.center_graph_focus();
            self.graph_interaction.center_after_layout = false;
        }
    }

    pub fn fit_visible_graph(&mut self) {
        let inset = self
            .graph_overlay_height
            .min((self.viewport[1] - 48.0).max(0.0));
        let available = [self.viewport[0], self.viewport[1] - inset];
        self.nav.camera.fit(self.graph_view.size, available);
        self.nav.camera.pan[1] += inset;
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
        let centered = self.graph_focus_center().is_some_and(|center| {
            let screen = self.nav.camera.screen(center);
            (screen[0] - self.viewport[0] / 2.0).abs() < 0.1
                && (screen[1] - self.viewport[1] / 2.0).abs() < 0.1
        });
        self.viewport = viewport;
        if centered && self.view_is_current() {
            self.center_graph_focus();
        }
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

    fn graph_focus_center(&self) -> Option<[f32; 2]> {
        let mut bounds = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        let mut found = false;
        for node in &self.graph_view.nodes {
            if self.graph_view.focus_keys.contains(&node.key) {
                found = true;
                bounds[0] = bounds[0].min(node.position[0]);
                bounds[1] = bounds[1].min(node.position[1]);
                bounds[2] = bounds[2].max(node.position[0] + node.size[0]);
                bounds[3] = bounds[3].max(node.position[1] + node.size[1]);
            }
        }
        found.then_some([(bounds[0] + bounds[2]) / 2.0, (bounds[1] + bounds[3]) / 2.0])
    }
}
