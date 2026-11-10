//! A single zoom's prepared geometry. View replacement explicitly clears this cache.
use super::{Route, ViewEdge};

#[derive(Default)]
pub struct GeometryCache {
    zoom: Option<f32>,
    routes: Vec<Route>,
    #[cfg(test)]
    pub builds: usize,
}

impl GeometryCache {
    pub(super) fn routes(&mut self, edges: &[ViewEdge], zoom: f32) -> &[Route] {
        if self.zoom != Some(zoom) || self.routes.len() != edges.len() {
            self.routes = edges
                .iter()
                .map(|edge| Route::new(&edge.points, zoom))
                .collect();
            self.zoom = Some(zoom);
            #[cfg(test)]
            {
                self.builds += 1;
            }
        }
        &self.routes
    }
}
