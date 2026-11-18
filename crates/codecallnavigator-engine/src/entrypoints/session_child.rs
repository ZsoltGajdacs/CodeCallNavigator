//! Owned persistent analysis-session transport; scheduling and retries belong to its caller.
mod stream;
use super::refresh_child::{Readers, drain_log, nonblocking};
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator_core::refresh::{self, Control, SessionRequest};
use codecallnavigator_engine::process_ownership::{CleanupFailure, OwnedProcess};
use std::{
    collections::VecDeque,
    io::Write,
    process::{ChildStdin, Command, ExitStatus, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use stream::Mailbox;
pub(super) use stream::Notifications;

const MAX_REQUEST: usize = 4 * 1024 * 1024;
const EXIT_GRACE: Duration = Duration::from_secs(5);

pub(super) struct SessionChild {
    child: Option<OwnedProcess>,
    input: Option<ChildStdin>,
    outgoing: Vec<u8>,
    offset: usize,
    pending_control: Option<Control>,
    mailbox: Arc<Mutex<Mailbox>>,
    stderr: Arc<Mutex<VecDeque<u8>>>,
    readers: Readers,
    ending: Option<Instant>,
    cleanup_failure: Option<CleanupFailure>,
}

impl SessionChild {
    pub fn start(request: &SessionRequest) -> Result<Self> {
        ensure!(
            request.version == refresh::SESSION_VERSION,
            "Unsupported analysis session version"
        );
        let mut outgoing = serde_json::to_vec(request)?;
        ensure!(
            outgoing.len() <= MAX_REQUEST,
            "Analysis-session request exceeds 4 MiB"
        );
        outgoing.push(b'\n');
        let child = OwnedProcess::spawn(
            Command::new(std::env::current_exe()?)
                .arg("analysis-session")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )?;
        let mut session = Self {
            child: Some(child),
            input: None,
            outgoing,
            offset: 0,
            pending_control: None,
            mailbox: Default::default(),
            stderr: Default::default(),
            readers: Readers::default(),
            ending: None,
            cleanup_failure: None,
        };
        if let Err(error) = session.initialize() {
            return session.finish_owned(Err(error)).map(|()| session);
        }
        Ok(session)
    }

    fn initialize(&mut self) -> Result<()> {
        let child = self
            .child
            .as_mut()
            .context("Analysis session is closed")?
            .child_mut();
        let input = child
            .stdin
            .take()
            .context("Analysis-session stdin unavailable")?;
        nonblocking(&input)?;
        self.input = Some(input);
        let stdout = child
            .stdout
            .take()
            .context("Analysis-session stdout unavailable")?;
        let stderr = child
            .stderr
            .take()
            .context("Analysis-session stderr unavailable")?;
        nonblocking(&stdout)?;
        nonblocking(&stderr)?;
        let mailbox = self.mailbox.clone();
        let stdout = self.readers.pipe(stdout);
        self.readers.spawn(move || stream::read(stdout, mailbox));
        let log = self.stderr.clone();
        let stderr = self.readers.pipe(stderr);
        self.readers.spawn(move || drain_log(stderr, log));
        Ok(())
    }

    pub fn poll(&mut self) -> Result<Notifications> {
        let status = self
            .child
            .as_mut()
            .map(OwnedProcess::try_wait)
            .transpose()?
            .flatten();
        let notices = self.notifications(status)?;
        if notices.terminal.is_none() && !notices.exited {
            self.write_pending()?;
        }
        Ok(notices)
    }

    fn notifications(&mut self, status: Option<ExitStatus>) -> Result<Notifications> {
        let mut mailbox = self
            .mailbox
            .lock()
            .map_err(|_| anyhow::anyhow!("Analysis-session mailbox poisoned"))?;
        let mut notices = mailbox.take();
        let process_exited = self.child.is_none() || status.is_some();
        notices.exited = process_exited && mailbox.closed;
        let ending = process_exited || mailbox.closed || notices.terminal.is_some();
        let closed = mailbox.closed;
        drop(mailbox);
        self.check_closure(ending, notices.exited)?;
        if closed {
            self.failed_exit(status, &mut notices);
        }
        Ok(notices)
    }

    fn check_closure(&mut self, ending: bool, exited: bool) -> Result<()> {
        if ending {
            self.ending.get_or_insert_with(Instant::now);
        }
        if exited {
            return Ok(());
        }
        ensure!(
            !self.ending.is_some_and(|at| at.elapsed() >= EXIT_GRACE),
            "Analysis session did not close its process and event stream"
        );
        Ok(())
    }

    fn failed_exit(&self, status: Option<ExitStatus>, notices: &mut Notifications) {
        let Some(status) = status else {
            return;
        };
        if status.success() || notices.terminal.is_some() {
            return;
        }
        notices.terminal = Some(codecallnavigator_core::query::Error::new(
            "analysis_session_exited",
            self.diagnostic(&anyhow::anyhow!("Analysis session failed ({status})")),
        ));
    }

    pub fn cancel_attempt(&mut self) -> Result<()> {
        ensure!(self.input.is_some(), "Analysis session is closed");
        self.pending_control = Some(Control::CancelRefresh {
            version: refresh::VERSION,
        });
        self.write_pending()
    }

    fn write_pending(&mut self) -> Result<()> {
        let Some(input) = self.input.as_mut() else {
            return Ok(());
        };
        if self.offset == self.outgoing.len() {
            let Some(control) = self.pending_control.take() else {
                return Ok(());
            };
            self.outgoing = serde_json::to_vec(&control)?;
            self.outgoing.push(b'\n');
            self.offset = 0;
        }
        let end = (self.offset + 65536).min(self.outgoing.len());
        match input.write(&self.outgoing[self.offset..end]) {
            Ok(0) => bail!("Analysis session closed its request pipe"),
            Ok(count) => self.offset += count,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<()> {
        self.input.take();
        let result = self.wait_for_shutdown();
        self.finish_owned(result)
    }

    fn wait_for_shutdown(&mut self) -> Result<()> {
        let deadline = Instant::now() + EXIT_GRACE;
        while Instant::now() < deadline {
            let Some(child) = self.child.as_mut() else {
                return self.previous_cleanup();
            };
            if child.try_wait()?.is_some()
                && self
                    .mailbox
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Analysis-session mailbox poisoned"))?
                    .closed
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        // The cooperative EOF grace expired; complete() performs bounded owned
        // cleanup. A live host after EOF is not allowed to escape ownership.
        Ok(())
    }

    fn finish_owned(&mut self, operation: Result<()>) -> Result<()> {
        self.input.take();
        let result = match self.child.take() {
            Some(mut child) => child.complete(operation),
            None => operation.and_then(|()| self.previous_cleanup()),
        };
        self.readers.stop();
        if let Err(error) = &result
            && let Some(failure) = error.downcast_ref::<CleanupFailure>()
        {
            self.cleanup_failure = Some(failure.clone());
        }
        result?;
        self.mailbox
            .lock()
            .map_err(|_| anyhow::anyhow!("Analysis-session mailbox poisoned"))?
            .result()
    }

    fn previous_cleanup(&self) -> Result<()> {
        match &self.cleanup_failure {
            Some(failure) => Err(failure.clone().into()),
            None => Ok(()),
        }
    }

    fn diagnostic(&self, error: &anyhow::Error) -> String {
        let tail = self
            .stderr
            .lock()
            .map(|log| {
                String::from_utf8_lossy(&log.iter().copied().collect::<Vec<_>>()).into_owned()
            })
            .unwrap_or_default();
        format!("{error:#}; analysis-session stderr: {tail}")
    }
}

impl Drop for SessionChild {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
