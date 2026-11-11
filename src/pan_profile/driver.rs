//! Three timed pointer drags, after the restored layout and viewport have settled.
use crate::app::State;
use rune::Runtime;
use std::time::{Duration, Instant};

pub(super) const DRAG_SECONDS: f64 = 10.0;
const DIRECTIONS: [[f32; 2]; 3] = [[180.0, 0.0], [180.0, 90.0], [-180.0, 90.0]];

type Layout = (Option<(u64, u64)>, usize, usize, [u32; 2]);

#[derive(Default)]
pub(super) struct Driver {
    layout: Option<Layout>,
    settled_since: Option<Instant>,
    active: Option<Drag>,
    completed: usize,
    requested: Option<Instant>,
    sampled_drag: Option<(usize, f64)>,
    finished: bool,
}

struct Drag {
    index: usize,
    started: Instant,
    origin: [f32; 2],
    point: [f32; 2],
}

impl Driver {
    pub(super) fn requested(&self) -> Option<Instant> {
        self.requested
    }

    pub(super) fn finished(&self) -> bool {
        self.finished
    }

    pub(super) fn sample(&self, runtime: &mut Runtime<State>) {
        let camera = &runtime.state.nav.camera;
        let (x, y, zoom) = (camera.pan[0], camera.pan[1], camera.zoom);
        runtime.trace_channel("codecallnavigator.camera.x", "position", x);
        runtime.trace_channel("codecallnavigator.camera.y", "position", y);
        runtime.trace_channel("codecallnavigator.camera.zoom", "level", zoom);
        runtime.trace_channel("codecallnavigator.backing_scale", "level", runtime.env.scale);
        runtime.trace_channel("codecallnavigator.viewport.width", "level", runtime.viewport.0);
        runtime.trace_channel("codecallnavigator.viewport.height", "level", runtime.viewport.1);
        let (drag, elapsed) = self
            .sampled_drag
            .map_or((-1.0, -1.0), |(i, t)| (i as f32, t as f32));
        runtime.trace_channel("codecallnavigator.drag", "phase", drag);
        runtime.trace_channel("codecallnavigator.drag_seconds", "level", elapsed);
        runtime.trace_channel(
            "codecallnavigator.nodes",
            "level",
            runtime.state.graph_view.nodes.len() as f32,
        );
        runtime.trace_channel(
            "codecallnavigator.edges",
            "level",
            runtime.state.graph_view.edges.len() as f32,
        );
    }

    pub(super) fn next(&mut self, runtime: &mut Runtime<State>) {
        if self.finished {
            return;
        }
        let now = Instant::now();
        if self.active.is_some() {
            self.advance_drag(runtime, now);
        } else if self.completed == DIRECTIONS.len() {
            self.finish(runtime, now);
        } else if self.ready(runtime, now) {
            self.start_drag(runtime, now);
        } else {
            self.sampled_drag = None;
            self.requested = None;
        }
    }

    fn advance_drag(&mut self, runtime: &mut Runtime<State>, now: Instant) {
        let drag = self.active.as_mut().expect("active drag");
        let seconds = now
            .duration_since(drag.started)
            .as_secs_f64()
            .min(DRAG_SECONDS);
        let offset = displacement(drag.index, seconds);
        drag.point = [drag.origin[0] + offset[0], drag.origin[1] + offset[1]];
        runtime.pointer_move(drag.point[0], drag.point[1]);
        self.requested = Some(now);
        self.sampled_drag = Some((drag.index, seconds));
        if seconds >= DRAG_SECONDS {
            runtime.pointer_up(drag.point[0], drag.point[1]);
            self.active = None;
            self.completed += 1;
            self.settled_since = Some(now);
        }
    }

