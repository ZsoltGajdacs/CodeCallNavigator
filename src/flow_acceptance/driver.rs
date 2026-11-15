//! One bounded native workflow; graph semantics still come exclusively from the CLI.
use super::{
    config::Ordinary as Configuration,
    input::Input,
    io::{Reply, Worker},
    observation::{self, CallSelection},
    report::{CaptureRequest, Report, WindowRequest},
};
use crate::{app::State, camera::Camera, flow_workspace::Session};
use anyhow::{Context, Result, ensure};
use rune::{Runtime, WindowHandle};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    WindowSetup,
    Resize,
    Boot,
    Open,
    Root,
    SelectCall,
    Call,
    SelectSynthetic,
    Synthetic,
    Pan,
    Panned,
    Scroll,
    Scrolled,
    Follow,
    Target,
    Back,
    Restored,
    CaptureHold,
    CaptureWait,
    CloseFlow,
    Graph,
    Writing,
    Closing,
}

#[derive(Default)]
struct ViewportStability {
    since: Option<Instant>,
}

impl ViewportStability {
    fn observe(&mut self, matches: bool, now: Instant) -> bool {
        if !matches {
            self.since = None;
            return false;
        }
        now.saturating_duration_since(*self.since.get_or_insert(now)) >= Duration::from_millis(250)
    }
}

#[derive(Clone)]
struct SavedFlow {
    camera: Camera,
    scroll: [f32; 2],
    selected: Option<usize>,
}

impl SavedFlow {
    fn capture(session: &Session) -> Self {
        Self {
            camera: session.camera.clone(),
            scroll: session.scroll.get(),
            selected: session.selected,
        }
    }
    fn restored(&self, session: &Session) -> bool {
        close(self.camera.pan, session.camera.pan)
            && (self.camera.zoom - session.camera.zoom).abs() < 0.001
            && close(self.scroll, session.scroll.get())
            && self.selected == session.selected
    }
}

pub(super) struct Driver {
    configuration: Configuration,
    phase: Phase,
    input: Input,
    io: Worker,
    report: Rc<RefCell<Report>>,
    origin: Instant,
    requested: Option<Instant>,
    root_navigation: Option<serde_json::Value>,
    call: Option<CallSelection>,
    synthetic: Option<usize>,
    synthetic_considered: bool,
    synthetic_scroll: [f32; 2],
    before_pan: Option<SavedFlow>,
    after_pan: Option<SavedFlow>,
    before_follow: Option<SavedFlow>,
    captured_since: Option<Instant>,
    window: Option<WindowHandle>,
    viewport_stability: ViewportStability,
}

impl Driver {
    pub fn new(
        configuration: Configuration,
        io: Worker,
        report: Rc<RefCell<Report>>,
        origin: Instant,
    ) -> Self {
        Self {
            configuration,
            phase: Phase::WindowSetup,
            input: Input::default(),
            io,
            report,
            origin,
            requested: None,
            root_navigation: None,
            call: None,
            synthetic: None,
            synthetic_considered: false,
            synthetic_scroll: [0.0; 2],
            before_pan: None,
            after_pan: None,
            before_follow: None,
            captured_since: None,
            window: None,
            viewport_stability: ViewportStability::default(),
        }
    }

    pub fn window_ready(&mut self, window: &WindowHandle) {
        if self.window.is_some() {
            return;
        }
        self.window = Some(window.clone());
        if let Err(error) = self.request_window_setup(window) {
            self.record_error(format!("{error:#}"));
        }
    }

    fn request_window_setup(&mut self, window: &WindowHandle) -> Result<()> {
        let window_id = window
            .window_number()
            .filter(|id| *id > 0)
            .context("Native setup requires the owned window's registered ID")?;
        {
            let mut report = self.report.borrow_mut();
            report.window_key = Some(window.key().into());
            report.window_id = Some(window_id);
        }
        self.io.setup_window(WindowRequest {
            version: 1,
            window_id,
            process_id: std::process::id(),
            expected_viewport: self.configuration.expected_viewport,
        })
    }
    pub fn encoded(&mut self, frame: &rune::FrameEncoded) {
        self.report.borrow_mut().encoded(frame, self.origin);
    }
    pub fn finished(&self) -> bool {
        self.phase == Phase::Closing
    }

