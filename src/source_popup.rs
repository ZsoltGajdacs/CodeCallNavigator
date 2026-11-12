//! Transient source previews: independent of navigation, persistence and graph analysis.
mod lifecycle;
mod render;
pub mod scenes;

use crate::{projection::ViewNode, source::SourceFile};
use std::sync::Arc;

pub use lifecycle::{SourceOutput, poll};
pub use render::view;

pub struct Hover {
    pub node: ViewNode,
    pub deadline: Option<f64>,
}

pub enum Contents {
    Files,
    Loading,
    Source(Arc<SourceFile>),
    Error(String),
}

pub struct Preview {
    pub node: ViewNode,
    pub contents: Contents,
    pub path: Option<String>,
    pub requested_line: u32,
    pub target: usize,
    pub location_warning: Option<String>,
    pub pinned: bool,
}

pub struct Popup {
    pub hover: Option<Hover>,
    pub active: Option<Preview>,
    pub inside: bool,
    pub trigger_inside: bool,
    pub close_at: Option<f64>,
    pub size: [f32; 2],
    pub rect: [f32; 4],
    pub window: [f32; 2],
    pub canvas: [f32; 2],
    pub serial: u64,
    pub cancelled: Arc<std::sync::atomic::AtomicBool>,
    pub now: f64,
    pub resizing: bool,
    pub resize_start: [f32; 2],
    pub resize_saved: [f32; 2],
}

impl Default for Popup {
    fn default() -> Self {
        Self {
            hover: None,
            active: None,
            inside: false,
            trigger_inside: false,
            close_at: None,
            size: [720.0, 480.0],
            rect: [16.0, 16.0, 720.0, 480.0],
            window: [1420.0, 900.0],
            canvas: [0.0; 2],
            serial: 0,
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            now: 0.0,
            resizing: false,
            resize_start: [720.0, 480.0],
            resize_saved: [720.0, 480.0],
        }
    }
}

impl Popup {
    pub fn bounds(&self, state: &crate::app::State) -> [f32; 4] {
        let size = self.constrained_size(self.size);
        let fixed = self.resizing || self.active.as_ref().is_some_and(|p| p.pinned);
        let origin = if fixed {
            [self.rect[0], self.rect[1]]
        } else {
            let node = self.active.as_ref().map(|p| &p.node).unwrap();
            let point = state.nav.camera.screen(node.position);
            let right = self.canvas[0] + point[0] + node.size[0] * state.nav.camera.zoom + 8.0;
            let left = self.canvas[0] + point[0] - size[0] - 8.0;
            [
                if right + size[0] <= self.window[0] - 16.0 {
                    right
                } else {
                    left
                },
                self.canvas[1] + point[1],
            ]
        };
        [
            origin[0].clamp(16.0, (self.window[0] - size[0] - 16.0).max(16.0)),
            origin[1].clamp(16.0, (self.window[1] - size[1] - 16.0).max(16.0)),
            size[0],
            size[1],
        ]
    }

    pub fn constrained_size(&self, size: [f32; 2]) -> [f32; 2] {
        [
            size[0].max(360.0).min((self.window[0] - 32.0).max(1.0)),
            size[1].max(240.0).min((self.window[1] - 32.0).max(1.0)),
        ]
    }
}
