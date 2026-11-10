//! Camera transforms and viewport geometry, independent of graph interactions.
use crate::projection::ViewNode;
use rune::{Drag, Phase};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera {
    pub pan: [f32; 2],
    pub zoom: f32,
    #[serde(skip)]
    drag_start: [f32; 2],
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pan: [24.0, 24.0],
            zoom: 1.0,
            drag_start: [0.0; 2],
        }
    }
}

impl Camera {
    pub fn sanitize(&mut self) {
        if !self.zoom.is_finite() {
            self.zoom = 1.0;
        }
        self.zoom = self.zoom.clamp(0.08, 3.0);
        if !self.pan.iter().all(|value| value.is_finite()) {
            self.pan = [24.0, 24.0];
        }
    }
    pub fn screen(&self, point: [f32; 2]) -> [f32; 2] {
        [
            point[0] * self.zoom + self.pan[0],
            point[1] * self.zoom + self.pan[1],
        ]
    }

    pub fn zoom_at(&mut self, point: [f32; 2], factor: f32) {
        let world = [
            (point[0] - self.pan[0]) / self.zoom,
            (point[1] - self.pan[1]) / self.zoom,
        ];
        self.zoom = (self.zoom * factor).clamp(0.08, 3.0);
        self.pan = [
            point[0] - world[0] * self.zoom,
            point[1] - world[1] * self.zoom,
        ];
    }

    pub fn fit(&mut self, world: [f32; 2], viewport: [f32; 2]) {
        self.zoom = ((viewport[0] - 48.0) / world[0].max(1.0))
            .min((viewport[1] - 48.0) / world[1].max(1.0))
            .clamp(0.08, 1.2);
        self.pan = [
            (viewport[0] - world[0] * self.zoom) / 2.0,
            (viewport[1] - world[1] * self.zoom) / 2.0,
        ];
    }

    pub fn drag(&mut self, drag: &Drag) {
        if drag.phase == Phase::Down {
            self.drag_start = self.pan;
        }
        self.pan = if drag.cancelled() {
            self.drag_start
        } else {
            [self.drag_start[0] + drag.dx, self.drag_start[1] + drag.dy]
        };
    }

    pub fn shows(&self, node: &ViewNode, viewport: [f32; 2]) -> bool {
        self.shows_box(node.position, node.size, viewport)
    }

    pub fn shows_box(&self, position: [f32; 2], size: [f32; 2], viewport: [f32; 2]) -> bool {
        let origin = self.screen(position);
        origin[0] + size[0] * self.zoom >= 0.0
            && origin[1] + size[1] * self.zoom >= 0.0
            && origin[0] <= viewport[0]
            && origin[1] <= viewport[1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_the_world_point_under_the_pointer() {
        let mut camera = Camera::default();
        let point = [150.0, 100.0];
        let shown = camera.screen(point);
        camera.zoom_at(shown, 2.0);
        assert_eq!(camera.screen(point), shown);
        camera.fit([1000.0, 500.0], [600.0, 400.0]);
        assert!(camera.zoom > 0.0 && camera.zoom < 1.0);
    }
}
