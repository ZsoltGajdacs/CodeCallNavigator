//! Viewport measurements and once-per-session fitting preserve restored cameras.
use super::{Side, SideId, Workspace};
impl Workspace {
    pub fn layout(&mut self, side: SideId, viewport: [f32; 2]) {
        self.viewports[side.index()] = viewport;
    }
    /// Fit once after this session's pane is measured, independently of its widget predecessor.
    pub fn measured(&mut self, side: SideId, key: &str, viewport: [f32; 2]) {
        self.layout(side, viewport);
        let Some(pair) = &mut self.current else {
            return;
        };
        let Side::Ready(ready) = pair.side(side) else {
            return;
        };
        if ready.session.scroll_key != key || !std::mem::take(&mut pair.fit_pending[side.index()]) {
            return;
        }
        if let Side::Ready(ready) = pair.side_mut(side)
            && let Some(chart) = &ready.session.ready
        {
            ready.session.camera.fit(chart.chart.size, viewport);
        }
    }
}
