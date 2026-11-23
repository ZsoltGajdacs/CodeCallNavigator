//! Independent endpoint gestures and target/Back restoration use ordinary controls.
use super::*;
use crate::{camera::Camera, flow_comparison::SideId, flow_workspace::Session};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Interaction {
    StartManual,
    BeforeSelect,
    BeforeSelected,
    BeforePan,
    BeforePanned,
    BeforeScroll,
    BeforeScrolled,
    AfterSelect,
    AfterSelected,
    AfterPan,
    AfterPanned,
    AfterScroll,
    AfterScrolled,
    Follow,
    Target,
    Back,
    Restored,
    BeforeTab,
    BeforeRestored,
    AfterTab,
    AfterRestored,
}
#[derive(Clone)]
struct SavedSide {
    camera: Camera,
    scroll: [f32; 2],
    selected: Option<usize>,
}
impl SavedSide {
    fn capture(session: &Session) -> Self {
        Self {
            camera: session.camera.clone(),
            scroll: session.scroll.get(),
            selected: session.selected,
        }
    }
    fn camera_matches(&self, session: &Session) -> bool {
        near(self.camera.pan, session.camera.pan)
            && (self.camera.zoom - session.camera.zoom).abs() < 0.001
    }
    fn matches(&self, session: &Session) -> bool {
        self.camera_matches(session)
            && near(self.scroll, session.scroll.get())
            && self.selected == session.selected
    }
}
pub(super) struct SavedPair {
    sides: [SavedSide; 2],
    history: usize,
}
impl SavedPair {
    fn capture(state: &State) -> Result<Self> {
        Ok(Self {
            sides: [
                SavedSide::capture(observation::session(state, SideId::Before)?),
                SavedSide::capture(observation::session(state, SideId::After)?),
            ],
            history: state.flow_comparison.history.len(),
        })
    }
}
fn near(a: [f32; 2], b: [f32; 2]) -> bool {
    a.into_iter().zip(b).all(|(x, y)| (x - y).abs() < 1.0)
}
impl Driver {
    pub(super) fn interaction_action(
        &mut self,
        runtime: &mut Runtime<State>,
        step: Interaction,
    ) -> Result<()> {
        match step {
            Interaction::StartManual => self.select_case(0, Purpose::ManualBase),
            Interaction::BeforeSelect => {
                self.select_call(runtime, SideId::Before, Interaction::BeforeSelected)?
            }
            Interaction::AfterSelect => {
                self.select_call(runtime, SideId::After, Interaction::AfterSelected)?
            }
            Interaction::BeforePan => self.pan(runtime, SideId::Before, Interaction::BeforePanned),
            Interaction::AfterPan => self.pan(runtime, SideId::After, Interaction::AfterPanned),
            Interaction::BeforeScroll => {
                self.scroll(runtime, SideId::Before, Interaction::BeforeScrolled)?
            }
            Interaction::AfterScroll => {
                self.scroll(runtime, SideId::After, Interaction::AfterScrolled)?
            }
            Interaction::Follow => {
                let call = self.calls[1].context("No observed After target")?;
                self.click(
                    runtime,
                    &format!("comparison-after-target-{}-{}", call.call, call.target),
                    Phase::Interaction(Interaction::Target),
                );
            }
            Interaction::Back => self.click(
                runtime,
                "comparison-back",
                Phase::Interaction(Interaction::Restored),
            ),
            Interaction::BeforeTab => self.click(
                runtime,
                "comparison-source-before",
                Phase::Interaction(Interaction::BeforeRestored),
            ),
            Interaction::AfterTab => self.click(
                runtime,
                "comparison-source-after",
                Phase::Interaction(Interaction::AfterRestored),
            ),
            _ => (),
        }
        Ok(())
    }
    fn select_call(
        &mut self,
        runtime: &mut Runtime<State>,
        side: SideId,
        next: Interaction,
    ) -> Result<()> {
        let session = observation::session(&runtime.state, side)?;
        let call =
            crate::flow_acceptance::observation::target(session, &self.config.cases[6].symbol)?;
        self.calls[side.index()] = Some(call);
        let key = observation::address(side).key(&format!("step-{}", call.step));
        self.click(runtime, &key, Phase::Interaction(next));
        Ok(())
    }
    fn pan(&mut self, runtime: &mut Runtime<State>, side: SideId, next: Interaction) {
        let address = observation::address(side);
        if self
            .input
            .pan_canvas(runtime, &address.key("canvas"), &address.key("step-"))
        {
            self.phase = Phase::Interaction(next);
        }
    }
    fn scroll(
        &mut self,
        runtime: &mut Runtime<State>,
        side: SideId,
        next: Interaction,
    ) -> Result<()> {
        let offset = observation::session(&runtime.state, side)?.scroll.get()[1];
        if self.input.scroll_pane(
            runtime,
            &observation::address(side).key("source-pane"),
            offset,
        ) {
            self.phase = Phase::Interaction(next);
        }
        Ok(())
    }
    pub(super) fn interaction_observed(
        &mut self,
        runtime: &Runtime<State>,
        step: Interaction,
    ) -> Result<()> {
        match step {
            Interaction::BeforeSelected => {
                self.selected(runtime, SideId::Before, Interaction::BeforePan)?
            }
            Interaction::AfterSelected => {
                self.selected(runtime, SideId::After, Interaction::AfterPan)?
            }
            Interaction::BeforePanned => {
                self.panned(runtime, SideId::Before, Interaction::BeforeScroll)?
            }
            Interaction::AfterPanned => {
                self.panned(runtime, SideId::After, Interaction::AfterScroll)?
            }
            Interaction::BeforeScrolled => {
                self.scrolled(runtime, SideId::Before, Interaction::AfterSelect)?
            }
            Interaction::AfterScrolled => {
                self.scrolled(runtime, SideId::After, Interaction::Follow)?
            }
            Interaction::Target => self.target_observed(runtime)?,
            Interaction::Restored => self.back_observed(runtime)?,
            Interaction::BeforeRestored => {
                self.tab_restored(runtime, SideId::Before, Interaction::AfterTab)?
            }
            Interaction::AfterRestored if self.source_restored(runtime, SideId::After)? => {
                self.external(
                    Operation::Capture,
                    Checkpoint::RestoredPair,
                    Phase::RefMove(RefMove::Move),
                )?;
            }
            _ => (),
        }
        Ok(())
    }
    fn selected(
        &mut self,
        runtime: &Runtime<State>,
        side: SideId,
        next: Interaction,
    ) -> Result<()> {
        let session = observation::session(&runtime.state, side)?;
        if session.selected != self.calls[side.index()].map(|call| call.step) {
            return Ok(());
        }
        if !observation::highlighted(runtime, side)? || !observation::source_drawn(runtime, side)? {
            return Ok(());
        }
        if side == SideId::After {
            self.other_unchanged(&runtime.state, side)?;
        }
        self.report.check(&format!("{side:?}_highlight_drawn"));
        self.saved = Some(SavedPair::capture(&runtime.state)?);
        self.phase = Phase::Interaction(next);
        Ok(())
    }
    fn other_unchanged(&self, state: &State, side: SideId) -> Result<()> {
        let other = match side {
            SideId::Before => SideId::After,
            SideId::After => SideId::Before,
        };
        let saved = self
            .saved
            .as_ref()
            .context("Missing independent pane observation")?;
        ensure!(
            saved.sides[other.index()].matches(observation::session(state, other)?),
            "Gesture changed the other endpoint's camera/selection/source offset"
        );
        Ok(())
    }
    fn panned(&mut self, runtime: &Runtime<State>, side: SideId, next: Interaction) -> Result<()> {
        let session = observation::session(&runtime.state, side)?;
        let saved = &self.saved.as_ref().context("No pre-pan observation")?.sides[side.index()];
        ensure!(
            !saved.camera_matches(session),
            "Native endpoint canvas drag did not change its camera"
        );
        ensure!(
            near(saved.scroll, session.scroll.get()),
            "Canvas drag moved source scroll"
        );
        self.other_unchanged(&runtime.state, side)?;
        self.report.check(&format!("{side:?}_camera_independent"));
        self.saved = Some(SavedPair::capture(&runtime.state)?);
        self.phase = Phase::Interaction(next);
        Ok(())
    }
    fn scrolled(
        &mut self,
        runtime: &Runtime<State>,
        side: SideId,
        next: Interaction,
    ) -> Result<()> {
        let session = observation::session(&runtime.state, side)?;
        let saved = &self
            .saved
            .as_ref()
            .context("No pre-scroll observation")?
            .sides[side.index()];
        ensure!(
            !near(saved.scroll, session.scroll.get()),
            "Native endpoint source wheel did not scroll"
        );
        ensure!(
            saved.camera_matches(session),
            "Source scroll changed the endpoint camera"
        );
        self.other_unchanged(&runtime.state, side)?;
        if !observation::source_drawn(runtime, side)? {
            return Ok(());
        }
        self.report.check(&format!("{side:?}_scroll_independent"));
        self.saved = Some(SavedPair::capture(&runtime.state)?);
        self.phase = Phase::Interaction(next);
        Ok(())
    }
    fn target_observed(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if observation::pair(runtime, &self.config.cases[6])?.is_none() {
            return Ok(());
        }
        ensure!(
            runtime.state.flow_comparison.history.len() == self.saved.as_ref().unwrap().history + 1,
            "Target did not preserve caller Pair history"
        );
        self.report.ready("Target", self.requested, self.drawn_at);
        self.report.check("endpoint_target_followed");
        self.phase = Phase::Interaction(Interaction::Back);
        Ok(())
    }
    fn back_observed(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if observation::pair(runtime, &self.config.cases[0])?.is_none() {
            return Ok(());
        }
        let saved = self.saved.as_ref().context("No saved caller Pair")?;
        for side in [SideId::Before, SideId::After] {
            ensure!(
                saved.sides[side.index()].matches(observation::session(&runtime.state, side)?),
                "Back changed endpoint camera/selection/source offset"
            );
        }
        ensure!(
            runtime
                .state
                .flow_comparison
                .current
                .as_ref()
                .unwrap()
                .active_source
                == SideId::After,
            "Back changed active source tab"
        );
        if observation::source_drawn(runtime, SideId::After)? {
            self.report.check("back_pair_restored");
            self.phase = Phase::Interaction(Interaction::BeforeTab);
        }
        Ok(())
    }
    fn source_restored(&mut self, runtime: &Runtime<State>, side: SideId) -> Result<bool> {
        if !observation::source_drawn(runtime, side)? {
            return Ok(false);
        }
        ensure!(
            self.saved
                .as_ref()
                .context("No saved source observation")?
                .sides[side.index()]
            .matches(observation::session(&runtime.state, side)?),
            "Hidden source tab lost its saved offset"
        );
        self.report
            .check(&format!("{side:?}_hidden_scroll_restored"));
        Ok(true)
    }
    fn tab_restored(
        &mut self,
        runtime: &Runtime<State>,
        side: SideId,
        next: Interaction,
    ) -> Result<()> {
        if self.source_restored(runtime, side)? {
            self.phase = Phase::Interaction(next);
        }
        Ok(())
    }
}