    fn start_drag(&mut self, runtime: &mut Runtime<State>, now: Instant) {
        let Some(origin) = blank_point(runtime) else {
            eprintln!("pan profile: no blank canvas point for the drag");
            self.finished = true;
            runtime.update(|_, tx| tx.quit());
            return;
        };
        runtime.pointer_down(origin[0], origin[1]);
        self.active = Some(Drag {
            index: self.completed,
            started: now,
            origin,
            point: origin,
        });
        self.requested = Some(now);
        self.sampled_drag = Some((self.completed, 0.0));
    }

    fn finish(&mut self, runtime: &mut Runtime<State>, now: Instant) {
        self.sampled_drag = None;
        self.requested = None;
        // Leave a final frame for queued pointer-up and asynchronous timestamp results.
        if self
            .settled_since
            .is_some_and(|at| now.duration_since(at) >= Duration::from_secs(1))
        {
            self.finished = true;
            runtime.update(|_, tx| tx.quit());
        }
    }

    fn ready(&mut self, runtime: &Runtime<State>, now: Instant) -> bool {
        let state = &runtime.state;
        let layout = (
            state.visible_revision,
            state.graph_view.nodes.len(),
            state.graph_view.edges.len(),
            state.viewport.map(f32::to_bits),
        );
        let pending = state.needs_layout || state.jobs.layout.is_pending();
        let empty_or_stale = layout.1 == 0 || !state.view_is_current();
        if pending || empty_or_stale {
            self.settled_since = None;
            return false;
        }
        if self.layout != Some(layout) {
            self.layout = Some(layout);
            self.settled_since = Some(now);
        }
        self.settled_since
            .is_some_and(|at| now.duration_since(at) >= Duration::from_secs(1))
    }
}

/// A continuous sweep that returns to its origin; the view stays populated throughout.
pub(super) fn displacement(index: usize, seconds: f64) -> [f32; 2] {
    let phase = (seconds / DRAG_SECONDS * 4.0) as f32;
    let amount = if phase <= 1.0 {
        phase
    } else if phase <= 3.0 {
        2.0 - phase
    } else {
        phase - 4.0
    };
    DIRECTIONS[index].map(|distance| distance * amount)
}

fn blank_point(runtime: &Runtime<State>) -> Option<[f32; 2]> {
    let elements = runtime.elements();
    let canvas = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("graph-canvas"))?
        .rect;
    let cards: Vec<_> = elements
        .iter()
        .filter(|e| {
            e.key
                .as_deref()
                .is_some_and(|key| key.starts_with("graph-card-"))
        })
        .map(|e| e.rect)
        .collect();
    (0..20).find_map(|column| {
        let point = [
            canvas[0] + 8.0 + (canvas[2] - 16.0) * column as f32 / 19.0,
            canvas[1] + canvas[3] - 8.0,
        ];
        (!cards.iter().any(|r| {
            point[0] >= r[0]
                && point[0] <= r[0] + r[2]
                && point[1] >= r[1]
                && point[1] <= r[1] + r[3]
        }))
        .then_some(point)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{scenes, view};

    #[test]
    fn native_probe_queues_the_production_pointer_drag_handler() {
        let mut state = State::new();
        scenes::install(&mut state);
        let mut runtime = Runtime::new(
            state,
            view::view,
            (1420.0, 900.0),
            rune::Env::new(true, 1.0),
        );
        runtime.settle_all();
        runtime.frame(0.0);
        let origin = blank_point(&runtime).expect("blank canvas point");
        let camera = runtime.state.nav.camera.pan;
        runtime.pointer_down(origin[0], origin[1]);
        runtime.frame(1.0 / 60.0);
        let mut driver = Driver {
            active: Some(Drag {
                index: 1,
                started: Instant::now() - Duration::from_millis(2500),
                origin,
                point: origin,
            }),
            ..Default::default()
        };
        driver.next(&mut runtime);
        runtime.frame(1.0 / 60.0);
        assert!((runtime.state.nav.camera.pan[0] - camera[0] - 180.0).abs() < 1.0);
        assert!((runtime.state.nav.camera.pan[1] - camera[1] - 90.0).abs() < 1.0);
        assert!(runtime.state.dirty_session);
    }
}
