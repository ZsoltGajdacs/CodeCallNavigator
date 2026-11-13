//! Owned CLI observation/coordinator transport; refresh policy remains in the engine.
mod activity;
use super::{Descriptor, query_process};
use crate::runtime::Runtime;
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator::{
    freshness::{self, EventPayload, FreshnessObservation, StreamEvent},
    process_ownership::Descendants,
    query::{self, Query},
    refresh::{Activity, Control},
};
use std::{
    io::{BufRead, BufReader, Read},
    os::unix::process::CommandExt,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const MAX_FRAME: u64 = 64 * 1024;
const LIVENESS: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub(crate) struct CleanupFailure(anyhow::Error);

impl std::fmt::Display for CleanupFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Owned refresh cleanup could not be confirmed: {:#}",
            self.0
        )
    }
}
impl std::error::Error for CleanupFailure {}

pub(crate) enum Notice {
    Observation(FreshnessObservation),
    Activity(Activity),
}

pub(crate) struct ObservationOptions<'a> {
    pub automatic: bool,
    pub receive: &'a dyn Fn(Notice),
    pub controls: Option<&'a mpsc::Receiver<Control>>,
}

impl<'a> ObservationOptions<'a> {
    pub fn passive(receive: &'a dyn Fn(Notice)) -> Self {
        Self {
            automatic: false,
            receive,
            controls: None,
        }
    }
}

pub(super) fn run(
    runtime: &Runtime,
    descriptor: Descriptor,
    cancel: &AtomicBool,
    options: ObservationOptions<'_>,
) -> Result<()> {
    let mut session = Session::start(runtime, options.automatic)?;
    let result = (|| {
        let bytes = request_bytes(descriptor)?;
        query_process::write_request(
            session.input.as_mut().expect("owned monitor stdin"),
            &bytes,
            cancel,
            Instant::now(),
        )?;
        let mut conversation = Conversation::default();
        let mut last_event = Instant::now();
        while !cancel.load(Ordering::Acquire) {
            session.update_owned()?;
            if let Some(controls) = options.controls {
                for control in controls.try_iter() {
                    let mut bytes = serde_json::to_vec(&control)?;
                    bytes.push(b'\n');
                    query_process::write_request(
                        session.input.as_mut().context("Monitor stdin is closed")?,
                        &bytes,
                        cancel,
                        Instant::now(),
                    )?;
                }
            }
            match session.frames.recv_timeout(Duration::from_millis(20)) {
                Ok(frame) => {
                    let observation = conversation.receive(frame?)?;
                    last_event = Instant::now();
                    (options.receive)(observation);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    bail!("CLI freshness monitor disconnected: {}", session.log_tail())
                }
                Err(mpsc::RecvTimeoutError::Timeout) => ensure!(
                    last_event.elapsed() < LIVENESS,
                    "CLI freshness monitor stopped responding"
                ),
            }
        }
        Ok(())
    })();
    let cleanup = session
        .finish()
        .map_err(|error| anyhow::Error::new(CleanupFailure(error)));
    match (result, cleanup) {
        (Ok(()), cleanup) => cleanup,
        (Err(error), Ok(())) => Err(error),
        (Err(error), Err(cleanup)) => {
            Err(cleanup.context(format!("Monitor also failed: {error:#}")))
        }
    }
}

fn request_bytes(descriptor: Descriptor) -> Result<Vec<u8>> {
    let request = query::Request {
        version: query::VERSION,
        project: descriptor.project,
        roots: descriptor.roots,
        expected_generation: None,
        query: Query::Freshness {
            observation_version: freshness::VERSION,
        },
    };
    let mut bytes = serde_json::to_vec(&request)?;
    ensure!(
        bytes.len() < 4 * 1024 * 1024,
        "Observation request exceeds 4 MiB"
    );
    bytes.push(b'\n');
    Ok(bytes)
}

#[derive(Default)]
struct Conversation {
    sequence: u64,
    generation: Option<String>,
    revision: Option<String>,
    runs_started: u64,
}

