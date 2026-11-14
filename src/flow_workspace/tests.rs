//! Controlled publisher completions complement the real CLI's cancellation UI test.
use super::*;
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug)]
enum Superseded {
    Serial,
    Epoch,
    Symbol,
}

#[derive(Clone, Copy)]
enum Answer {
    Ready,
    Error,
}

fn ready(symbol: &str) -> Ready {
    let graph = crate::flow_scenes::data();
    let mut function = graph.flow_inventory.as_ref().unwrap().functions[0].clone();
    function.symbol = symbol.into();
    Ready {
        chart: flow::build(&function).unwrap(),
        source: source::from_snapshot(
            graph.architecture.as_ref().unwrap().files[0]
                .content
                .clone(),
            Path::new(&function.file),
        ),
    }
}

fn current(state: &mut State, symbol: &str) {
    let workspace = &mut state.flow_workspace;
    let ready = Arc::new(ready(symbol));
    let mut session = Session::new(symbol.into(), state.epoch, workspace.serial);
    session.ready = Some(ready.clone());
    session.message = "Current session status".into();
    session.camera.pan = [31.0, 41.0];
    session.camera.zoom = 0.75;
    session.selected = Some(0);
    session.scroll.set([11.0, 120.0]);
    workspace.current = Some(session);
    workspace.epoch = state.epoch;
    workspace.cache.insert((state.epoch, symbol.into()), ready);
}

fn supersede(state: &mut State, reason: Superseded) {
    let symbol = match reason {
        Superseded::Serial => {
            state.flow_workspace.serial += 1;
            "current"
        }
        Superseded::Epoch => {
            state.epoch += 1;
            "current"
        }
        Superseded::Symbol => "new-symbol",
    };
    current(state, symbol);
}

#[derive(Debug, PartialEq)]
struct PublishedState {
    symbol: String,
    message: String,
    selected: Option<usize>,
    camera: serde_json::Value,
    scroll: [f32; 2],
    ready: *const Ready,
    cache: Vec<((u64, String), *const Ready)>,
    fit_pending: bool,
    restore_scroll: Option<[f32; 2]>,
}

fn published(state: &State) -> PublishedState {
    let workspace = &state.flow_workspace;
    let session = workspace.current.as_ref().unwrap();
    let mut cache: Vec<_> = workspace
        .cache
        .iter()
        .map(|(key, value)| (key.clone(), Arc::as_ptr(value)))
        .collect();
    cache.sort_by(|a, b| a.0.cmp(&b.0));
    PublishedState {
        symbol: session.symbol.clone(),
        message: session.message.clone(),
        selected: session.selected,
        camera: serde_json::to_value(&session.camera).unwrap(),
        scroll: session.scroll.get(),
        ready: Arc::as_ptr(session.ready.as_ref().unwrap()),
        cache,
        fit_pending: workspace.fit_pending,
        restore_scroll: workspace.restore_scroll,
    }
}

struct ControlledJob {
    release: mpsc::Sender<()>,
    returned: mpsc::Receiver<()>,
}

impl ControlledJob {
    fn complete(self, workspace: &Workspace) {
        self.release.send(()).unwrap();
        self.returned
            .recv_timeout(Duration::from_secs(5))
            .expect("Old worker really returned an answer");
        wait_until_landed(workspace);
    }
}

fn start_old_job(state: &State, answer: Answer) -> ControlledJob {
    let (started, running) = mpsc::channel();
    let (release, waiting) = mpsc::channel();
    let (completed, returned) = mpsc::channel();
    let workspace = &state.flow_workspace;
    let result = match answer {
        Answer::Ready => Ok(ready("current")),
        Answer::Error => Err("Old job error must not replace current status".into()),
    };
    let output = Output {
        serial: workspace.serial,
        epoch: state.epoch,
        symbol: "current".into(),
        result,
    };
    workspace.job.request(output, move |output| {
        started.send(()).unwrap();
        waiting
            .recv_timeout(Duration::from_secs(5))
            .expect("Release the controlled old result");
        completed.send(()).unwrap();
        output
    });
    running
        .recv_timeout(Duration::from_secs(5))
        .expect("Old worker really started");
    ControlledJob { release, returned }
}

fn wait_until_landed(workspace: &Workspace) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while workspace.job.is_pending() {
        assert!(
            Instant::now() < deadline,
            "Released old result did not land"
        );
        thread::sleep(Duration::from_millis(1));
    }
}

fn rejects_late_answer(reason: Superseded, answer: Answer) {
    let mut state = State::new();
    current(&mut state, "current");
    let job = start_old_job(&state, answer);
    // Leave Latest uncancelled so its answer really reaches the workspace publisher.
    supersede(&mut state, reason);
    let before = published(&state);
    job.complete(&state.flow_workspace);
    assert!(
        !take_output(&mut state),
        "Superseded result was published for {reason:?}"
    );
    assert!(
        state.flow_workspace.job.take().is_none(),
        "Publisher must consume the late result exactly once"
    );
    assert_eq!(
        published(&state),
        before,
        "Late result changed valid current workspace for {reason:?}"
    );
}

#[test]
fn late_ready_results_cannot_replace_newer_serial_epoch_or_symbol() {
    for reason in [Superseded::Serial, Superseded::Epoch, Superseded::Symbol] {
        rejects_late_answer(reason, Answer::Ready);
    }
}

#[test]
fn late_error_results_cannot_replace_newer_serial_epoch_or_symbol() {
    for reason in [Superseded::Serial, Superseded::Epoch, Superseded::Symbol] {
        rejects_late_answer(reason, Answer::Error);
    }
}

#[test]
fn matching_controlled_completion_really_reaches_the_publisher() {
    let mut state = State::new();
    current(&mut state, "current");
    let before = published(&state);
    start_old_job(&state, Answer::Ready).complete(&state.flow_workspace);
    assert!(
        take_output(&mut state),
        "Matching controlled result must actually publish"
    );
    assert_ne!(
        published(&state),
        before,
        "Positive control must observe installation"
    );
}
