//! Screen-space cubic tessellation with bounded visual error and subdivision work.
const TOLERANCE: f32 = 0.5;
const MAX_DEPTH: u8 = 12;

pub(super) fn flatten(points: &[[f32; 2]]) -> Vec<[f32; 2]> {
    if points.len() < 4 || !(points.len() - 1).is_multiple_of(3) {
        return points.to_vec();
    }
    let mut result = Vec::with_capacity(points.len());
    result.push(points[0]);
    for start in (0..points.len() - 1).step_by(3) {
        subdivide(points[start..start + 4].try_into().unwrap(), 0, &mut result);
    }
    result
}

fn subdivide(curve: [[f32; 2]; 4], depth: u8, result: &mut Vec<[f32; 2]>) {
    let chord = [curve[0], curve[3]];
    if depth == MAX_DEPTH
        || curve[1..3]
            .iter()
            .all(|&point| distance_squared(point, chord) <= TOLERANCE * TOLERANCE)
    {
        result.push(curve[3]);
        return;
    }
    let a = midpoint(curve[0], curve[1]);
    let b = midpoint(curve[1], curve[2]);
    let c = midpoint(curve[2], curve[3]);
    let d = midpoint(a, b);
    let e = midpoint(b, c);
    let middle = midpoint(d, e);
    subdivide([curve[0], a, d, middle], depth + 1, result);
    subdivide([middle, e, c, curve[3]], depth + 1, result);
}

fn midpoint(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] * 0.5 + b[0] * 0.5, a[1] * 0.5 + b[1] * 0.5]
}

fn distance_squared(point: [f32; 2], chord: [[f32; 2]; 2]) -> f32 {
    let delta = [chord[1][0] - chord[0][0], chord[1][1] - chord[0][1]];
    let offset = [point[0] - chord[0][0], point[1] - chord[0][1]];
    let length = delta[0] * delta[0] + delta[1] * delta[1];
    let fraction = if length > 0.0 {
        ((offset[0] * delta[0] + offset[1] * delta[1]) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (offset[0] - fraction * delta[0]).powi(2) + (offset[1] - fraction * delta[1]).powi(2)
}

/// Endpoint direction remains independent of tessellation tolerance and zoom.
pub(super) fn arrow_direction(points: &[[f32; 2]]) -> Option<([f32; 2], f32)> {
    let &tip = points.last()?;
    let &previous = points.iter().rev().find(|&&point| point != tip)?;
    Some((tip, (tip[1] - previous[1]).atan2(tip[0] - previous[0])))
}
