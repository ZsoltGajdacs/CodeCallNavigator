//! Acceptance files and screenshot acknowledgments never block a native frame.
use super::report::{CaptureRequest, Report, WindowRequest};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

enum Command {
    WindowSetup(WindowRequest),
    Capture(CaptureRequest),
    Finish(Box<Report>),
    External(super::comparison::protocol::Request),
    FinishComparison(Box<super::comparison::report::Report>),
}

pub(super) enum Reply {
    WindowSetup(Result<()>),
    Capture(Result<()>),
    Finished(Result<()>),
    External { sequence: u32, result: Result<()> },
}

pub(super) struct Worker {
    sender: mpsc::Sender<Command>,
    receiver: mpsc::Receiver<Reply>,
    stopped: Arc<AtomicBool>,
}

struct FileWorker {
    directory: PathBuf,
    timeout: Duration,
    stopped: Arc<AtomicBool>,
}

impl Worker {
    pub fn start(directory: PathBuf, timeout: Duration) -> Result<Self> {
        for name in [
            "report.json",
            "capture-request.json",
            "capture-ack.json",
            "window-request.json",
            "window-ack.json",
        ] {
            ensure!(
                !directory.join(name).exists(),
                "Acceptance output already exists: {name}"
            );
        }
        for entry in fs::read_dir(&directory)? {
            let name = entry?.file_name();
            ensure!(
                !name.to_string_lossy().starts_with("external-"),
                "Stale comparison protocol file exists"
            );
        }
        let (sender, commands) = mpsc::channel();
        let (replies, receiver) = mpsc::channel();
        let stopped = Arc::new(AtomicBool::new(false));
        let files = FileWorker {
            directory,
            timeout,
            stopped: stopped.clone(),
        };
        thread::Builder::new()
            .name("flow-acceptance-io".into())
            .spawn(move || files.work(commands, replies))?;
        Ok(Self {
            sender,
            receiver,
            stopped,
        })
    }

    pub fn setup_window(&self, request: WindowRequest) -> Result<()> {
        ensure!(
            request.window_id > 0 && request.process_id > 0,
            "Native setup requires explicit window and process IDs"
        );
        self.sender
            .send(Command::WindowSetup(request))
            .context("Acceptance file worker stopped")
    }

    pub fn capture(&self, request: CaptureRequest) -> Result<()> {
        ensure!(
            request.window_id > 0,
            "Native capture requires a registered window ID"
        );
        self.sender
            .send(Command::Capture(request))
            .context("Acceptance file worker stopped")
    }

    pub fn finish(&self, report: Report) -> Result<()> {
        self.sender
            .send(Command::Finish(Box::new(report)))
            .context("Acceptance file worker stopped")
    }

    pub fn external(&self, request: super::comparison::protocol::Request) -> Result<()> {
        request.validate()?;
        self.sender
            .send(Command::External(request))
            .context("Comparison file worker stopped")
    }
    pub fn finish_comparison(&self, report: super::comparison::report::Report) -> Result<()> {
        self.sender
            .send(Command::FinishComparison(Box::new(report)))
            .context("Comparison file worker stopped")
    }
    pub fn reply(&self) -> Option<Reply> {
        self.receiver.try_recv().ok()
    }

    pub fn cancel_wait(&self) {
        self.stopped.store(true, Ordering::Release);
    }
}

impl FileWorker {
    fn work(self, commands: mpsc::Receiver<Command>, replies: mpsc::Sender<Reply>) {
        let mut sequence = 0;
        while let Ok(command) = commands.recv() {
            let terminal = matches!(command, Command::Finish(_) | Command::FinishComparison(_));
            let reply = match command {
                Command::WindowSetup(request) => Reply::WindowSetup(self.exchange(
                    "window-request.json",
                    "window-ack.json",
                    &request,
                )),
                Command::Capture(request) => Reply::Capture(self.exchange(
                    "capture-request.json",
                    "capture-ack.json",
                    &request,
                )),
                Command::External(request) => {
                    let result = if request.sequence == sequence + 1 {
                        sequence = request.sequence;
                        self.exchange_comparison(&request)
                    } else {
                        Err(anyhow::anyhow!(
                            "External request sequence must increase by one"
                        ))
                    };
                    Reply::External {
                        sequence: request.sequence,
                        result,
                    }
                }
                Command::FinishComparison(report) => {
                    Reply::Finished(write_json(&self.directory, "report.json", &report))
                }
                Command::Finish(report) => {
                    Reply::Finished(write_json(&self.directory, "report.json", &report))
                }
            };
            if replies.send(reply).is_err() || terminal {
                break;
            }
        }
    }

