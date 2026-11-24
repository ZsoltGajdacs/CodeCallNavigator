//! Window geometry and pointer lifetime shared by graph and comparison previews.
use rune::{Drag, Phase, Tx};
use std::time::Duration;

const OPEN_DELAY: Duration = Duration::from_millis(400);
const CLOSE_DELAY: Duration = Duration::from_millis(300);

pub struct Hover<T> {
    pub target: T,
    deadline: f64,
}

impl<T> Hover<T> {
    pub fn ready(&self, now: f64, tx: &mut Tx) -> bool {
        if now >= self.deadline {
            return true;
        }
        tx.wake_after(Duration::from_secs_f64(self.deadline - now));
        false
    }
}

pub struct Interaction {
    pub inside: bool,
    pub trigger_inside: bool,
    pub close_at: Option<f64>,
    pub size: [f32; 2],
    pub rect: [f32; 4],
    pub window: [f32; 2],
    pub now: f64,
    pub pinned: bool,
    resizing: Option<Resize>,
}

struct Resize {
    start: [f32; 2],
    saved: [f32; 2],
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            inside: false,
            trigger_inside: false,
            close_at: None,
            size: [440.0, 260.0],
            rect: [16.0, 16.0, 440.0, 260.0],
            window: [1420.0, 900.0],
            now: 0.0,
            pinned: false,
            resizing: None,
        }
    }
}

impl Interaction {
    pub fn can_hover(&self) -> bool {
        !self.inside && !self.fixed()
    }

    pub fn begin_hover<T>(&mut self, target: T, tx: &mut Tx) -> Hover<T> {
        self.close_at = None;
        tx.wake_after(OPEN_DELAY);
        Hover {
            target,
            deadline: self.now + OPEN_DELAY.as_secs_f64(),
        }
    }

    pub fn schedule_close(&mut self, tx: &mut Tx) {
        self.close_at = Some(self.now + CLOSE_DELAY.as_secs_f64());
        tx.wake_after(CLOSE_DELAY);
    }

    pub fn popup_hover(&mut self, inside: bool, tx: &mut Tx) {
        self.inside = inside;
        if inside {
            self.close_at = None;
        } else {
            self.schedule_close(tx);
        }
    }

    pub fn should_close(&mut self, focused: bool, tx: &mut Tx) -> bool {
        let hovered = self.inside || self.trigger_inside;
        let retained = self.fixed() || focused;
        if hovered || retained {
            self.close_at = None;
            return false;
        }
        let deadline = *self
            .close_at
            .get_or_insert(self.now + CLOSE_DELAY.as_secs_f64());
        if self.now >= deadline {
            return true;
        }
        tx.wake_after(Duration::from_secs_f64(deadline - self.now));
        false
    }

    pub fn reset(&mut self) {
        self.inside = false;
        self.trigger_inside = false;
        self.close_at = None;
        self.pinned = false;
        self.resizing = None;
    }

    pub fn fixed(&self) -> bool {
        self.pinned || self.resizing.is_some()
    }

    pub fn toggle_pin(&mut self) {
        self.pinned = !self.pinned;
        self.close_at = None;
    }

    pub fn resize(&mut self, drag: &Drag) {
        if drag.phase == Phase::Down {
            self.resizing = Some(Resize {
                start: [self.rect[2], self.rect[3]],
                saved: self.size,
            });
        }
        let Some(resize) = &self.resizing else { return };
        let size = if drag.cancelled() {
            resize.saved
        } else {
            [resize.start[0] + drag.dx, resize.start[1] + drag.dy]
        };
        self.size = self.constrained_size(size);
        if drag.phase == Phase::Up {
            self.resizing = None;
        }
    }

    pub fn constrained_size(&self, size: [f32; 2]) -> [f32; 2] {
        [
            size[0].max(360.0).min((self.window[0] - 32.0).max(1.0)),
            size[1].max(240.0).min((self.window[1] - 32.0).max(1.0)),
        ]
    }

    pub fn fixed_bounds(&self) -> [f32; 4] {
        self.bounds_at([self.rect[0], self.rect[1]])
    }

    pub fn bounds_next_to(&self, trigger: [f32; 4]) -> [f32; 4] {
        let [x, y, w, _] = trigger;
        let width = self.constrained_size(self.size)[0];
        let right = x + w + 8.0;
        let x = if right + width <= self.window[0] - 16.0 {
            right
        } else {
            x - width - 8.0
        };
        self.bounds_at([x, y])
    }

    fn bounds_at(&self, origin: [f32; 2]) -> [f32; 4] {
        let [w, h] = self.constrained_size(self.size);
        [
            origin[0].clamp(16.0, (self.window[0] - w - 16.0).max(16.0)),
            origin[1].clamp(16.0, (self.window[1] - h - 16.0).max(16.0)),
            w,
            h,
        ]
    }
}
