//! Pan-independent routes and viewport-limited emission with continuous dash phase.
use super::{
    Style, arrow, geometry,
    viewport::{Viewport, point},
};
use rune::{LineCap, Shape};

pub(super) struct Paint {
    pub pan: [f32; 2],
    pub viewport: Viewport,
}

pub(super) struct Route {
    points: Vec<[f32; 2]>,
    distances: Vec<f64>,
    arrow: Option<([f32; 2], f32)>,
    bounds: [f32; 4],
}

impl Route {
    pub fn new(controls: &[[f32; 2]], zoom: f32) -> Self {
        let controls: Vec<_> = controls
            .iter()
            .map(|p| [p[0] * zoom, p[1] * zoom])
            .collect();
        let points = geometry::flatten(&controls);
        let mut distance = 0.0;
        let mut distances = vec![distance];
        for pair in points.windows(2) {
            distance += length([pair[0], pair[1]]);
            distances.push(distance);
        }
        let bounds = points.iter().fold(
            [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ],
            |mut b, p| {
                for axis in 0..2 {
                    b[axis] = b[axis].min(p[axis]);
                    b[axis + 2] = b[axis + 2].max(p[axis]);
                }
                b
            },
        );
        Self {
            points,
            distances,
            arrow: geometry::arrow_direction(&controls),
            bounds,
        }
    }

    pub fn paint(&self, mut shape: Shape, paint: Paint, style: Style) -> Shape {
        if !paint.viewport.intersects(self.bounds, paint.pan) {
            return shape;
        }
        shape = self.strokes(shape, &paint, style);
        if let Some((tip, angle)) = self.arrow {
            let tip = [tip[0] + paint.pan[0], tip[1] + paint.pan[1]];
            if paint.viewport.contains(tip) {
                shape = arrow(shape, Some((tip, angle)), style.color);
            }
        }
        shape
    }

    fn strokes(&self, mut shape: Shape, paint: &Paint, style: Style) -> Shape {
        let Paint { pan, viewport } = *paint;
        if !style.dashed && viewport.covers(self.bounds, pan) {
            let points: Vec<_> = self
                .points
                .iter()
                .map(|p| [p[0] + pan[0], p[1] + pan[1]])
                .collect();
            return shape.stroke(points, style.width, style.color);
        }
        for (index, pair) in self.points.windows(2).enumerate() {
            let segment = [pair[0], pair[1]].map(|p| [p[0] + pan[0], p[1] + pan[1]]);
            let Some(interval) = viewport.interval(segment) else {
                continue;
            };
            shape = if style.dashed {
                dashes(
                    shape,
                    segment,
                    DashSpan {
                        interval,
                        distances: [self.distances[index], self.distances[index + 1]],
                    },
                    style,
                )
            } else {
                line(
                    shape,
                    [point(segment, interval[0]), point(segment, interval[1])],
                    style,
                )
            };
        }
        shape
    }
}

fn length(segment: [[f32; 2]; 2]) -> f64 {
    (f64::from(segment[1][0]) - f64::from(segment[0][0]))
        .hypot(f64::from(segment[1][1]) - f64::from(segment[0][1]))
}

fn line(shape: Shape, segment: [[f32; 2]; 2], style: Style) -> Shape {
    shape.line(
        segment[0],
        segment[1],
        style.width,
        LineCap::Round,
        style.color,
    )
}

struct DashSpan {
    interval: [f64; 2],
    distances: [f64; 2],
}

fn dashes(mut shape: Shape, segment: [[f32; 2]; 2], span: DashSpan, style: Style) -> Shape {
    let DashSpan {
        interval,
        distances,
    } = span;
    let length = distances[1] - distances[0];
    if length <= f64::from(f32::EPSILON) {
        return shape;
    }
    let start = interval[0] * length;
    let span = (interval[1] - interval[0]) * length;
    let mut phase = (distances[0] + start).rem_euclid(11.0);
    let mut distance = 0.0;
    while distance < span {
        let drawing = phase < 7.0;
        let step = (if drawing { 7.0 - phase } else { 11.0 - phase }).min(span - distance);
        let next = distance + step;
        if next <= distance {
            break;
        }
        if drawing {
            shape = line(
                shape,
                [
                    point(segment, (start + distance) / length),
                    point(segment, (start + next) / length),
                ],
                style,
            );
        }
        distance = next;
        phase = (phase + step).rem_euclid(11.0);
    }
    shape
}
