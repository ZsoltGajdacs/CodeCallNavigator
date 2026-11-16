//! UI visibility owns the idle deadline; the CLI only executes resource requests.
use crate::{
    app::State,
    background_work::{Tracked, Workers},
    engine_client::{Descriptor, QueryContext},
};
use rune::{Latest, Tx, Waker};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

const IDLE_SECONDS: f64 = 60.0;

#[derive(Default)]
struct Idle {
    used: bool,
    deadline: Option<f64>,
}

enum Decision {
    Retain,
    WakeAfter(Duration),
    Release,
}

impl Idle {
    fn active(&mut self) {
        self.used = true;
        self.deadline = None;
    }

    fn observe(&mut self, now: f64, visible: bool) -> Decision {
        if visible {
            self.active();
            return Decision::Retain;
        }
        if !self.used {
            return Decision::Retain;
        }
        let deadline = *self.deadline.get_or_insert(now + IDLE_SECONDS);
        if now < deadline {
            Decision::WakeAfter(Duration::from_secs_f64(deadline - now))
        } else {
            self.used = false;
            self.deadline = None;
            Decision::Release
        }
    }
}

#[derive(Default)]
pub struct Workspace {
    idle: Idle,
    revision: Arc<AtomicU64>,
    job: Latest<Tracked<Output>>,
    workers: Arc<Workers>,
}

struct Output {
    descriptor: Descriptor,
    revision: u64,
    result: Result<(), String>,
}

impl Workspace {
    /// Called synchronously before opening a flow, before a background query can
    /// queue behind an idle release on the CLI connection.
    pub(crate) fn activity(&mut self) {
        self.idle.active();
        self.revision.fetch_add(1, Ordering::AcqRel);
    }

    pub(crate) fn set_waker(&self, waker: Waker) {
        self.job.set_waker(waker.clone());
        self.workers.set_waker(waker);
    }

    pub(crate) fn stopped(&self) -> bool {
        self.workers.stopped()
    }

    pub(crate) fn retire(&mut self, context: Option<QueryContext>) {
        self.idle = Idle::default();
        self.request(context);
    }

    fn request(&mut self, context: Option<QueryContext>) {
        let expected = self.revision.fetch_add(1, Ordering::AcqRel) + 1;
        let Some(context) = context else {
            return;
        };
        let revision = self.revision.clone();
        let input = self.workers.track((context, revision));
        self.job.request(input, move |input| {
            input.map(|(context, revision)| Output {
                descriptor: context.descriptor.clone(),
                revision: expected,
                result: context
                    .release_flows_if_current(&revision, expected, &AtomicBool::new(false))
                    .map_err(|error| format!("{error:#}")),
            })
        });
    }

    fn take_error(&mut self, context: Option<&QueryContext>) -> Option<String> {
        let output = self.job.take()?;
        let output = output.value;
        if output.revision != self.revision.load(Ordering::Acquire)
            || context.is_none_or(|context| context.descriptor != output.descriptor)
        {
            return None;
        }
        output.result.err()
    }
}

pub(crate) fn poll(state: &mut State, now: f64, tx: &mut Tx) -> bool {
    let mut changed = false;
    if let Some(error) = state
        .flow_residency
        .take_error(state.query_context.as_ref())
    {
        state.warning = format!("Could not release unused flow data: {error}");
        changed = true;
    }
    if state.closing {
        return changed;
    }
    let visible = state.flow_workspace.current.is_some() || state.flow_comparison.controls.visible;
    match state.flow_residency.idle.observe(now, visible) {
        Decision::Retain => (),
        Decision::WakeAfter(delay) => tx.wake_after(delay),
        Decision::Release => {
            state.flow_workspace.release_hidden();
            state.flow_comparison.release_hidden();
            state.flow_residency.request(state.query_context.clone());
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_starts_after_last_view_hides_and_releases_once() {
        let mut idle = Idle::default();
        assert!(matches!(idle.observe(0.0, false), Decision::Retain));
        assert!(matches!(idle.observe(10.0, true), Decision::Retain));
        assert!(matches!(idle.observe(500.0, true), Decision::Retain));
        assert!(
            matches!(idle.observe(501.0, false), Decision::WakeAfter(delay) if delay == Duration::from_secs(60))
        );
        assert!(matches!(idle.observe(560.9, false), Decision::WakeAfter(_)));
        assert!(matches!(idle.observe(561.0, false), Decision::Release));
        assert!(matches!(idle.observe(900.0, false), Decision::Retain));
    }

    #[test]
    fn reopening_cancels_deadline_and_invalidates_queued_release() {
        let mut workspace = Workspace::default();
        workspace.activity();
        assert!(matches!(
            workspace.idle.observe(0.0, false),
            Decision::WakeAfter(_)
        ));
        let queued = workspace.revision.load(Ordering::Acquire);
        workspace.activity();
        assert_ne!(workspace.revision.load(Ordering::Acquire), queued);
        assert!(matches!(
            workspace.idle.observe(60.0, true),
            Decision::Retain
        ));
        assert!(
            matches!(workspace.idle.observe(61.0, false), Decision::WakeAfter(delay) if delay == Duration::from_secs(60))
        );
        assert!(matches!(
            workspace.idle.observe(120.0, false),
            Decision::WakeAfter(_)
        ));
        assert!(matches!(
            workspace.idle.observe(121.0, false),
            Decision::Release
        ));
    }
}
