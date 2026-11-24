//! Exercise the interactive source popup through framed native Runtime input.
use super::{Driver, Phase, Runtime, State, Step, observation, preview_drawn};
use crate::{camera::Camera, flow_comparison::SideId};
use anyhow::{Context, Result, ensure};
use rune::{Key, KeyEvent, Mods, runtime::ElementInfo};
use std::time::{Duration, Instant};

const PANEL: &str = "comparison-unified-source";
const SCROLL: &str = "comparison-source-scroll";
const RESIZE: &str = "comparison-source-resize";
const PIN: &str = "comparison-source-pin";
const HOVER_HOLD: Duration = Duration::from_millis(380);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::flow_acceptance::comparison::driver) enum Kind {
    Function,
    Class,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::flow_acceptance::comparison::driver) enum Probe {
    Enter,
    Retained,
    Scroll,
    Scrolled,
    Resize,
    Resized,
    Pin,
    Leave,
    Pinned,
    Dismiss,
    Dismissed,
}

pub(in crate::flow_acceptance::comparison::driver) struct PopupObservation {
    cameras: [[f32; 3]; 2],
    scroll_x: f32,
    width: f32,
}

impl Kind {
    pub(super) fn phase(self, probe: Probe) -> Phase {
        Phase::FlowFirst(Step::Popup(self, probe))
    }

    fn cameras(self, state: &State) -> Result<[[f32; 3]; 2]> {
        let camera = |camera: &Camera| [camera.pan[0], camera.pan[1], camera.zoom];
        match self {
            Self::Function => Ok([
                camera(&observation::session(state, SideId::Before)?.camera),
                camera(&observation::session(state, SideId::After)?.camera),
            ]),
            Self::Class => Ok(state
                .flow_comparison
                .class_pair()
                .context("No paired class cameras")?
                .cameras
                .each_ref()
                .map(camera)),
        }
    }
}

impl Driver {
    pub(super) fn popup_action(
        &mut self,
        runtime: &mut Runtime<State>,
        kind: Kind,
        step: Probe,
    ) -> Result<()> {
        match step {
            Probe::Enter => self.move_for_popup(runtime, SCROLL, kind.phase(Probe::Retained)),
            Probe::Scroll => {
                let observed = self.popup.as_ref().context("No popup observation")?;
                let dx = if observed.scroll_x > 80.0 {
                    80.0
                } else {
                    -80.0
                };
                if self.input.scroll_by(runtime, SCROLL, [dx, 0.0]) {
                    self.phase = kind.phase(Probe::Scrolled);
                }
            }
            Probe::Resize => {
                if self.input.drag(runtime, RESIZE, [48.0, 0.0]) {
                    self.phase = kind.phase(Probe::Resized);
                }
            }
            Probe::Pin => self.click(runtime, PIN, kind.phase(Probe::Leave)),
            Probe::Leave => {
                self.move_for_popup(runtime, "comparison-base", kind.phase(Probe::Pinned))
            }
            Probe::Dismiss => {
                runtime.key(KeyEvent {
                    key: Key::Escape,
                    mods: Mods::default(),
                });
                self.phase = kind.phase(Probe::Dismissed);
            }
            _ => {}
        }
        Ok(())
    }

    fn move_for_popup(&mut self, runtime: &mut Runtime<State>, target: &str, next: Phase) {
        if self.input.hover(runtime, target) {
            self.requested = Some(Instant::now());
            self.phase = next;
        }
    }

    pub(super) fn popup_observed(
        &mut self,
        runtime: &Runtime<State>,
        kind: Kind,
        step: Probe,
    ) -> Result<()> {
        let hover_held = self.requested.is_some_and(|at| at.elapsed() >= HOVER_HOLD);
        match step {
            Probe::Retained if hover_held => self.capture_popup(runtime, kind)?,
            Probe::Scrolled => self.popup_scrolled(runtime, kind)?,
            Probe::Resized => self.popup_resized(runtime, kind)?,
            Probe::Pinned if hover_held => {
                ensure!(
                    preview_drawn(runtime, "Leaf(2)"),
                    "Pinned source popup closed on exit"
                );
                self.phase = kind.phase(Probe::Dismiss);
            }
            Probe::Dismissed if popup(&runtime.elements()).is_none() => {
                self.finish_popup(runtime, kind)?;
            }
            _ => (),
        }
        Ok(())
    }

    fn popup_scrolled(&mut self, runtime: &Runtime<State>, kind: Kind) -> Result<()> {
        let observed = self.check_cameras(runtime, kind)?;
        let scroll_x = line_scroll(&runtime.elements())?;
        ensure!(
            (scroll_x - observed.scroll_x).abs() > 8.0,
            "{kind:?} popup did not scroll its drawn diff rows"
        );
        self.phase = kind.phase(Probe::Resize);
        Ok(())
    }

    fn capture_popup(&mut self, runtime: &Runtime<State>, kind: Kind) -> Result<()> {
        ensure!(
            preview_drawn(runtime, "Leaf(2)"),
            "{kind:?} source popup dismissed when the pointer moved into its source body"
        );
        let elements = runtime.elements();
        self.popup = Some(PopupObservation {
            cameras: kind.cameras(&runtime.state)?,
            scroll_x: line_scroll(&elements)?,
            width: popup(&elements).context("No drawn source popup")?.rect[2],
        });
        self.phase = kind.phase(Probe::Scroll);
        Ok(())
    }

    fn check_cameras(&self, runtime: &Runtime<State>, kind: Kind) -> Result<&PopupObservation> {
        let observed = self.popup.as_ref().context("No popup observation")?;
        ensure!(
            kind.cameras(&runtime.state)? == observed.cameras,
            "{kind:?} source popup gesture moved an underlying graph camera"
        );
        Ok(observed)
    }

    fn popup_resized(&mut self, runtime: &Runtime<State>, kind: Kind) -> Result<()> {
        let observed = self.check_cameras(runtime, kind)?;
        let elements = runtime.elements();
        let width = popup(&elements)
            .context("Resize dismissed the source popup")?
            .rect[2];
        ensure!(
            width > observed.width + 16.0,
            "{kind:?} source popup did not grow"
        );
        let (check, next) = match kind {
            Kind::Function => ("function_source_popup_scrolled_and_resized", Probe::Dismiss),
            Kind::Class => ("class_source_popup_scrolled_and_resized", Probe::Pin),
        };
        self.report.check(check);
        self.phase = kind.phase(next);
        Ok(())
    }

    fn finish_popup(&mut self, runtime: &Runtime<State>, kind: Kind) -> Result<()> {
        self.check_cameras(runtime, kind)?;
        self.popup = None;
        self.phase = Phase::FlowFirst(match kind {
            Kind::Function => Step::Focus,
            Kind::Class => {
                self.report.check("source_popup_pinned_and_dismissed");
                Step::FocusClassNeighbor
            }
        });
        Ok(())
    }
}

fn popup(elements: &[ElementInfo]) -> Option<&ElementInfo> {
    elements
        .iter()
        .find(|element| element.key.as_deref() == Some(PANEL) && visible(element))
}

fn line_scroll(elements: &[ElementInfo]) -> Result<f32> {
    elements
        .iter()
        .find(|element| {
            element
                .key
                .as_deref()
                .is_some_and(|key| key.starts_with("comparison-unified-row-"))
                && visible(element)
        })
        .map(|element| element.scrolled[0])
        .context("No drawn source diff rows")
}

fn visible(element: &ElementInfo) -> bool {
    !element.exiting && element.visible[2].min(element.visible[3]) > 1.0
}