impl Conversation {
    fn receive(&mut self, frame: StreamEvent) -> Result<Notice> {
        ensure!(
            frame.version == freshness::VERSION,
            "Unsupported freshness event version"
        );
        ensure!(
            frame.sequence > self.sequence,
            "Freshness event sequence did not increase"
        );
        let observation = match frame.payload {
            EventPayload::Activity { activity } => {
                ensure!(
                    self.sequence != 0,
                    "Refresh activity preceded initial observation"
                );
                activity::validate(&activity)?;
                ensure!(
                    activity.runs_started >= self.runs_started,
                    "Refresh run counter moved backwards"
                );
                self.runs_started = activity.runs_started;
                self.sequence = frame.sequence;
                return Ok(Notice::Activity(activity));
            }
            EventPayload::Initial { observation } => {
                ensure!(
                    self.sequence == 0 && frame.sequence == 1,
                    "Unexpected initial freshness event"
                );
                observation
            }
            EventPayload::Change { observation } => {
                ensure!(
                    self.sequence != 0,
                    "Freshness monitor omitted initial event"
                );
                observation
            }
            EventPayload::Heartbeat { observation } => {
                ensure!(
                    self.sequence != 0
                        && self.generation.as_ref() == Some(&observation.generation)
                        && self.revision == observation.input_revision,
                    "Freshness heartbeat changed generation or revision"
                );
                observation
            }
            EventPayload::Failed { error } => bail!("{}: {}", error.code, error.message),
        };
        validate(&observation)?;
        self.sequence = frame.sequence;
        self.generation = Some(observation.generation.clone());
        self.revision = observation.input_revision.clone();
        Ok(Notice::Observation(observation))
    }
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate(observation: &FreshnessObservation) -> Result<()> {
    ensure!(
        observation.version == freshness::VERSION,
        "Unsupported freshness observation version"
    );
    ensure!(
        observation.contract_valid
            || observation.input_status
                == codecallnavigator::artifact::verification::InputStatus::Unverifiable,
        "Invalid artifact contract must report unverifiable inputs"
    );
    ensure!(
        digest(&observation.generation),
        "Invalid freshness generation"
    );
    ensure!(
        observation.input_revision.as_deref().is_none_or(digest),
        "Invalid freshness revision"
    );
    ensure!(
        !observation.coverage.complete || observation.input_revision.is_some(),
        "Complete freshness coverage omitted revision"
    );
    ensure!(
        !observation.coverage.certifies_build && observation.mode.limited,
        "Unsupported freshness coverage or mode"
    );
    ensure!(
        observation.reasons.len() <= query::MAX_FRESHNESS_REASONS
            && serde_json::to_vec(&observation.reasons)?.len() <= query::MAX_FRESHNESS_REASON_BYTES,
        "Oversized freshness reasons"
    );
    ensure!(
        observation.reasons_total >= observation.reasons.len()
            && observation.omitted_reasons == observation.reasons_total - observation.reasons.len(),
        "Inconsistent freshness reason counts"
    );
    ensure!(
        observation.verification_milliseconds.is_finite()
            && observation.verification_milliseconds >= 0.0,
        "Invalid verification duration"
    );
    Ok(())
}

struct Session {
    child: Option<Child>,
    input: Option<ChildStdin>,
    frames: mpsc::Receiver<Result<StreamEvent>>,
    log: tempfile::NamedTempFile,
    automatic: bool,
    owned: Option<Descendants>,
    last_inventory: Instant,
    stopped: bool,
}

impl Session {
    fn start(runtime: &Runtime, automatic: bool) -> Result<Self> {
        let log = tempfile::NamedTempFile::new()?;
        let mut command = Command::new(&runtime.engine);
        command.arg("observe");
        if automatic {
            command.arg("--auto-refresh");
        }
        let child = command
            .env("CODECALLNAVIGATOR_RESOURCES", &runtime.resources)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log.reopen()?))
            .process_group(0)
            .spawn()
            .context("Cannot start CLI freshness monitor")?;
        let (send, frames) = mpsc::sync_channel(1);
        // Own the child before configuring any fallible pipe operation.
        let mut session = Self {
            child: Some(child),
            input: None,
            frames,
            log,
            automatic,
            owned: None,
            last_inventory: Instant::now(),
            stopped: false,
        };
        if automatic {
            session.owned = Some(Descendants::new(
                session.child.as_ref().expect("owned monitor child"),
            )?);
        }
        let input = session
            .child
            .as_mut()
            .expect("owned monitor child")
            .stdin
            .take()
            .context("Missing freshness monitor stdin")?;
        query_process::nonblocking(&input)?;
        session.input = Some(input);
        let output = session
            .child
            .as_mut()
            .expect("owned monitor child")
            .stdout
            .take()
            .context("Missing freshness monitor stdout")?;
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let frame = read_frame(&mut reader);
                let failed = frame.is_err();
                if send.send(frame).is_err() || failed {
                    break;
                }
            }
        });
        Ok(session)
    }
    fn update_owned(&mut self) -> Result<()> {
        if self.last_inventory.elapsed() >= Duration::from_secs(1) {
            if let Some(owned) = &mut self.owned {
                owned.refresh()?;
            }
            self.last_inventory = Instant::now();
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.finish_with_grace(Duration::from_secs(15))
    }

    fn finish_with_grace(&mut self, grace: Duration) -> Result<()> {
        if self.stopped {
            return Ok(());
        }
        let mut diagnostic = self.update_owned_now().err();
        self.input.take();
        if let Some(child) = self.child.as_mut() {
            child.stdin.take();
        }
        if self.automatic {
            self.graceful_exit(&mut diagnostic, grace);
        }
        if let Err(error) = self.update_owned_now() {
            diagnostic = Some(error);
        }
        if let Err(error) = self.terminate_owned() {
            diagnostic = Some(error);
        }
        self.stopped = true;
        diagnostic.map_or(Ok(()), |error| {
            Err(error.context("Automatic refresh descendant cleanup could not be confirmed"))
        })
    }

    fn update_owned_now(&mut self) -> Result<()> {
        self.owned.as_mut().map_or(Ok(()), Descendants::refresh)
    }

    fn terminate_owned(&mut self) -> Result<()> {
        let Some(child) = self.child.as_mut() else {
            return Ok(());
        };
        let result = match &mut self.owned {
            Some(owned) => owned.stop(child),
            None => {
                query_process::stop_child(child);
                Ok(())
            }
        };
        if result.is_err()
            && let Some(child) = self.child.take()
        {
            codecallnavigator::process_ownership::reap_eventually(child);
        }
        result
    }

    fn graceful_exit(&mut self, diagnostic: &mut Option<anyhow::Error>, grace: Duration) {
        let started = Instant::now();
        while started.elapsed() < grace {
            if let Err(error) = self.update_owned() {
                *diagnostic = Some(error);
            }
            if self
                .child
                .as_mut()
                .is_none_or(|child| child.try_wait().ok().flatten().is_some())
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn log_tail(&self) -> String {
        std::fs::read_to_string(self.log.path())
            .unwrap_or_default()
            .chars()
            .rev()
            .take(2000)
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    }
}

fn read_frame(reader: &mut impl BufRead) -> Result<StreamEvent> {
    let mut bytes = Vec::new();
    let count = reader.take(MAX_FRAME + 1).read_until(b'\n', &mut bytes)?;
    ensure!(
        count > 0 && count as u64 <= MAX_FRAME,
        "Freshness stream ended or exceeded 64 KiB frame limit"
    );
    serde_json::from_slice(&bytes).context("Invalid CLI freshness event")
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("{error:#}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codecallnavigator::artifact::verification::InputStatus;

    fn frame(sequence: u64, payload: EventPayload) -> StreamEvent {
        StreamEvent {
            version: 1,
            sequence,
            payload,
        }
    }
    #[test]
    fn stalled_auto_coordinator_cleanup_reaches_independently_grouped_worker() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("coordinator");
        let marker = root.path().join("worker.pid");
        let script = format!(
            r#"#!/usr/bin/env python3
import pathlib,subprocess,time
worker=subprocess.Popen(['python3','-c','import time;time.sleep(60)'],start_new_session=True)
pathlib.Path({marker:?}).write_text(str(worker.pid))
time.sleep(60)
"#,
            marker = marker.to_string_lossy()
        );
        std::fs::write(&executable, script).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        let runtime = Runtime {
            engine: executable,
            resources: root.path().into(),
            dot: Default::default(),
        };
        let mut session = Session::start(&runtime, true).unwrap();
        let started = Instant::now();
        while !marker.is_file() && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        let pid = std::fs::read_to_string(marker).unwrap();
        session
            .finish_with_grace(Duration::from_millis(80))
            .unwrap();
        assert!(
            session
                .child
                .as_mut()
                .unwrap()
                .try_wait()
                .unwrap()
                .is_some()
        );
        let started = Instant::now();
        loop {
            let output = Command::new("/bin/ps")
                .args(["-o", "stat=", "-p", pid.trim()])
                .output()
                .unwrap();
            if output.stdout.is_empty() || output.stdout.starts_with(b"Z") {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "Owned worker survived cleanup"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn refresh_activity_is_separate_from_validity_and_counters_cannot_regress() {
        use codecallnavigator::refresh::{Activity, Phase};
        let mut conversation = Conversation::default();
        conversation
            .receive(frame(
                1,
                EventPayload::Initial {
                    observation: observation(),
                },
            ))
            .unwrap();
        let mut activity = Activity::new(Phase::Watching);
        activity.runs_started = 2;
        assert!(matches!(
            conversation
                .receive(frame(
                    2,
                    EventPayload::Activity {
                        activity: activity.clone()
                    }
                ))
                .unwrap(),
            Notice::Activity(_)
        ));
        assert!(matches!(
            conversation
                .receive(frame(
                    3,
                    EventPayload::Heartbeat {
                        observation: observation()
                    }
                ))
                .unwrap(),
            Notice::Observation(_)
        ));
        activity.runs_started = 1;
        assert!(
            conversation
                .receive(frame(4, EventPayload::Activity { activity }))
                .is_err()
        );
    }

    fn observation() -> FreshnessObservation {
        crate::freshness_workspace::fixture(InputStatus::Match, &"a".repeat(64))
    }

    #[test]
    fn observation_frames_reject_duplicates_malformed_digests_and_changed_heartbeats() {
        let mut conversation = Conversation::default();
        conversation
            .receive(frame(
                1,
                EventPayload::Initial {
                    observation: observation(),
                },
            ))
            .unwrap();
        assert!(
            conversation
                .receive(frame(
                    1,
                    EventPayload::Change {
                        observation: observation()
                    }
                ))
                .is_err()
        );
        let mut changed = observation();
        changed.generation = "b".repeat(64);
        assert!(
            conversation
                .receive(frame(
                    2,
                    EventPayload::Heartbeat {
                        observation: changed
                    }
                ))
                .is_err()
        );
        let mut invalid = observation();
        invalid.input_revision = Some("not-a-content-digest".into());
        assert!(
            conversation
                .receive(frame(
                    2,
                    EventPayload::Change {
                        observation: invalid
                    }
                ))
                .is_err()
        );
        assert!(read_frame(&mut &vec![b'x'; MAX_FRAME as usize + 1][..]).is_err());
    }

    #[test]
    fn invalid_contract_cannot_claim_matching_or_stale_inputs() {
        for status in [InputStatus::Match, InputStatus::Stale] {
            let mut invalid = observation();
            invalid.contract_valid = false;
            invalid.input_status = status;
            assert!(
                Conversation::default()
                    .receive(frame(
                        1,
                        EventPayload::Initial {
                            observation: invalid
                        }
                    ))
                    .is_err(),
                "Invalid contract claimed {status:?}"
            );
        }
    }

    #[test]
    fn retry_renegotiates_capabilities_after_cli_upgrade() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let real = Runtime::discover().unwrap();
        let artifact = codecallnavigator::artifact::canonical(
            &codecallnavigator::model::GraphData::default(),
            &[("repo".into(), root.path().into())],
        )
        .unwrap();
        std::fs::create_dir(root.path().join(".codecallnavigator")).unwrap();
        std::fs::write(
            root.path().join(codecallnavigator::artifact::GRAPH_PATH),
            artifact.bytes().unwrap(),
        )
        .unwrap();
        let wrapper = root.path().join("cli");
        let legacy = root.path().join("legacy");
        std::fs::write(&legacy, "legacy compatibility fixture").unwrap();
        let program = format!(
            r#"#!/usr/bin/env python3
import os,sys,json,subprocess,pathlib
real={}
if sys.argv[1]=='capabilities' and pathlib.Path({}).exists():
 data=json.loads(subprocess.check_output([real,'capabilities']))
 data.pop('freshness',None)
 print(json.dumps(data))
else:
 os.execv(real,[real,*sys.argv[1:]])
"#,
            serde_json::to_string(&real.engine).unwrap(),
            serde_json::to_string(&legacy).unwrap()
        );
        std::fs::write(&wrapper, program).unwrap();
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
        let runtime = Runtime {
            engine: wrapper,
            ..real
        };
        let context = super::super::QueryContext::for_project(&runtime, root.path().into());
        let cancel = AtomicBool::new(false);
        assert!(
            !context
                .observe(&cancel, &|_| panic!("Legacy CLI must not start monitor"))
                .unwrap()
        );
        std::fs::remove_file(legacy).unwrap();
        let observed = std::cell::Cell::new(false);
        assert!(
            context
                .observe(&cancel, &|_| {
                    observed.set(true);
                    cancel.store(true, Ordering::Release);
                })
                .unwrap()
        );
        assert!(
            observed.get(),
            "Explicit retry must inspect upgraded CLI capabilities"
        );
    }

    #[test]
    fn real_cli_observes_repairs_and_owned_cancellation_completes() {
        use codecallnavigator::{artifact, fingerprint, model::GraphData};
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let source = root.join("app.ts");
        std::fs::write(&source, "export const value = 1;").unwrap();
        let roots = [("repo".into(), root.clone())];
        let captured = fingerprint::capture_excluding([root.clone(), source.clone()], &[]).unwrap();
        let mut graph = artifact::canonical(&GraphData::default(), &roots).unwrap();
        graph.graph["analysis"] = serde_json::to_value(
            artifact::provenance::build(&captured, &[], "{}", &roots).unwrap(),
        )
        .unwrap();
        std::fs::create_dir(root.join(".codecallnavigator")).unwrap();
        std::fs::write(root.join(artifact::GRAPH_PATH), graph.bytes().unwrap()).unwrap();
        let context = super::super::QueryContext::for_project(&Runtime::discover().unwrap(), root);
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let (send, receive) = mpsc::channel();
        let (done, result) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let answer = context.observe(&worker_cancel, &|observation| {
                send.send(observation).unwrap();
            });
            done.send(answer).unwrap();
        });
        let initial = receive.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(initial.input_status, InputStatus::Match);
        std::fs::write(&source, "export const value = ").unwrap();
        let invalid = receive.recv_timeout(Duration::from_secs(5)).unwrap();
        std::fs::write(source, "export const value = 2;").unwrap();
        let repaired = receive.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(repaired.input_status, InputStatus::Stale);
        assert_ne!(invalid.input_revision, repaired.input_revision);
        cancel.store(true, Ordering::Release);
        assert!(
            result
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                .unwrap()
        );
        worker.join().unwrap();
    }
}
