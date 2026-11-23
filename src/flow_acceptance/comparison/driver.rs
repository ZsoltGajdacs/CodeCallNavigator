//! A concrete native workflow; only input handlers may change comparison application state.
mod flow_first;
mod interaction;
mod recovery;
mod selection;
mod worktree;
use super::{
    config::{CaseKind, Configuration},
    observation,
    protocol::{Checkpoint, Operation, Request},
    report::Report,
};
use crate::{
    app::State,
    flow_acceptance::{
        input::Input,
        io::{Reply, Worker},
        report::WindowRequest,
    },
};
use anyhow::{Context, Result, ensure};
use interaction::{Interaction, SavedPair};
use recovery::{Cancellation, RefMove};
use rune::{Runtime, WindowHandle};
use selection::{Editor, Purpose, Selecting};
use std::time::{Duration, Instant};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    WindowSetup,
    Resize,
    Boot,
    Editing(Editor),
    FlowFirst(flow_first::Step),
    Selection(Selecting),
    Interaction(Interaction),
    RefMove(RefMove),
    Cancellation(Cancellation),
    Worktree(worktree::Step),
    ExternalWait,
    Writing,
    Closing,
}
pub(crate) struct Driver {
    config: Configuration,
    phase: Phase,
    input: Input,
    io: Worker,
    report: Report,
    origin: Instant,
    drawn_at: Instant,
    window: Option<WindowHandle>,
    stable_since: Option<Instant>,
    requested: Option<Instant>,
    selected_case: usize,
    purpose: Purpose,
    saved: Option<SavedPair>,
    calls: [Option<crate::flow_acceptance::observation::CallSelection>; 2],
    external_sequence: u32,
    external_pending: Option<(Checkpoint, Phase)>,
}
impl Driver {
    pub fn new(config: Configuration, io: Worker, origin: Instant) -> Self {
        let report = Report::new(&config);
        Self {
            config,
            phase: Phase::WindowSetup,
            input: Input::default(),
            io,
            report,
            origin,
            drawn_at: origin,
            window: None,
            stable_since: None,
            requested: None,
            selected_case: 0,
            purpose: Purpose::Cases,
            saved: None,
            calls: [None, None],
            external_sequence: 0,
            external_pending: None,
        }
    }
    pub fn window_ready(&mut self, window: &WindowHandle) {
        if self.window.is_some() {
            return;
        }
        self.window = Some(window.clone());
        if let Err(error) = self.setup_window(window) {
            self.record_error(format!("{error:#}"));
        }
    }
    fn setup_window(&mut self, window: &WindowHandle) -> Result<()> {
        let id = window
            .window_number()
            .filter(|id| *id > 0)
            .context("Comparison setup requires its owned window ID")?;
        self.report.window_id = Some(id);
        self.io.setup_window(WindowRequest {
            version: 1,
            window_id: id,
            process_id: std::process::id(),
            expected_viewport: self.config.expected_viewport,
        })
    }
    pub fn finished(&self) -> bool {
        self.phase == Phase::Closing
    }
    pub fn encoded(&mut self, frame: &rune::FrameEncoded) {
        self.report.frames.encoded(frame, self.origin);
    }
    pub fn advance(&mut self, runtime: &mut Runtime<State>) {
        while let Some(reply) = self.io.reply() {
            if let Err(error) = self.reply(runtime, reply) {
                self.fail(runtime, format!("{error:#}"));
            }
        }
        if matches!(self.phase, Phase::Writing | Phase::Closing) {
            return;
        }
        if self.report.status == "failed" {
            self.fail(runtime, self.report.error.clone().unwrap_or_default());
            return;
        }
        if self.origin.elapsed() > self.config.timeout() {
            self.fail(
                runtime,
                format!("Comparison acceptance timed out in {:?}", self.phase),
            );
            return;
        }
        self.input.next(runtime);
        if self.input.busy() {
            return;
        }
        if let Err(error) = self.action(runtime) {
            self.fail(runtime, format!("{error:#}"));
        }
    }
    fn action(&mut self, runtime: &mut Runtime<State>) -> Result<()> {
        match self.phase {
            Phase::Editing(step) => self.editor_action(runtime, step),
            Phase::FlowFirst(step) => self.flow_first_action(runtime, step),
            Phase::Selection(step) => self.selection_action(runtime, step),
            Phase::Interaction(step) => self.interaction_action(runtime, step),
            Phase::RefMove(step) => self.ref_move_action(runtime, step),
            Phase::Cancellation(step) => self.cancel_action(runtime, step),
            Phase::Worktree(step) => self.worktree_action(runtime, step),
            _ => Ok(()),
        }
    }
    /// Only the actual native host's Drawn callback reaches this observation path.
    pub fn observed(&mut self, runtime: &Runtime<State>) {
        self.drawn_at = Instant::now();
        self.report.observe(runtime);
        if let Err(error) = self.observe(runtime) {
            self.record_error(format!("{error:#}"));
        }
    }
    fn observe(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if !matches!(
            self.phase,
            Phase::WindowSetup | Phase::Resize | Phase::Writing | Phase::Closing
        ) {
            ensure!(
                self.config
                    .viewport_matches([runtime.viewport.0, runtime.viewport.1]),
                "Comparison window escaped its minimum viewport"
            );
        }
        self.record_ready(runtime)?;
        if self.input.busy() {
            return Ok(());
        }
        match self.phase {
            Phase::Resize => self.resized(runtime),
            Phase::Boot => self.boot(runtime)?,
            Phase::FlowFirst(step) => self.flow_first_observed(runtime, step)?,
            Phase::Selection(step) => self.selection_observed(runtime, step)?,
            Phase::Interaction(step) => self.interaction_observed(runtime, step)?,
            Phase::RefMove(step) => self.ref_move_observed(runtime, step)?,
            Phase::Cancellation(step) => self.cancel_observed(runtime, step)?,
            Phase::Worktree(step) => self.worktree_observed(runtime, step)?,
            _ => (),
        }
        Ok(())
    }
    fn record_ready(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if self.phase == Phase::Interaction(Interaction::Target) {
            if observation::pair(runtime, &self.config.cases[6])?.is_some() {
                self.report.ready("Target", self.requested, self.drawn_at);
            }
            return Ok(());
        }
        if self.phase != Phase::Selection(Selecting::Pair) {
            return Ok(());
        }
        let case = &self.config.cases[self.selected_case];
        if observation::pair(runtime, case)?.is_some() {
            self.report.ready(
                &format!("{:?}-{:?}", case.kind, self.purpose),
                self.requested,
                self.drawn_at,
            );
        }
        Ok(())
    }
    fn resized(&mut self, runtime: &Runtime<State>) {
        if !self
            .config
            .viewport_matches([runtime.viewport.0, runtime.viewport.1])
        {
            self.stable_since = None;
            return;
        }
        if self.stable_since.get_or_insert_with(Instant::now).elapsed()
            >= Duration::from_millis(250)
        {
            self.report.check("minimum_viewport_drawn");
            self.phase = Phase::Boot;
        }
    }
    fn boot(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if runtime.state.runtime.is_none() {
            return Ok(());
        }
        ensure!(
            !runtime.state.uses_fixture_queries(),
            "Comparison probe used a fixture backend"
        );
        ensure!(
            runtime.state.graph.is_none(),
            "Comparison native acceptance requires no current graph"
        );
        ensure!(
            runtime.state.root.text == self.config.project.to_string_lossy(),
            "Comparison boot selected another project"
        );
        self.report.check("normal_no_graph_boot");
        self.phase = Phase::Editing(Editor::Entry);
        Ok(())
    }
    fn click(&mut self, runtime: &mut Runtime<State>, key: &str, next: Phase) {
        if self.input.click(runtime, key) {
            self.phase = next;
            self.requested = Some(Instant::now());
        }
    }
    fn external(
        &mut self,
        operation: Operation,
        checkpoint: Checkpoint,
        next: Phase,
    ) -> Result<()> {
        self.external_sequence += 1;
        let capture = operation == Operation::Capture;
        let request = Request {
            version: 2,
            sequence: self.external_sequence,
            operation,
            checkpoint,
            project: self.config.project.clone(),
            window_id: if capture { self.report.window_id } else { None },
            comparison_id: if capture {
                Some(self.expected_binding().comparison_id.clone())
            } else {
                None
            },
        };
        self.io.external(request)?;
        self.external_pending = Some((checkpoint, next));
        self.phase = Phase::ExternalWait;
        Ok(())
    }
    fn reply(&mut self, runtime: &mut Runtime<State>, reply: Reply) -> Result<()> {
        match reply {
            Reply::Finished(result) => {
                if self.phase == Phase::Closing {
                    return Ok(());
                }
                if let Err(error) = result {
                    let message = format!("Cannot save comparison report: {error:#}");
                    eprintln!("{message}");
                    self.record_error(message);
                }
                runtime.update(crate::jobs::close);
                self.phase = Phase::Closing;
            }
            _ if matches!(self.phase, Phase::Writing | Phase::Closing) => (),

            Reply::WindowSetup(result) if self.phase == Phase::WindowSetup => {
                result?;
                let window = self
                    .window
                    .as_ref()
                    .context("Comparison setup lost its owned window")?;
                ensure!(
                    window.window_number() == self.report.window_id,
                    "Comparison setup changed window ownership"
                );
                let [w, h] = self.config.expected_viewport;
                window.resize_instant((w as f32, h as f32));
                self.report.check("owned_window_setup_acknowledged");
                self.phase = Phase::Resize;
            }
            Reply::External { sequence, result } => {
                ensure!(
                    sequence == self.external_sequence && self.phase == Phase::ExternalWait,
                    "Unexpected comparison external reply"
                );
                result?;
                let (checkpoint, next) = self
                    .external_pending
                    .take()
                    .context("Missing external continuation")?;
                if matches!(
                    checkpoint,
                    Checkpoint::UnavailableFlow | Checkpoint::ManualPair | Checkpoint::RestoredPair
                ) {
                    self.report.captures.push(checkpoint);
                }
                self.phase = next;
            }
            _ => anyhow::bail!("Unexpected native comparison worker reply"),
        }
        Ok(())
    }
    fn expected_binding(&self) -> &super::config::Binding {
        if matches!(self.purpose, Purpose::Reloaded | Purpose::Fresh) {
            &self.config.reloaded
        } else {
            &self.config.initial
        }
    }
    fn record_error(&mut self, error: String) {
        self.report.status = "failed".into();
        self.report.error = Some(error);
    }
    fn fail(&mut self, runtime: &mut Runtime<State>, error: String) {
        if matches!(self.phase, Phase::Writing | Phase::Closing) {
            return;
        }
        self.io.cancel_wait();
        self.record_error(error);
        if let Err(error) = self.finish(false) {
            eprintln!("comparison acceptance: {error:#}");
            runtime.update(crate::jobs::close);
            self.phase = Phase::Closing;
        }
    }
    fn finish(&mut self, passed: bool) -> Result<()> {
        if matches!(self.phase, Phase::Writing | Phase::Closing) {
            return Ok(());
        }
        self.report.status = if passed { "passed" } else { "failed" }.into();
        self.io.finish_comparison(self.report.clone())?;
        self.phase = Phase::Writing;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
