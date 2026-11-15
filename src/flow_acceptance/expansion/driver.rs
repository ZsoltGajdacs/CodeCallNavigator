//! A fixed native journey queues real input, then advances only on a matching Drawn observation.
use super::{
    config::Configuration,
    observation::{self, Check, Evidence, Select},
    report::{ReadySample, Report},
};
use crate::{
    app::State,
    flow_acceptance::{
        input::Input,
        io::{Reply, Worker},
        observation::CallSelection,
        report::{CaptureRequest, WindowRequest},
    },
};
use anyhow::{Context, Result, ensure};
use rune::{FrameEncoded, Runtime, WindowHandle};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

enum Action {
    Click(&'static str),
    Select(Select),
    Expand,
    Follow,
    Wait(Check),
    Pan,
    Scroll,
    Capture,
    Finish,
}
#[derive(PartialEq)]
enum Phase {
    WindowSetup,
    Resize,
    Running,
    Capture,
    Writing,
    Closing,
}
struct PendingSample {
    operation: &'static str,
    started: Instant,
    previous: usize,
}
pub(in crate::flow_acceptance) struct Driver {
    config: Configuration,
    worker: Worker,
    report: Report,
    input: Input,
    evidence: Evidence,
    actions: VecDeque<Action>,
    phase: Phase,
    origin: Instant,
    window: Option<WindowHandle>,
    stable_since: Option<Instant>,
    selected: Option<CallSelection>,
    sample: Option<PendingSample>,
}
impl Driver {
    pub fn new(config: Configuration, worker: Worker, origin: Instant) -> Self {
        let report = Report::new(&config);
        Self {
            config,
            worker,
            report,
            input: Input::default(),
            evidence: Evidence::default(),
            actions: journey(),
            phase: Phase::WindowSetup,
            origin,
            window: None,
            stable_since: None,
            selected: None,
            sample: None,
        }
    }
    pub fn window_ready(&mut self, window: &WindowHandle) {
        if self.window.is_some() {
            return;
        }
        self.window = Some(window.clone());
        if let Err(error) = self.setup_window(window) {
            self.record_error(error);
        }
    }
    fn setup_window(&mut self, window: &WindowHandle) -> Result<()> {
        let id = window
            .window_number()
            .filter(|id| *id > 0)
            .context("Expansion probe window has no registered identity")?;
        self.report.window_id = Some(id);
        self.report.window_key = Some(window.key().into());
        self.worker.setup_window(WindowRequest {
            version: 1,
            window_id: id,
            process_id: std::process::id(),
            expected_viewport: self.config.expected_viewport,
        })
    }
    pub fn finished(&self) -> bool {
        self.phase == Phase::Closing
    }
    pub fn encoded(&mut self, frame: &FrameEncoded) {
        self.report.frames.encoded(frame, self.origin);
    }
    pub fn advance(&mut self, runtime: &mut Runtime<State>) {
        self.replies(runtime);
        if matches!(self.phase, Phase::Writing | Phase::Closing) {
            return;
        }
        if self.origin.elapsed() > self.config.timeout() {
            self.record_error(anyhow::anyhow!(
                "Expansion native journey timed out with {} remaining actions",
                self.actions.len()
            ));
        }
        if self.report.error.is_some() {
            self.finish(runtime, false);
            return;
        }
        self.input.next(runtime);
        if self.input.busy() || self.phase != Phase::Running {
            return;
        }
        if let Err(error) = self.action(runtime) {
            self.record_error(error);
        }
    }
    fn action(&mut self, runtime: &mut Runtime<State>) -> Result<()> {
        self.check_viewport(runtime)?;
        let Some(action) = self.actions.pop_front() else {
            anyhow::bail!("Expansion journey ended without terminal assertion");
        };
        let measurement = match &action {
            Action::Click(
                key @ ("flow-expand-mode" | "flow-collapse-occurrence" | "flow-collapse-all"),
            ) => Some(*key),
            Action::Expand => Some("expand-call"),
            _ => None,
        };
        let sample = measurement.and_then(|operation| {
            let ready = runtime
                .state
                .flow_workspace
                .current
                .as_ref()?
                .ready
                .as_ref()?;
            Some(PendingSample {
                operation,
                started: Instant::now(),
                previous: std::sync::Arc::as_ptr(ready) as usize,
            })
        });
        let done = match action {
            Action::Click(key) => self.input.click(runtime, key),
            Action::Select(select) => self.select(runtime, select)?,
            Action::Expand => self.call_control(runtime, "flow-expand")?,
            Action::Follow => self.call_control(runtime, "flow-target")?,
            Action::Pan => self.input.pan(runtime),
            Action::Scroll => {
                let offset = observation::session(runtime)?.scroll.get()[1];
                self.input.scroll_source(runtime, offset)
            }
            Action::Capture => {
                self.capture(runtime)?;
                true
            }
            Action::Finish => {
                self.finish(runtime, true);
                true
            }
            Action::Wait(_) => false,
        };
        if !done {
            self.actions.push_front(action);
        } else if sample.is_some() {
            self.sample = sample;
        }
        Ok(())
    }
    fn select(&mut self, runtime: &mut Runtime<State>, select: Select) -> Result<bool> {
        let call = select(&self.evidence, observation::session(runtime)?, &self.config)?;
        let clicked = self
            .input
            .click(runtime, &format!("flow-step-{}", call.step));
        if clicked {
            self.selected = Some(call);
        }
        Ok(clicked)
    }
    fn call_control(&mut self, runtime: &mut Runtime<State>, prefix: &str) -> Result<bool> {
        let call = self.selected.context("No selected compiler call slot")?;
        Ok(self
            .input
            .click(runtime, &format!("{prefix}-{}-{}", call.call, call.target)))
    }
    pub fn observed(&mut self, runtime: &Runtime<State>) {
        self.report.observe(runtime);
        self.observe_readiness(runtime);
        if self.input.busy() || matches!(self.phase, Phase::Writing | Phase::Closing) {
            return;
        }
        if let Err(error) = self.observe(runtime) {
            self.record_error(error);
        }
    }
    fn observe_readiness(&mut self, runtime: &Runtime<State>) {
        let Some(sample) = &self.sample else {
            return;
        };
        let Some(session) = runtime.state.flow_workspace.current.as_ref() else {
            return;
        };
        let Some(ready) = &session.ready else {
            return;
        };
        if session.symbol != self.config.root_symbol
            || std::sync::Arc::as_ptr(ready) as usize == sample.previous
        {
            return;
        }
        self.report.ready_drawn.push(ReadySample {
            operation: sample.operation,
            milliseconds: sample.started.elapsed().as_secs_f64() * 1000.0,
            steps: ready.chart.steps.len(),
            links: ready.chart.links.len(),
            query_milliseconds: session
                .expansion_timings()
                .map(|(query, _)| query.as_secs_f64() * 1000.0),
            layout_milliseconds: session
                .expansion_timings()
                .map(|(_, layout)| layout.as_secs_f64() * 1000.0),
        });
        self.sample = None;
    }
    fn observe(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if self.phase == Phase::Resize {
            self.resized(runtime);
            return Ok(());
        }
        if self.phase != Phase::Running {
            return Ok(());
        }
        self.check_viewport(runtime)?;
        if runtime.state.query_context.is_some() {
            ensure!(
                runtime.state.generation.as_deref() == Some(self.config.generation.as_str()),
                "Native expansion adopted another artifact generation"
            );
            self.report.check("generation_unchanged");
        }
        let Some(Action::Wait(check)) = self.actions.front() else {
            return Ok(());
        };
        if check(&mut self.evidence, runtime, &self.config, &mut self.report)? {
            self.actions.pop_front();
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
            self.phase = Phase::Running;
        }
    }
    fn check_viewport(&self, runtime: &Runtime<State>) -> Result<()> {
        ensure!(
            self.config
                .viewport_matches([runtime.viewport.0, runtime.viewport.1]),
            "Expansion native viewport differs from required minimum"
        );
        Ok(())
    }
    fn capture(&mut self, runtime: &Runtime<State>) -> Result<()> {
        let window = self.window.as_ref().context("No owned capture window")?;
        let window_id = window
            .window_number()
            .filter(|id| Some(*id) == self.report.window_id)
            .context("Capture window changed identity")?;
        self.worker.capture(CaptureRequest {
            version: 1,
            checkpoint: "restored-expanded-root",
            window_id,
            symbol: self.config.root_symbol.clone(),
            generation: self.config.generation.clone(),
            expected_viewport: self.config.expected_viewport,
            viewport: [runtime.viewport.0, runtime.viewport.1],
        })?;
        self.phase = Phase::Capture;
        Ok(())
    }
    fn replies(&mut self, runtime: &mut Runtime<State>) {
        while let Some(reply) = self.worker.reply() {
            if let Err(error) = self.reply(runtime, reply) {
                self.record_error(error);
            }
        }
    }
    fn reply(&mut self, runtime: &mut Runtime<State>, reply: Reply) -> Result<()> {
        match reply {
            Reply::WindowSetup(result) if self.phase == Phase::WindowSetup => {
                result?;
                let window = self
                    .window
                    .as_ref()
                    .context("Owned setup window disappeared")?;
                ensure!(
                    window.window_number() == self.report.window_id,
                    "Setup acknowledgment belongs to another window"
                );
                let [width, height] = self.config.expected_viewport;
                window.resize_instant((width as f32, height as f32));
                self.report.check("owned_window_setup_acknowledged");
                self.phase = Phase::Resize;
            }
            Reply::Capture(result) if self.phase == Phase::Capture => {
                result?;
                self.report.captures.push("restored-expanded-root".into());
                self.phase = Phase::Running;
            }
            Reply::Finished(result) => {
                if let Err(error) = result {
                    eprintln!("Cannot save expansion native report: {error:#}");
                }
                runtime.update(crate::jobs::close);
                self.phase = Phase::Closing;
            }
            _ => anyhow::bail!("Unexpected native acceptance worker reply"),
        }
        Ok(())
    }
    fn record_error(&mut self, error: anyhow::Error) {
        self.report
            .error
            .get_or_insert_with(|| format!("{error:#}"));
    }
    fn finish(&mut self, runtime: &mut Runtime<State>, passed: bool) {
        if !passed {
            self.worker.cancel_wait();
        }
        self.report.status = if passed { "passed" } else { "failed" }.into();
        match self.worker.finish_expansion(self.report.clone()) {
            Ok(()) => self.phase = Phase::Writing,
            Err(error) => {
                eprintln!("Cannot queue expansion native report: {error:#}");
                runtime.update(crate::jobs::close);
                self.phase = Phase::Closing;
            }
        }
    }
}
fn journey() -> VecDeque<Action> {
    use Action::*;
    use observation as o;
    VecDeque::from([
        Wait(o::boot),
        Click("open-flowchart"),
        Wait(o::root),
        Click("flow-expand-mode"),
        Wait(o::bootstrap),
        Select(o::first_call),
        Expand,
        Wait(o::first),
        Select(o::first_child),
        Wait(o::child_source),
        Expand,
        Wait(o::cycle),
        Select(o::marker),
        Wait(o::cycle_visible),
        Select(o::second_call),
        Expand,
        Wait(o::both),
        Select(o::first_entry),
        Wait(o::child_source),
        Click("flow-collapse-occurrence"),
        Wait(o::one),
        Select(o::first_child),
        Wait(o::child_source),
        Wait(o::before_pan),
        Pan,
        Wait(o::panned),
        Scroll,
        Wait(o::scrolled),
        Follow,
        Wait(o::target),
        Click("flow-back"),
        Wait(o::restored),
        Capture,
        Click("flow-collapse-all"),
        Wait(o::ordinary),
        Click("flow-close"),
        Wait(o::graph),
        Finish,
    ])
}
