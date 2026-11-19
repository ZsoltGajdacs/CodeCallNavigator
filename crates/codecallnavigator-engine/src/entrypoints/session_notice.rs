//! Bounded delivery for continuous analysis notices, independently of attempt scheduling.
use super::observation_transport::Output;
use codecallnavigator_core::{
    freshness::{self, EventPayload, StreamEvent},
    query::Error,
    refresh::{self, Activity, Phase},
};
use std::{
    sync::{Mutex, mpsc},
    thread,
    time::Duration,
};

pub(super) struct Notice<'a> {
    output: &'a Output,
    human_progress: bool,
    sequence: u64,
    pending: Option<StreamEvent>,
    publication: Option<StreamEvent>,
}

impl<'a> Notice<'a> {
    pub(super) fn new(output: &'a Output, human_progress: bool) -> Self {
        Self {
            output,
            human_progress,
            sequence: 0,
            pending: None,
            publication: None,
        }
    }

    pub(super) fn activity(&mut self, runs_started: u64, phase: Phase) {
        let published = matches!(phase, Phase::Published { .. });
        let event = StreamEvent {
            sequence: 0,
            version: freshness::VERSION,
            payload: EventPayload::Activity {
                activity: Activity {
                    version: refresh::VERSION,
                    runs_started,
                    phase,
                },
            },
        };
        if published {
            self.pending = None;
            self.publication = Some(event);
        } else {
            self.pending = Some(event);
        }
        self.flush();
    }

    pub(super) fn flush(&mut self) {
        for slot in [&mut self.publication, &mut self.pending] {
            if let Some(event) = slot.as_mut() {
                event.sequence = self.sequence + 1;
                if !self.output.offer(event.clone()) {
                    break;
                }
                self.sequence += 1;
                *slot = None;
            }
        }
    }

    pub(super) fn failed(&mut self, error: Error) {
        self.pending = None;
        self.flush();
        self.output.finish(StreamEvent {
            sequence: self.sequence + 1,
            version: freshness::VERSION,
            payload: EventPayload::Failed { error },
        });
    }
}

impl Notice<'_> {
    pub(super) fn during_attempt<T>(
        &mut self,
        run: u64,
        phase: &Phase,
        analyze: impl FnOnce(&dyn Fn(String)) -> T,
    ) -> T {
        self.activity(run, phase.clone());
        let human_progress = self.human_progress;
        let notice = Mutex::new(self);
        thread::scope(|scope| {
            // The sender drops even if analysis unwinds. Retry any pending stage
            // while its producer is busy, using only nonblocking output offers.
            let (_finished, retry) = mpsc::sync_channel::<()>(0);
            let notices = &notice;
            scope.spawn(move || {
                while retry.recv_timeout(Duration::from_millis(20))
                    == Err(mpsc::RecvTimeoutError::Timeout)
                {
                    notices.lock().expect("session notice lock").flush();
                }
            });
            analyze(&|message| {
                if human_progress {
                    eprintln!("{message}");
                }
                let Some(phase) = running_stage(phase.clone(), &message) else {
                    return;
                };
                notice
                    .lock()
                    .expect("session notice lock")
                    .activity(run, phase);
            })
        })
    }
}

fn running_stage(mut phase: Phase, message: &str) -> Option<Phase> {
    let Phase::Running {
        progress_message, ..
    } = &mut phase
    else {
        return None;
    };
    let message = message.trim_matches(|ch: char| ch.is_whitespace() || ch.is_control());
    if message.is_empty() {
        return None;
    }
    *progress_message = Some(
        message
            .chars()
            .take(refresh::MAX_PROGRESS_MESSAGE_CHARS)
            .map(|ch| if ch.is_control() { ' ' } else { ch })
            .collect(),
    );
    Some(phase)
}
