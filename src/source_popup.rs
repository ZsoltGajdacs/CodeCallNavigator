//! Transient source previews: independent of navigation, persistence and graph analysis.
mod interaction;
mod lifecycle;
mod panel;
mod render;
pub mod scenes;

use crate::{projection::ViewNode, source::SourceFile};
use std::sync::Arc;

pub(crate) use interaction::{Hover, Interaction};
pub use lifecycle::{SourceOutput, poll};
pub(crate) use panel::{Content, Heading, Owner, Panel};
pub use render::view;

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
}

#[derive(Default)]
pub struct Popup {
    pub hover: Option<Hover<ViewNode>>,
    pub active: Option<Preview>,
    pub interaction: Interaction,
    pub canvas: [f32; 2],
    pub serial: u64,
    pub cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl Popup {
    pub fn bounds(&self, state: &crate::app::State) -> [f32; 4] {
        let popup = &self.interaction;
        if popup.fixed() {
            return popup.fixed_bounds();
        }
        let node = &self.active.as_ref().unwrap().node;
        let node = state
            .graph_view
            .nodes
            .iter()
            .find(|current| current.key == node.key)
            .unwrap_or(node);
        let camera = &state.nav.camera;
        let [x, y] = camera.screen(node.position);
        popup.bounds_next_to([
            self.canvas[0] + x,
            self.canvas[1] + y,
            node.size[0] * camera.zoom,
            node.size[1] * camera.zoom,
        ])
    }
}
