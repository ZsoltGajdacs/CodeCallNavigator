//! Screen-space line clipping; padding keeps caps, antialiasing and arrows intact.
#[derive(Clone, Copy)]
pub(super) struct Viewport {
    bounds: [f32; 4],
}

impl Viewport {
    pub fn new(size: [f32; 2]) -> Self {
        Self {
            bounds: [-10.0, -10.0, size[0] + 10.0, size[1] + 10.0],
        }
    }

    pub fn unbounded() -> Self {
        Self {
            bounds: [
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::INFINITY,
            ],
        }
    }

    pub fn contains(self, point: [f32; 2]) -> bool {
        (0..2).all(|axis| point[axis] >= self.bounds[axis] && point[axis] <= self.bounds[axis + 2])
    }

    pub fn intersects(self, bounds: [f32; 4], pan: [f32; 2]) -> bool {
        (0..2).all(|axis| {
            bounds[axis + 2] + pan[axis] >= self.bounds[axis]
                && bounds[axis] + pan[axis] <= self.bounds[axis + 2]
        })
    }

    pub fn covers(self, bounds: [f32; 4], pan: [f32; 2]) -> bool {
        self.contains([bounds[0] + pan[0], bounds[1] + pan[1]])
            && self.contains([bounds[2] + pan[0], bounds[3] + pan[1]])
    }

    /// Visible parameter interval on the original segment, for preserving dash phase.
    pub fn interval(self, segment: [[f32; 2]; 2]) -> Option<[f64; 2]> {
        let mut interval = [0.0_f64, 1.0_f64];
        for (axis, coordinate) in segment[0].iter().enumerate() {
            let origin = f64::from(*coordinate);
            let delta = f64::from(segment[1][axis]) - origin;
            if delta == 0.0 {
                if origin < f64::from(self.bounds[axis])
                    || origin > f64::from(self.bounds[axis + 2])
                {
                    return None;
                }
                continue;
            }
            let a = (f64::from(self.bounds[axis]) - origin) / delta;
            let b = (f64::from(self.bounds[axis + 2]) - origin) / delta;
            interval[0] = interval[0].max(a.min(b));
            interval[1] = interval[1].min(a.max(b));
            if interval[0] > interval[1] {
                return None;
            }
        }
        Some(interval)
    }
}

pub(super) fn point(segment: [[f32; 2]; 2], t: f64) -> [f32; 2] {
    [0, 1].map(|axis| {
        (f64::from(segment[0][axis]) * (1.0 - t) + f64::from(segment[1][axis]) * t) as f32
    })
}
