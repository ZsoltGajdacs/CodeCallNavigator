//! Standalone Shift taps; callers reset the sequence on pointer input or focus loss.
use rune::{Key, KeyEvent, Modifier, Mods};

const TAP_WINDOW: f64 = 0.4;

#[derive(Default)]
enum Sequence {
    #[default]
    Idle,
    Down {
        started: f64,
        previous: Option<f64>,
    },
    Up {
        started: f64,
    },
}

#[derive(Default)]
pub(crate) struct DoubleShift {
    held: u8,
    sequence: Sequence,
}

impl DoubleShift {
    pub(crate) fn reset(&mut self) {
        self.held = 0;
        self.sequence = Sequence::Idle;
    }

    /// Time is monotonic seconds. Complete both taps within the same 400 ms window.
    pub(crate) fn key(&mut self, event: &KeyEvent, time: f64) -> bool {
        let Some((bit, pressed)) = shift_key(event) else {
            self.sequence = Sequence::Idle;
            return false;
        };
        let previous_held = self.held;
        if pressed {
            self.held |= bit;
        } else {
            self.held &= !bit;
        }
        if !time.is_finite() || other_modifiers(event.mods) {
            self.sequence = Sequence::Idle;
            return false;
        }
        if pressed {
            self.press(previous_held, time);
            false
        } else {
            self.release(previous_held == bit && !event.mods.shift, time)
        }
    }

    fn press(&mut self, previous_held: u8, time: f64) {
        if previous_held != 0 {
            self.sequence = Sequence::Idle;
            return;
        }
        let previous = match self.sequence {
            Sequence::Up { started } if within_window(started, time) => Some(started),
            _ => None,
        };
        self.sequence = Sequence::Down {
            started: time,
            previous,
        };
    }

    fn release(&mut self, standalone: bool, time: f64) -> bool {
        let sequence = std::mem::take(&mut self.sequence);
        let Sequence::Down { started, previous } = sequence else {
            return false;
        };
        if !standalone || !within_window(started, time) {
            return false;
        }
        if let Some(first) = previous {
            return within_window(first, time);
        }
        self.sequence = Sequence::Up { started };
        false
    }
}

fn shift_key(event: &KeyEvent) -> Option<(u8, bool)> {
    match event.key {
        Key::Modifier {
            modifier: Modifier::ShiftLeft,
            pressed,
        } => Some((1, pressed)),
        Key::Modifier {
            modifier: Modifier::ShiftRight,
            pressed,
        } => Some((2, pressed)),
        _ => None,
    }
}

fn other_modifiers(mods: Mods) -> bool {
    mods.cmd || mods.ctrl || mods.alt || mods.logo
}

fn within_window(started: f64, time: f64) -> bool {
    (0.0..=TAP_WINDOW).contains(&(time - started))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shift(detector: &mut DoubleShift, side: Modifier, pressed: bool, time: f64) -> bool {
        detector.key(
            &KeyEvent {
                key: Key::Modifier {
                    modifier: side,
                    pressed,
                },
                mods: Mods {
                    shift: pressed,
                    ..Default::default()
                },
            },
            time,
        )
    }

    fn tap(detector: &mut DoubleShift, side: Modifier, time: f64) -> bool {
        assert!(!shift(detector, side, true, time));
        shift(detector, side, false, time + 0.025)
    }

    #[test]
    fn either_side_can_complete_the_second_tap() {
        for second in [Modifier::ShiftLeft, Modifier::ShiftRight] {
            let mut detector = DoubleShift::default();
            assert!(!tap(&mut detector, Modifier::ShiftLeft, 1.0));
            assert!(tap(&mut detector, second, 1.2));
            assert!(!tap(&mut detector, second, 1.3));
        }
    }

    #[test]
    fn holds_and_slow_sequences_do_not_trigger() {
        let mut detector = DoubleShift::default();
        assert!(!shift(&mut detector, Modifier::ShiftLeft, true, 1.0));
        assert!(!shift(&mut detector, Modifier::ShiftLeft, false, 1.5));
        assert!(!tap(&mut detector, Modifier::ShiftLeft, 1.6));
        assert!(!tap(&mut detector, Modifier::ShiftLeft, 2.1));
    }

    #[test]
    fn other_key_and_external_reset_cancel_pending_taps() {
        let mut detector = DoubleShift::default();
        assert!(!tap(&mut detector, Modifier::ShiftLeft, 1.0));
        assert!(!detector.key(
            &KeyEvent {
                key: Key::Text("A".into()),
                mods: Mods::default()
            },
            1.1
        ));
        assert!(!tap(&mut detector, Modifier::ShiftLeft, 1.2));
        detector.reset();
        assert!(!tap(&mut detector, Modifier::ShiftLeft, 1.3));
    }

    #[test]
    fn overlapping_shifts_and_repeat_presses_cancel_the_sequence() {
        let mut detector = DoubleShift::default();
        assert!(!shift(&mut detector, Modifier::ShiftLeft, true, 1.0));
        assert!(!shift(&mut detector, Modifier::ShiftRight, true, 1.05));
        assert!(!shift(&mut detector, Modifier::ShiftLeft, false, 1.1));
        assert!(!shift(&mut detector, Modifier::ShiftRight, false, 1.15));
        assert!(!tap(&mut detector, Modifier::ShiftLeft, 1.2));
        detector.reset();
        assert!(!shift(&mut detector, Modifier::ShiftLeft, true, 2.0));
        assert!(!shift(&mut detector, Modifier::ShiftLeft, true, 2.05));
        assert!(!shift(&mut detector, Modifier::ShiftLeft, false, 2.1));
        assert!(!tap(&mut detector, Modifier::ShiftLeft, 2.2));
    }

    #[test]
    fn other_modifiers_and_invalid_time_cancel_the_sequence() {
        let mut detector = DoubleShift::default();
        assert!(!tap(&mut detector, Modifier::ShiftLeft, 1.0));
        let event = KeyEvent {
            key: Key::Modifier {
                modifier: Modifier::ShiftLeft,
                pressed: true,
            },
            mods: Mods {
                shift: true,
                ctrl: true,
                ..Default::default()
            },
        };
        assert!(!detector.key(&event, 1.1));
        assert!(!shift(&mut detector, Modifier::ShiftLeft, false, 1.15));
        assert!(!tap(&mut detector, Modifier::ShiftLeft, 1.2));
        detector.reset();
        assert!(!tap(&mut detector, Modifier::ShiftLeft, 2.0));
        assert!(!shift(&mut detector, Modifier::ShiftLeft, true, f64::NAN));
        assert!(!shift(&mut detector, Modifier::ShiftLeft, false, 2.2));
    }
}
