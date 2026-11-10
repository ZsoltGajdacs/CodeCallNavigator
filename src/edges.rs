//! Paint Graphviz's reported geometry through Rune; no layout or call inference.
use crate::theme;
use crate::{app::State, graph_emphasis::GraphEmphasis, projection::ViewEdge};
use rune::{Color, LineCap, Shape};
use rune_kit::Kit;

#[path = "edges/geometry.rs"]
mod geometry;

#[derive(Clone, Copy)]
struct Style {
    color: Color,
    width: f32,
    dashed: bool,
}

pub fn render(state: &State, k: &Kit, emphasis: &GraphEmphasis) -> Shape {
    let mut shape = Shape::new();
    // Paint context first so bright connections stay on top at crossings.
    for emphasized in [false, true] {
        for (index, edge) in state.graph_view.edges.iter().enumerate() {
            if emphasis.edge_emphasized(index) != emphasized {
                continue;
            }
            let edge_style = style(edge, k, emphasis.edge_opacity(index));
            shape = render_edge(shape, state, edge, edge_style);
        }
    }
    shape
}

fn render_edge(shape: Shape, state: &State, edge: &ViewEdge, style: Style) -> Shape {
    let points: Vec<_> = edge
        .points
        .iter()
        .map(|&point| state.nav.camera.screen(point))
        .collect();
    if !visible(&points, state.viewport) {
        return shape;
    }
    stroke(shape, &points, style)
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

fn stroke(mut shape: Shape, control_points: &[[f32; 2]], style: Style) -> Shape {
    let points = spline_points(control_points);
    let arrow_direction = geometry::arrow_direction(control_points);
    if style.dashed {
        shape = dashed(shape, &points, style);
    } else {
        shape = shape.stroke(points, style.width, style.color);
    }
    arrow(shape, arrow_direction, style.color)
}

fn spline_points(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    geometry::flatten(points)
}

fn dashed(mut shape: Shape, points: &[[f32; 2]], style: Style) -> Shape {
    let mut cursor = DashCursor {
        drawing: true,
        remaining: 7.0,
    };
    for segment in points.windows(2) {
        shape = cursor.segment(shape, segment, style);
    }
    shape
}

struct DashCursor {
    drawing: bool,
    remaining: f32,
}

impl DashCursor {
    fn segment(&mut self, mut shape: Shape, segment: &[[f32; 2]], style: Style) -> Shape {
        let length = (segment[1][0] - segment[0][0]).hypot(segment[1][1] - segment[0][1]);
        if length <= f32::EPSILON {
            return shape;
        }
        let mut distance = 0.0;
        while distance < length {
            let step = self.remaining.min(length - distance);
            shape = self.draw_step(
                shape,
                [
                    lerp(segment, distance / length),
                    lerp(segment, (distance + step) / length),
                ],
                style,
            );
            distance += step;
            self.advance(step);
        }
        shape
    }

    fn draw_step(&self, shape: Shape, endpoints: [[f32; 2]; 2], style: Style) -> Shape {
        if !self.drawing {
            return shape;
        }
        shape.line(
            endpoints[0],
            endpoints[1],
            style.width,
            LineCap::Round,
            style.color,
        )
    }

    fn advance(&mut self, step: f32) {
        self.remaining -= step;
        if self.remaining <= 0.001 {
            self.drawing = !self.drawing;
            self.remaining = if self.drawing { 7.0 } else { 4.0 };
        }
    }
}

fn lerp(segment: &[[f32; 2]], fraction: f32) -> [f32; 2] {
    [0, 1].map(|axis| segment[0][axis] + (segment[1][axis] - segment[0][axis]) * fraction)
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
