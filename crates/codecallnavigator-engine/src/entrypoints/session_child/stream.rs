//! Bounded, coalesced session notices retain a publication independently of activity.
use anyhow::{Context, Result, ensure};
use codecallnavigator_core::{
    freshness::{self, EventPayload, StreamEvent},
    query::Error,
    refresh::{self, Activity, Phase},
};
use std::{
    io::{BufRead, BufReader, Read},
    sync::{Arc, Mutex},
};

const MAX_FRAME: u64 = 64 * 1024;

#[derive(Default)]
pub(crate) struct Notifications {
    pub activity: Option<Activity>,
    pub publication: Option<Activity>,
    pub terminal: Option<Error>,
    pub exited: bool,
}

#[derive(Default)]
pub(super) struct Mailbox {
    notices: Notifications,
    pub closed: bool,
    terminal: Option<Error>,
}

impl Mailbox {
    pub fn take(&mut self) -> Notifications {
        let mut notices = std::mem::take(&mut self.notices);
        notices.terminal = self.terminal.clone();
        notices
    }

    pub fn result(&self) -> Result<()> {
        match &self.terminal {
            Some(error) => anyhow::bail!("{}: {}", error.code, error.message),
            None => Ok(()),
        }
    }

    fn event(&mut self, payload: EventPayload) -> Result<()> {
        match payload {
            EventPayload::Activity { activity } => {
                if matches!(activity.phase, Phase::Published { .. }) {
                    self.notices.publication = Some(activity.clone());
                }
                self.notices.activity = Some(activity);
            }
            EventPayload::Failed { error } => self.terminal = Some(error),
            _ => anyhow::bail!("Unexpected analysis-session observation payload"),
        }
        Ok(())
    }

    fn failed(&mut self, error: anyhow::Error) {
        if self.terminal.is_none() {
            self.terminal = Some(Error::new(
                "analysis_session_protocol",
                format!("{error:#}"),
            ));
        }
    }
}

pub(super) fn read(stdout: impl Read, mailbox: Arc<Mutex<Mailbox>>) {
    let result = parse(stdout, &mailbox);
    if let Ok(mut state) = mailbox.lock() {
        if let Err(error) = result {
            state.failed(error);
        }
        state.closed = true;
    }
}

fn parse(stdout: impl Read, mailbox: &Mutex<Mailbox>) -> Result<()> {
    let mut input = BufReader::new(stdout);
    let mut sequence = 0;
    let mut runs = 0;
    let mut terminal = false;
    loop {
        let mut bytes = Vec::new();
        let count = (&mut input)
            .take(MAX_FRAME + 1)
            .read_until(b'\n', &mut bytes)?;
        if count == 0 {
            return Ok(());
        }
        ensure!(
            count as u64 <= MAX_FRAME,
            "Analysis-session event exceeds 64 KiB"
        );
        let event: StreamEvent =
            serde_json::from_slice(&bytes).context("Malformed analysis-session event")?;
        ensure!(
            !terminal,
            "Analysis session sent events after terminal failure"
        );
        validate(&event, sequence, runs)?;
        sequence = event.sequence;
        if let EventPayload::Activity { activity } = &event.payload {
            runs = activity.runs_started;
        }
        terminal = matches!(event.payload, EventPayload::Failed { .. });
        mailbox
            .lock()
            .map_err(|_| anyhow::anyhow!("Analysis-session mailbox poisoned"))?
            .event(event.payload)?;
    }
}

fn validate(event: &StreamEvent, sequence: u64, runs: u64) -> Result<()> {
    ensure!(
        event.version == freshness::VERSION,
        "Unsupported analysis-session event version"
    );
    ensure!(
        event.sequence > sequence,
        "Analysis-session sequence did not increase"
    );
    if sequence == 0 {
        ensure!(
            event.sequence == 1,
            "Analysis-session initial sequence must be one"
        );
    }
    if let EventPayload::Activity { activity } = &event.payload {
        ensure!(
            activity.version == refresh::VERSION,
            "Unsupported analysis-session activity version"
        );
        ensure!(
            activity.runs_started >= runs,
            "Analysis-session run count decreased"
        );
        validate_publication(&activity.phase)?;
    }
    Ok(())
}

fn validate_publication(phase: &Phase) -> Result<()> {
    if let Phase::Published {
        generation,
        metrics,
        ..
    } = phase
    {
        ensure!(
            generation.len() == 64
                && generation
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
            "Invalid analysis-session publication digest"
        );
        ensure!(
            metrics.is_object(),
            "Invalid analysis-session publication metrics"
        );
    }
    Ok(())
}