    fn exchange_comparison(&self, request: &super::comparison::protocol::Request) -> Result<()> {
        request.validate()?;
        write_json(&self.directory, &request.filename("request"), request)?;
        super::comparison::protocol::wait(&self.directory, request, self.timeout, &self.stopped)
    }
    fn exchange(
        &self,
        request_name: &str,
        acknowledgment_name: &str,
        request: &impl Serialize,
    ) -> Result<()> {
        write_json(&self.directory, request_name, request)?;
        wait_acknowledgment(
            &self.directory,
            acknowledgment_name,
            self.timeout,
            &self.stopped,
        )
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
    }
}

#[derive(Deserialize)]
struct Acknowledgment {
    version: u32,
    success: bool,
    error: Option<String>,
}

fn wait_acknowledgment(
    directory: &Path,
    name: &str,
    timeout: Duration,
    stopped: &AtomicBool,
) -> Result<()> {
    let deadline = Instant::now() + timeout;
    loop {
        ensure!(
            !stopped.load(Ordering::Acquire),
            "Acceptance stopped before {name}"
        );
        if let Some(()) = read_acknowledgment(directory, name)? {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "Native acknowledgment {name} timed out"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

fn read_acknowledgment(directory: &Path, name: &str) -> Result<Option<()>> {
    match fs::read(directory.join(name)) {
        Ok(bytes) => acknowledge(&bytes)
            .map(Some)
            .with_context(|| format!("Native acknowledgment {name} failed")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("Cannot read native acknowledgment {name}"))
        }
    }
}

fn acknowledge(bytes: &[u8]) -> Result<()> {
    let acknowledgment: Acknowledgment =
        serde_json::from_slice(bytes).context("Invalid native acknowledgment")?;
    ensure!(
        acknowledgment.version == 1,
        "Unsupported native acknowledgment version"
    );
    ensure!(
        acknowledgment.success,
        "Native operation failed: {}",
        acknowledgment
            .error
            .as_deref()
            .unwrap_or("unspecified native failure")
    );
    Ok(())
}

fn write_json(directory: &Path, name: &str, value: &impl Serialize) -> Result<()> {
    let temporary = directory.join(format!(".{name}.tmp"));
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, directory.join(name))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_acknowledgment_has_its_own_versioned_sequence_and_checkpoint() {
        assert!(super::super::comparison::protocol::sample().acknowledge(br#"{"version":2,"sequence":1,"operation":"arm_git_gate","checkpoint":"gate-armed","success":true}"#).is_ok(),"The comparison protocol needs an independent version two acknowledgment");
    }

    #[test]
    fn stale_window_setup_acknowledgments_cannot_be_reused() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(
            directory.path().join("window-ack.json"),
            br#"{"version":1,"success":true}"#,
        )
        .unwrap();
        assert!(Worker::start(directory.path().into(), Duration::from_secs(1)).is_err());
    }

    #[test]
    fn failed_or_unknown_capture_acknowledgments_cannot_pass() {
        assert!(acknowledge(br#"{"version":1,"success":true}"#).is_ok());
        assert!(acknowledge(br#"{"version":1,"success":false,"error":"capture denied"}"#).is_err());
        assert!(
            super::super::comparison::protocol::sample()
                .acknowledge(br#"{"version":2,"success":true}"#)
                .is_err()
        );
        assert!(acknowledge(br#"{"version":1}"#).is_err());
    }

    #[test]
    fn window_setup_waits_off_thread_for_ack_and_reports_failure() {
        let directory = tempfile::tempdir().unwrap();
        let worker = Worker::start(directory.path().into(), Duration::from_secs(5)).unwrap();
        worker
            .setup_window(WindowRequest {
                version: 1,
                window_id: 123,
                process_id: std::process::id(),
                expected_viewport: [1050, 680],
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !directory.path().join("window-request.json").exists() {
            assert!(
                Instant::now() < deadline,
                "Worker did not publish setup request"
            );
            thread::yield_now();
        }
        assert!(
            worker.reply().is_none(),
            "Setup passed without acknowledgment"
        );
        write_json(
            directory.path(),
            "window-ack.json",
            &serde_json::json!({
                "version": 1, "success": false, "error": "Owned window could not be prepared"
            }),
        )
        .unwrap();
        loop {
            if let Some(reply) = worker.reply() {
                assert!(matches!(reply, Reply::WindowSetup(Err(_))));
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Worker did not report setup failure"
            );
            thread::yield_now();
        }
    }

    #[test]
    fn absent_acknowledgments_timeout_and_cancel_without_success() {
        let directory = tempfile::tempdir().unwrap();
        let stopped = AtomicBool::new(false);
        assert!(
            wait_acknowledgment(
                directory.path(),
                "window-ack.json",
                Duration::ZERO,
                &stopped
            )
            .is_err()
        );
        stopped.store(true, Ordering::Release);
        assert!(
            wait_acknowledgment(
                directory.path(),
                "capture-ack.json",
                Duration::from_secs(5),
                &stopped
            )
            .is_err()
        );
    }
}
