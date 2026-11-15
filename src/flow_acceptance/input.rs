//! Split gestures across real frames; never recursively step a native runtime.
use rune::{Runtime, WheelEvent};
use std::time::{Duration, Instant};

enum Gesture {
    Release([f32; 2]),
    EditRelease { point: [f32; 2], text: String },
    Paste(String),
    Move { end: [f32; 2] },
    Settling(Instant),
}

#[derive(Default)]
pub(super) struct Input {
    gesture: Option<Gesture>,
}

impl Input {
    pub fn busy(&self) -> bool {
        self.gesture.is_some()
    }

    pub fn next<S: 'static>(&mut self, runtime: &mut Runtime<S>) {
        self.gesture = match self.gesture.take() {
            Some(Gesture::EditRelease { point, text }) => {
                runtime.pointer_up(point[0], point[1]);
                Some(Gesture::Paste(text))
            }
            Some(Gesture::Paste(text)) => {
                runtime.key(rune::KeyEvent {
                    key: rune::Key::Command('a'),
                    mods: rune::Mods {
                        cmd: true,
                        logo: true,
                        ..Default::default()
                    },
                });
                runtime.paste(text);
                Some(Gesture::Settling(
                    Instant::now() + Duration::from_millis(100),
                ))
            }
            Some(Gesture::Release(point)) => {
                runtime.pointer_up(point[0], point[1]);
                Some(Gesture::Settling(
                    Instant::now() + Duration::from_millis(100),
                ))
            }
            Some(Gesture::Move { end }) => {
                runtime.pointer_move(end[0], end[1]);
                Some(Gesture::Release(end))
            }
            Some(Gesture::Settling(until)) if Instant::now() < until => {
                Some(Gesture::Settling(until))
            }
            _ => None,
        };
    }

    pub fn click<S: 'static>(&mut self, runtime: &mut Runtime<S>, key: &str) -> bool {
        self.begin_press(runtime, key, None)
    }
    pub fn edit<S: 'static>(&mut self, runtime: &mut Runtime<S>, key: &str, text: &str) -> bool {
        self.begin_press(runtime, key, Some(text))
    }
    fn begin_press<S: 'static>(
        &mut self,
        runtime: &mut Runtime<S>,
        key: &str,
        text: Option<&str>,
    ) -> bool {
        let Some(point) = point(runtime, key) else {
            return false;
        };
        runtime.pointer_move(point[0], point[1]);
        runtime.pointer_down(point[0], point[1]);
        self.gesture = Some(match text {
            Some(text) => Gesture::EditRelease {
                point,
                text: text.into(),
            },
            None => Gesture::Release(point),
        });
        true
    }
    pub fn pan<S: 'static>(&mut self, runtime: &mut Runtime<S>) -> bool {
        self.pan_canvas(runtime, "flow-canvas", "flow-step-")
    }
    pub fn pan_canvas<S: 'static>(
        &mut self,
        runtime: &mut Runtime<S>,
        key: &str,
        steps: &str,
    ) -> bool {
        let elements = runtime.elements();
        let Some(canvas) = elements
            .iter()
            .find(|element| element.key.as_deref() == Some(key))
        else {
            return false;
        };
        let [x, y, width, height] = canvas.visible;
        let candidate = (1..5).find_map(|column| {
            (1..5).find_map(|row| {
                let candidate = [
                    x + width * column as f32 / 6.0,
                    y + height * row as f32 / 6.0,
                ];
                let covered = elements.iter().any(|element| {
                    element
                        .key
                        .as_deref()
                        .is_some_and(|key| key.starts_with(steps))
                        && contains(element.rect, candidate)
                });
                (!covered && contains(canvas.visible, [candidate[0] + 24.0, candidate[1] + 18.0]))
                    .then_some(candidate)
            })
        });
        let Some(origin) = candidate else {
            return false;
        };
        runtime.pointer_move(origin[0], origin[1]);
        runtime.pointer_down(origin[0], origin[1]);
        self.gesture = Some(Gesture::Move {
            end: [origin[0] + 24.0, origin[1] + 18.0],
        });
        true
    }

    pub fn scroll_source<S: 'static>(&mut self, runtime: &mut Runtime<S>, offset: f32) -> bool {
        self.scroll_pane(runtime, "flow-source-pane", offset)
    }
    pub fn scroll_pane<S: 'static>(
        &mut self,
        runtime: &mut Runtime<S>,
        key: &str,
        offset: f32,
    ) -> bool {
        let Some(at) = point(runtime, key) else {
            return false;
        };
        let delta = if offset > 80.0 { 80.0 } else { -80.0 };
        runtime.wheel_event(WheelEvent::new(0.0, delta).at(at[0], at[1]).precise(true));
        self.gesture = Some(Gesture::Settling(
            Instant::now() + Duration::from_millis(120),
        ));
        true
    }
}

fn contains(rectangle: [f32; 4], point: [f32; 2]) -> bool {
    let [x, y, width, height] = rectangle;
    point[0] > x && point[0] < x + width && point[1] > y && point[1] < y + height
}

fn point<S: 'static>(runtime: &Runtime<S>, key: &str) -> Option<[f32; 2]> {
    let element = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some(key))?;
    let [x, y, width, height] = element.visible;
    (width > 1.0 && height > 1.0).then_some([x + width / 2.0, y + height / 2.0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, el, text};

    #[test]
    fn clicks_are_dispatched_through_runtime_controls_across_frames() {
        let mut runtime = Runtime::new(
            0usize,
            |_, _| {
                el().size(100.0, 50.0)
                    .key("actual-control")
                    .on_press(|count: &mut usize| *count += 1)
                    .child(text("press"))
            },
            (200.0, 100.0),
            Env::new(true, 1.0),
        );
        runtime.frame(0.0);
        let mut input = Input::default();
        assert!(!input.click(&mut runtime, "missing"));
        assert!(input.click(&mut runtime, "actual-control"));
        runtime.frame(1.0 / 60.0);
        assert_eq!(runtime.state, 0);
        input.next(&mut runtime);
        runtime.frame(1.0 / 60.0);
        assert_eq!(runtime.state, 1);
    }
    #[test]
    fn editing_uses_real_focus_select_all_and_runtime_paste() {
        let mut runtime = Runtime::new(
            rune::TextEdit::new("old"),
            |state, _| {
                rune_kit::TextInput::new(state)
                    .on_edit(|state: &mut rune::TextEdit| state)
                    .build(&crate::theme::kit(true))
                    .key("editor")
            },
            (400.0, 100.0),
            Env::new(true, 1.0),
        );
        runtime.frame(0.0);
        let mut input = Input::default();
        assert!(input.edit(&mut runtime, "editor", "refs/tags/fixture-after"));
        for _ in 0..4 {
            runtime.frame(1.0 / 60.0);
            input.next(&mut runtime);
        }
        runtime.frame(1.0 / 60.0);
        assert_eq!(runtime.state.text, "refs/tags/fixture-after");
    }
}
