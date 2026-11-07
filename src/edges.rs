//! Paint Graphviz's reported geometry through Rune; no layout or call inference.
use crate::{
    camera::Camera,
    projection::{GraphView, ViewEdge},
};
use rune::{Color, LineCap, Shape};

#[derive(Clone, Copy)]
struct Style {
    color: Color,
    width: f32,
    dashed: bool,
}

pub fn render(view: &GraphView, camera: &Camera, viewport: [f32; 2], dark: bool) -> Shape {
    let mut shape = Shape::new();
    for edge in &view.edges {
        let points: Vec<_> = edge
            .points
            .iter()
            .map(|&point| camera.screen(point))
            .collect();
        if !visible(&points, viewport) {
            continue;
        }
        shape = stroke(shape, &points, style(edge, dark));
    }
    shape
}

fn style(edge: &ViewEdge, dark: bool) -> Style {
    let reference = matches!(
        edge.kind.as_str(),
        "methodgroup" | "_LM" | "reference" | "callback"
    );
    let direct = matches!(edge.certainty.as_str(), "direct" | "proven-direct");
    let color = if reference {
        rune::oklch(if dark { 0.76 } else { 0.5 }, 0.13, 300.0)
    } else if direct {
        rune::tok::theme(dark).text_secondary.alpha(0.7)
    } else {
        rune::oklch(if dark { 0.76 } else { 0.5 }, 0.12, 75.0)
    };
    Style {
        color,
        width: (1.2 + (edge.count as f32).ln_1p() * 0.12).min(2.4),
        dashed: !direct || reference,
    }
}

fn visible(points: &[[f32; 2]], viewport: [f32; 2]) -> bool {
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
    if style.dashed {
        shape = dashed(shape, &points, style);
    } else {
        shape = shape.stroke(points.clone(), style.width, style.color);
    }
    arrow(shape, &points, style.color)
}

fn spline_points(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    if !(points.len() - 1).is_multiple_of(3) {
        return points.to_vec();
    }
    let mut flattened = vec![points[0]];
    for start in (0..points.len() - 1).step_by(3) {
        for step in 1..=24 {
            flattened.push(cubic(&points[start..start + 4], step as f32 / 24.0));
        }
    }
    flattened
}

fn cubic(points: &[[f32; 2]], t: f32) -> [f32; 2] {
    let u = 1.0 - t;
    let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    [0, 1].map(|axis| {
        weights
            .iter()
            .zip(points)
            .map(|(weight, point)| weight * point[axis])
            .sum()
    })
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

fn arrow(shape: Shape, points: &[[f32; 2]], color: Color) -> Shape {
    let tip = points[points.len() - 1];
    let previous = points[points.len() - 2];
    let angle = (tip[1] - previous[1]).atan2(tip[0] - previous[0]);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn culling_keeps_crossing_splines_and_drops_offscreen_geometry() {
        assert!(visible(&[[-20.0, 50.0], [150.0, 50.0]], [100.0, 100.0]));
        assert!(!visible(&[[-20.0, -50.0], [150.0, -50.0]], [100.0, 100.0]));
        let controls = [[0.0, 0.0], [10.0, 20.0], [30.0, 20.0], [40.0, 0.0]];
        let spline = spline_points(&controls);
        assert_eq!(spline.first(), Some(&controls[0]));
        assert_eq!(spline.last(), Some(&controls[3]));
        assert!(spline[12][1] > 0.0);
    }
}