    pub fn advance(&mut self, runtime: &mut Runtime<State>) {
        self.replies(runtime);
        if matches!(self.phase, Phase::Writing | Phase::Closing) {
            return;
        }
        if self.origin.elapsed() > self.configuration.timeout() {
            self.fail(
                runtime,
                format!("Native flow acceptance timed out in {:?}", self.phase),
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
        if !matches!(self.phase, Phase::WindowSetup | Phase::Resize) {
            self.check_viewport(runtime)?;
        }
        match self.phase {
            Phase::Open => self.click(runtime, "open-flowchart", Phase::Root),
            Phase::SelectCall => self.click(
                runtime,
                &format!("flow-step-{}", self.call.expect("observed call").step),
                Phase::Call,
            ),
            Phase::SelectSynthetic => self.click(
                runtime,
                &format!(
                    "flow-step-{}",
                    self.synthetic.expect("observed synthetic step")
                ),
                Phase::Synthetic,
            ),
            Phase::Pan if self.input.pan(runtime) => self.phase = Phase::Panned,
            Phase::Scroll => {
                let offset = self
                    .before_pan
                    .as_ref()
                    .expect("observed root before pan")
                    .scroll[1];
                if self.input.scroll_source(runtime, offset) {
                    self.phase = Phase::Scrolled;
                }
            }
            Phase::Follow => {
                let call = self.call.expect("observed call");
                self.click(
                    runtime,
                    &format!("flow-target-{}-{}", call.call, call.target),
                    Phase::Target,
                );
            }
            Phase::Back => self.click(runtime, "flow-back", Phase::Restored),
            Phase::CloseFlow => self.click(runtime, "flow-close", Phase::Graph),
            _ => (),
        }
        Ok(())
    }

    fn click(&mut self, runtime: &mut Runtime<State>, key: &str, next: Phase) {
        if self.input.click(runtime, key) {
            self.requested = Some(Instant::now());
            self.phase = next;
        }
    }

    /// Called only by App::on_frame_drawn, after the native host presented this frame.
    pub fn observed(&mut self, runtime: &Runtime<State>) {
        self.record_ready_drawn(runtime);
        if self.input.busy() || matches!(self.phase, Phase::Writing | Phase::Closing) {
            return;
        }
        self.report.borrow_mut().observe(runtime);
        if let Err(error) = self.observe(runtime) {
            // Runtime mutation/normal close is deferred to the compositor's next frame.
            self.record_error(format!("{error:#}"));
        }
    }

    fn record_error(&self, message: String) {
        let mut report = self.report.borrow_mut();
        report.error = Some(message);
        report.status = "failed".into();
    }

    fn record_ready_drawn(&mut self, runtime: &Runtime<State>) {
        let (symbol, sample) = match self.phase {
            Phase::Root => (&self.configuration.root_symbol, true),
            Phase::Target => (&self.configuration.target_symbol, false),
            _ => return,
        };
        if !runtime
            .state
            .flow_workspace
            .current
            .as_ref()
            .is_some_and(|session| session.symbol == *symbol && session.ready.is_some())
        {
            return;
        }
        let Some(milliseconds) = self.elapsed_request() else {
            return;
        };
        let mut report = self.report.borrow_mut();
        let timing = if sample {
            &mut report.timings.root_click_to_ready_drawn_ms
        } else {
            &mut report.timings.target_click_to_ready_drawn_ms
        };
        timing.get_or_insert(milliseconds);
    }

    fn observe(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if !matches!(self.phase, Phase::WindowSetup | Phase::Resize) {
            self.check_viewport(runtime)?;
        }
        match self.phase {
            Phase::Resize => self.resized(runtime),
            Phase::Boot => self.boot(runtime)?,
            Phase::Root => self.root(runtime)?,
            Phase::Call => self.selected_call(runtime)?,
            Phase::Synthetic => self.selected_synthetic(runtime)?,
            Phase::Panned => self.panned(runtime)?,
            Phase::Scrolled => self.scrolled(runtime)?,
            Phase::Target => self.target(runtime)?,
            Phase::Restored => self.restored(runtime)?,
            Phase::CaptureHold => self.capture(runtime)?,
            Phase::Graph => self.graph(runtime)?,
            _ => (),
        }
        Ok(())
    }

    fn resized(&mut self, runtime: &Runtime<State>) {
        let matches = self
            .configuration
            .viewport_matches([runtime.viewport.0, runtime.viewport.1]);
        if self.viewport_stability.observe(matches, Instant::now()) {
            self.report.borrow_mut().checks.viewport_ready = true;
            self.phase = Phase::Boot;
        }
    }

    fn check_viewport(&self, runtime: &Runtime<State>) -> Result<()> {
        ensure!(
            self.configuration
                .viewport_matches([runtime.viewport.0, runtime.viewport.1]),
            "Native viewport {:?} differs from required {:?}",
            runtime.viewport,
            self.configuration.expected_viewport
        );
        Ok(())
    }

    fn boot(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if !observation::boot(&runtime.state, &self.configuration)? {
            return Ok(());
        }
        self.root_navigation = Some(serde_json::to_value(&runtime.state.nav)?);
        self.phase = Phase::Open;
        Ok(())
    }

    fn root(&mut self, runtime: &Runtime<State>) -> Result<()> {
        let Some(session) = observation::session(&runtime.state, &self.configuration.root_symbol)?
        else {
            return Ok(());
        };
        let ready = session.ready.as_ref().expect("observed ready root");
        ensure!(
            ready.source.text == self.configuration.source,
            "Native chart did not display the exact analyzed source"
        );
        observation::controls_visible(runtime)?;
        self.call = Some(observation::target(
            session,
            &self.configuration.target_symbol,
        )?);
        let mut report = self.report.borrow_mut();
        report.checks.root_opened = true;
        report.checks.source_exact = true;
        report.source_hash = Some(
            blake3::hash(ready.source.text.as_bytes())
                .to_hex()
                .to_string(),
        );
        report.source_bytes = Some(ready.source.text.len());
        self.phase = Phase::SelectCall;
        Ok(())
    }

    fn selected_call(&mut self, runtime: &Runtime<State>) -> Result<()> {
        let Some(session) = observation::session(&runtime.state, &self.configuration.root_symbol)?
        else {
            return Ok(());
        };
        let call = self.call.expect("observed call");
        if session.selected != Some(call.step)
            || !observation::highlighted(runtime, session, call.step)?
        {
            return Ok(());
        }
        let Some(details) = observation::target_details(runtime, session, call)? else {
            return Ok(());
        };
        {
            let mut report = self.report.borrow_mut();
            report.checks.source_highlighted = true;
            report.target_signature = Some(details.signature);
            report.target_certainty = Some(details.certainty);
            report.target_provenance = Some(details.provenance);
        }
        if !self.synthetic_considered {
            self.synthetic_considered = true;
            self.synthetic = session
                .ready
                .as_ref()
                .expect("ready root")
                .chart
                .steps
                .iter()
                .position(|step| step.span == codecallnavigator::flow::SourceSpan::default());
            if self.synthetic.is_some() {
                self.synthetic_scroll = session.scroll.get();
                self.phase = Phase::SelectSynthetic;
                return Ok(());
            }
        }
        self.before_pan = Some(SavedFlow::capture(session));
        self.phase = Phase::Pan;
        Ok(())
    }

    fn selected_synthetic(&mut self, runtime: &Runtime<State>) -> Result<()> {
        let Some(session) = observation::session(&runtime.state, &self.configuration.root_symbol)?
        else {
            return Ok(());
        };
        if session.selected != self.synthetic {
            return Ok(());
        }
        ensure!(
            close(session.scroll.get(), self.synthetic_scroll),
            "Unlocated cleanup step changed source scroll"
        );
        ensure!(
            observation::unlocated_has_no_highlight(runtime),
            "Unlocated cleanup step highlighted unrelated source"
        );
        ensure!(
            observation::unlocated_explained(runtime),
            "Unlocated cleanup step has no visible source-location explanation"
        );
        self.report.borrow_mut().checks.zero_span_checked = Some(true);
        self.phase = Phase::SelectCall;
        Ok(())
    }

    fn panned(&mut self, runtime: &Runtime<State>) -> Result<()> {
        let Some(session) = observation::session(&runtime.state, &self.configuration.root_symbol)?
        else {
            return Ok(());
        };
        let before = self.before_pan.as_ref().expect("observed root before pan");
        ensure!(
            !close(before.camera.pan, session.camera.pan),
            "Native canvas drag did not change the flow camera"
        );
        ensure!(
            close(before.scroll, session.scroll.get()),
            "Flow camera drag changed independent source scroll"
        );
        self.report.borrow_mut().checks.camera_dragged = true;
        self.after_pan = Some(SavedFlow::capture(session));
        self.phase = Phase::Scroll;
        Ok(())
    }

    fn scrolled(&mut self, runtime: &Runtime<State>) -> Result<()> {
        let Some(session) = observation::session(&runtime.state, &self.configuration.root_symbol)?
        else {
            return Ok(());
        };
        ensure!(
            !close(
                self.before_pan
                    .as_ref()
                    .expect("observed root before pan")
                    .scroll,
                session.scroll.get()
            ),
            "Native source wheel did not move the source pane"
        );
        let camera = &self
            .after_pan
            .as_ref()
            .expect("observed root after pan")
            .camera;
        ensure!(
            close(camera.pan, session.camera.pan)
                && (camera.zoom - session.camera.zoom).abs() < 0.001,
            "Source wheel changed the independent flow camera"
        );
        {
            let mut report = self.report.borrow_mut();
            report.checks.source_scrolled = true;
            report.checks.source_scroll_preserved_camera = true;
        }
        self.before_follow = Some(SavedFlow::capture(session));
        self.phase = Phase::Follow;
        Ok(())
    }

    fn target(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if observation::session(&runtime.state, &self.configuration.target_symbol)?.is_none() {
            return Ok(());
        }
        ensure!(
            runtime.state.flow_workspace.back.len() == 1,
            "Following the actual target did not preserve one caller history entry"
        );
        let mut report = self.report.borrow_mut();
        report.checks.target_followed = true;
        self.phase = Phase::Back;
        Ok(())
    }

    fn restored(&mut self, runtime: &Runtime<State>) -> Result<()> {
        let Some(session) = observation::session(&runtime.state, &self.configuration.root_symbol)?
        else {
            return Ok(());
        };
        ensure!(
            self.before_follow
                .as_ref()
                .expect("observed caller before following")
                .restored(session),
            "Back did not restore caller selection, camera and source scroll"
        );
        self.report.borrow_mut().checks.back_restored = true;
        self.captured_since = Some(Instant::now());
        self.phase = Phase::CaptureHold;
        Ok(())
    }

    fn capture(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if self
            .captured_since
            .expect("observed capture hold")
            .elapsed()
            < Duration::from_millis(250)
        {
            return Ok(());
        }
        let Some(window_id) = self.window.as_ref().and_then(WindowHandle::window_number) else {
            return Ok(());
        };
        ensure!(
            window_id > 0,
            "Native window has no registered capture identity"
        );
        ensure!(
            self.report.borrow().window_id == Some(window_id),
            "Native capture window differs from the acknowledged setup window"
        );
        observation::controls_visible(runtime)?;
        let generation = runtime
            .state
            .generation
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Native capture lost graph generation"))?;
        self.report.borrow_mut().window_id = Some(window_id);
        self.io.capture(CaptureRequest {
            version: 1,
            checkpoint: "restored-root",
            window_id,
            symbol: self.configuration.root_symbol.clone(),
            generation,
            expected_viewport: self.configuration.expected_viewport,
            viewport: [runtime.viewport.0, runtime.viewport.1],
        })?;
        self.phase = Phase::CaptureWait;
        Ok(())
    }

    fn graph(&mut self, runtime: &Runtime<State>) -> Result<()> {
        if runtime.state.flow_workspace.current.is_some() {
            return Ok(());
        }
        ensure!(
            Some(serde_json::to_value(&runtime.state.nav)?) == self.root_navigation,
            "Closing flow changed call-graph navigation"
        );
        self.report.borrow_mut().checks.graph_restored = true;
        self.finish(true, None)?;
        Ok(())
    }

    fn elapsed_request(&self) -> Option<f64> {
        self.requested.map(|at| at.elapsed().as_secs_f64() * 1000.0)
    }

    fn replies(&mut self, runtime: &mut Runtime<State>) {
        self.handle_observed_error(runtime);
        while let Some(reply) = self.io.reply() {
            self.handle_reply(runtime, reply);
        }
    }

    fn handle_observed_error(&mut self, runtime: &mut Runtime<State>) {
        if !matches!(self.phase, Phase::Writing | Phase::Closing)
            && self.report.borrow().status == "failed"
        {
            let error = self
                .report
                .borrow()
                .error
                .clone()
                .unwrap_or_else(|| "Native observation failed".into());
            self.fail(runtime, error);
        }
    }

    fn handle_reply(&mut self, runtime: &mut Runtime<State>, reply: Reply) {
        match reply {
            Reply::WindowSetup(result) if self.phase == Phase::WindowSetup => {
                if let Err(error) = result.and_then(|_| self.resize_window()) {
                    self.fail(runtime, format!("{error:#}"));
                }
            }
            Reply::Capture(result) if self.phase == Phase::CaptureWait => {
                self.capture_finished(runtime, result)
            }
            Reply::Finished(result) => {
                if let Err(error) = result {
                    eprintln!("flow acceptance: cannot save terminal report: {error:#}");
                }
                runtime.update(crate::jobs::close);
                self.phase = Phase::Closing;
            }
            _ => (),
        }
    }

    fn resize_window(&mut self) -> Result<()> {
        let window = self
            .window
            .as_ref()
            .context("Native setup lost its owned window")?;
        ensure!(
            window.window_number() == self.report.borrow().window_id,
            "Native window changed while setup acknowledgment was pending"
        );
        let [width, height] = self.configuration.expected_viewport;
        window.resize_instant((width as f32, height as f32));
        self.report.borrow_mut().checks.window_setup_acknowledged = true;
        self.phase = Phase::Resize;
        Ok(())
    }

    fn capture_finished(&mut self, runtime: &mut Runtime<State>, result: Result<()>) {
        match result {
            Ok(()) => self.phase = Phase::CloseFlow,
            Err(error) => self.fail(runtime, format!("{error:#}")),
        }
    }

    fn fail(&mut self, runtime: &mut Runtime<State>, message: String) {
        self.io.cancel_wait();
        if let Err(error) = self.finish(false, Some(message)) {
            eprintln!("flow acceptance: {error:#}");
            runtime.update(crate::jobs::close);
            self.phase = Phase::Closing;
        }
    }

    fn finish(&mut self, passed: bool, error: Option<String>) -> Result<()> {
        let mut report = self.report.borrow_mut();
        report.status = if passed { "passed" } else { "failed" }.into();
        report.error = error;
        self.io.finish(report.clone())?;
        self.phase = Phase::Writing;
        Ok(())
    }
}

fn close(left: [f32; 2], right: [f32; 2]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| (left - right).abs() < 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn driver() -> (Driver, tempfile::TempDir) {
        let directory = tempfile::tempdir().unwrap();
        let configuration: Configuration = serde_json::from_value(serde_json::json!({
            "version": 1, "project": "/tmp/project", "root_symbol": "Run",
            "target_symbol": "Leaf", "source": "class Example {}",
            "expected_viewport": [1050, 680]
        }))
        .unwrap();
        let worker = Worker::start(directory.path().into(), configuration.timeout()).unwrap();
        let report = Rc::new(RefCell::new(Report::new(&configuration)));
        (
            Driver::new(configuration, worker, report, Instant::now()),
            directory,
        )
    }

    #[test]
    fn window_setup_waits_for_acknowledgment_and_rejects_wrong_viewports() {
        let (mut driver, _directory) = driver();
        let mut runtime = Runtime::new(
            State::fixture(),
            crate::view::view,
            (1147.0, 1344.0),
            rune::Env::new(true, 1.0),
        );
        driver.observe(&runtime).unwrap();
        assert_eq!(driver.phase, Phase::WindowSetup);
        assert!(!driver.report.borrow().checks.viewport_ready);
        assert!(driver.check_viewport(&runtime).is_err());
        runtime.set_viewport((1050.0, 680.0), 1.0);
        driver.observe(&runtime).unwrap();
        assert_eq!(
            driver.phase,
            Phase::WindowSetup,
            "Size alone cannot replace the owned native window setup acknowledgment"
        );
        assert!(driver.check_viewport(&runtime).is_ok());
    }

    #[test]
    fn viewport_requires_stable_matching_drawn_observations_after_resizing() {
        let now = Instant::now();
        let mut stable = ViewportStability::default();
        assert!(!stable.observe(true, now));
        assert!(!stable.observe(true, now + Duration::from_millis(249)));
        assert!(!stable.observe(false, now + Duration::from_millis(250)));
        assert!(!stable.observe(true, now + Duration::from_millis(300)));
        assert!(!stable.observe(true, now + Duration::from_millis(549)));
        assert!(stable.observe(true, now + Duration::from_millis(550)));
    }

    fn ready_runtime() -> Runtime<State> {
        let graph = crate::flow_scenes::data();
        let function = &graph.flow_inventory.as_ref().unwrap().functions[0];
        let ready = crate::flow_workspace::Ready {
            chart: codecallnavigator::flow::build(function).unwrap(),
            source: crate::source::from_snapshot("source".into(), std::path::Path::new("flow.ts")),
        };
        let mut state = State::fixture();
        state.flow_workspace.current = Some(Session {
            symbol: "Run".into(),
            scroll_key: "test-scroll".into(),
            ready: Some(std::sync::Arc::new(ready)),
            camera: Camera::default(),
            selected: None,
            scroll: Rc::new(std::cell::Cell::new([0.0; 2])),
            message: String::new(),
        });
        let mut runtime = Runtime::new(
            state,
            |_, _| rune::el().key("action").size(100.0, 100.0),
            (1050.0, 680.0),
            rune::Env::new(true, 1.0),
        );
        runtime.frame(1.0 / 60.0);
        runtime
    }

    #[test]
    fn first_ready_drawn_timing_is_sampled_while_input_is_busy_and_retained() {
        let (mut driver, _directory) = driver();
        let mut runtime = ready_runtime();
        assert!(driver.input.click(&mut runtime, "action"));
        driver.phase = Phase::Root;
        driver.requested = Some(Instant::now() - Duration::from_millis(5));
        driver.observed(&runtime);
        let first = driver
            .report
            .borrow()
            .timings
            .root_click_to_ready_drawn_ms
            .unwrap();
        assert!(first >= 5.0);
        assert_eq!(
            driver.phase,
            Phase::Root,
            "Busy input must still defer phase advancement"
        );
        driver.requested = Some(Instant::now() - Duration::from_secs(1));
        driver.observed(&runtime);
        assert_eq!(
            driver.report.borrow().timings.root_click_to_ready_drawn_ms,
            Some(first)
        );
        driver.phase = Phase::Target;
        driver.observed(&runtime);
        assert!(
            driver
                .report
                .borrow()
                .timings
                .target_click_to_ready_drawn_ms
                .is_none(),
            "Caller readiness cannot satisfy the callee timing"
        );
        runtime
            .state
            .flow_workspace
            .current
            .as_mut()
            .unwrap()
            .symbol = "Leaf".into();
        driver.observed(&runtime);
        assert!(
            driver
                .report
                .borrow()
                .timings
                .target_click_to_ready_drawn_ms
                .is_some()
        );
    }
}
