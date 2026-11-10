//! Paint Graphviz's reported geometry through Rune; no layout or call inference.
use crate::theme;
use crate::{app::State, graph_emphasis::GraphEmphasis, projection::ViewEdge};
use rune::{Color, Shape};
use rune_kit::Kit;

#[path = "edges/cache.rs"]
mod cache;
#[path = "edges/geometry.rs"]
mod geometry;
#[path = "edges/route.rs"]
mod prepared;
#[path = "edges/viewport.rs"]
mod viewport;

pub use cache::GeometryCache;
use prepared::{Paint, Route};
use viewport::Viewport;

#[derive(Clone, Copy)]
struct Style {
    color: Color,
    width: f32,
    dashed: bool,
}

pub fn render(state: &State, k: &Kit, emphasis: &GraphEmphasis) -> Shape {
    let mut shape = Shape::new();
    let mut cache = state.edge_geometry.borrow_mut();
    let routes = cache.routes(&state.graph_view.edges, state.nav.camera.zoom);
    let viewport = Viewport::new(state.viewport);
    // Paint context first so bright connections stay on top at crossings.
    for emphasized in [false, true] {
        for (index, edge) in state.graph_view.edges.iter().enumerate() {
            if emphasis.edge_emphasized(index) != emphasized {
                continue;
            }
            let edge_style = style(edge, k, emphasis.edge_opacity(index));
            shape = routes[index].paint(
                shape,
                Paint {
                    pan: state.nav.camera.pan,
                    viewport,
                },
                edge_style,
            );
        }
    }
    shape
}

fn style(edge: &ViewEdge, k: &Kit, opacity: f32) -> Style {
    let reference = codegraph::model::relationship_is_reference(&edge.kind);
    let direct = !codegraph::model::relationship_is_uncertain(&edge.certainty);
    let color = if reference {
        theme::callback(k)
    } else if direct {
        k.c.text_muted.alpha(0.7)
    } else {
        theme::uncertain(k)
    };
    Style {
        color: color.alpha(color.a * opacity),
        width: (1.2 + (edge.count as f32).ln_1p() * 0.12).min(2.4),
        dashed: !direct || reference,
    }
}

pub(crate) fn visible(points: &[[f32; 2]], viewport: [f32; 2]) -> bool {
    if points.len() < 2 {
        return false;
    }
    let bounds = points.iter().fold(
        [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ],
        |mut bounds, point| {
            bounds[0] = bounds[0].min(point[0]);
            bounds[1] = bounds[1].min(point[1]);
            bounds[2] = bounds[2].max(point[0]);
            bounds[3] = bounds[3].max(point[1]);
            bounds
        },
    );
    bounds[2] >= -10.0
        && bounds[3] >= -10.0
        && bounds[0] <= viewport[0] + 10.0
        && bounds[1] <= viewport[1] + 10.0
}

fn stroke(shape: Shape, control_points: &[[f32; 2]], style: Style) -> Shape {
    Route::new(control_points, 1.0).paint(
        shape,
        Paint {
            pan: [0.0, 0.0],
            viewport: Viewport::unbounded(),
        },
        style,
    )
}

#[cfg(test)]
fn spline_points(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    geometry::flatten(points)
}

fn arrow(shape: Shape, direction: Option<([f32; 2], f32)>, color: Color) -> Shape {
    let Some((tip, angle)) = direction else {
        return shape;
    };
    let rear = [tip[0] - 7.0 * angle.cos(), tip[1] - 7.0 * angle.sin()];
    shape.fill(
        vec![
            tip,
            [rear[0] + 3.0 * angle.sin(), rear[1] - 3.0 * angle.cos()],
            [rear[0] - 3.0 * angle.sin(), rear[1] + 3.0 * angle.cos()],
        ],
        color,
    )
}

/// Draw an already-laid-out connector using the shared spline and arrow renderer.
pub fn route(shape: Shape, points: &[[f32; 2]], color: Color, dashed: bool) -> Shape {
    if points.len() < 2 {
        return shape;
    }
    stroke(
        shape,
        points,
        Style {
            color,
            width: 1.5,
            dashed,
        },
    )
}

#[cfg(test)]
#[path = "edges/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "edges/viewport_tests.rs"]
mod viewport_tests;
