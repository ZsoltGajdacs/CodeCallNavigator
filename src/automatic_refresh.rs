//! Session authorization and presentation for the loaded project's engine coordinator.
use crate::engine_client::Descriptor;
use codecallnavigator::refresh::{Activity, Phase};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Authorization {
    project: Option<PathBuf>,
    pub activity: Option<Activity>,
    stage_since: Option<Instant>,
    displayed_stage_second: Option<u64>,
    adoption_attempt: Option<String>,
}

impl Authorization {
    pub fn enabled(&self, project: &Path) -> bool {
        self.project.as_deref() == Some(project)
    }

    pub fn toggle(&mut self, descriptor: &Descriptor) {
        self.project = (!self.enabled(&descriptor.project)).then(|| descriptor.project.clone());
        self.clear_activity();
        self.adoption_attempt = None;
    }

    pub fn reconcile(&mut self, descriptor: Option<&Descriptor>) {
        if self
            .project
            .as_ref()
            .is_some_and(|authorized| descriptor.is_none_or(|loaded| &loaded.project != authorized))
        {
            *self = Self::default();
        }
    }

    pub fn adoption_due(&mut self, descriptor: &Descriptor, generation: &str) -> bool {
        if !self.enabled(&descriptor.project)
            || self.adoption_attempt.as_deref() == Some(generation)
        {
            return false;
        }
        self.adoption_attempt = Some(generation.into());
        true
    }

    pub fn retry_available(&self) -> bool {
        self.activity
            .as_ref()
            .is_some_and(|activity| matches!(activity.phase, Phase::Unavailable { .. }))
    }

    pub fn accept_activity(&mut self, activity: Activity, now: Instant) {
        let next = running_stage(&activity);
        if next != self.activity.as_ref().and_then(running_stage) {
            self.stage_since = next.map(|_| now);
        }
        self.activity = Some(activity);
    }

    pub fn clear_activity(&mut self) {
        self.activity = None;
        self.stage_since = None;
    }

    pub fn clear_active_work(&mut self) {
        if self.can_cancel() {
            self.clear_activity();
        }
    }

    pub fn stage_elapsed(&self, now: Instant) -> Option<Duration> {
        self.stage_since
            .map(|since| now.saturating_duration_since(since))
    }

    pub fn poll_stage_clock(&mut self, visible: bool, now: Instant, tx: &mut rune::Tx) -> bool {
        let elapsed = visible.then(|| self.stage_elapsed(now)).flatten();
        if let Some(elapsed) = elapsed {
            tx.wake_after(Duration::from_nanos(
                1_000_000_000 - u64::from(elapsed.subsec_nanos()),
            ));
        }
        let second = elapsed.map(|elapsed| elapsed.as_secs());
        std::mem::replace(&mut self.displayed_stage_second, second) != second
    }

    pub fn can_cancel(&self) -> bool {
        self.activity.as_ref().is_some_and(|activity| {
            matches!(
                activity.phase,
                Phase::Running { .. } | Phase::Busy { .. } | Phase::Debouncing { .. }
            )
        })
    }

    pub fn details(&self) -> Option<String> {
        match &self.activity.as_ref()?.phase {
            Phase::Unavailable { error } => Some(format!("{}: {}", error.code, error.message)),
            _ => None,
        }
    }

    pub fn message(&self) -> Option<String> {
        let activity = self.activity.as_ref()?;
        Some(match &activity.phase {
            Phase::Watching => "Automatic refresh is watching saved inputs".into(),
            Phase::Debouncing { .. } => "Automatic refresh is waiting for edits to settle…".into(),
            Phase::Running {
                progress_message, ..
            } => progress_message
                .clone()
                .unwrap_or_else(|| "Analyzing saved inputs…".into()),
            Phase::Busy { .. } => "Automatic refresh is waiting for another analysis".into(),
            Phase::Published { retention, .. } => retention.as_ref().map_or_else(
                || "Automatic refresh published a saved graph".into(),
                crate::analysis_feedback::retention_message,
            ),
            Phase::Failed { .. } => {
                "Automatic refresh failed · waiting for another saved edit".into()
            }
            Phase::Cancelled { reason, .. } => {
                format!("Automatic refresh cancelled · {}", cancel_message(*reason))
            }
            Phase::Unavailable { .. } => {
                "Automatic refresh is blocked · open details for the cause and recovery".into()
            }
        })
    }
}

fn running_stage(activity: &Activity) -> Option<(u64, Option<&str>)> {
    match &activity.phase {
        Phase::Running {
            run,
            progress_message,
            ..
        } => Some((*run, progress_message.as_deref())),
        _ => None,
    }
}

fn cancel_message(reason: codecallnavigator::refresh::CancelReason) -> &'static str {
    use codecallnavigator::refresh::CancelReason;
    match reason {
        CancelReason::Superseded => "newer saved changes are pending",
        CancelReason::UserCancelled => "waiting for another edit",
        CancelReason::Shutdown => "monitor stopped",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn descriptor(project: &str) -> Descriptor {
        Descriptor {
            project: project.into(),
            ..Default::default()
        }
    }

    #[test]
    fn running_stage_elapsed_tracks_attempt_and_message() {
        let mut state = Authorization::default();
        let start = Instant::now();
        let second = Duration::from_secs(1);
        let running = |run, stage: Option<&str>| {
            Activity::new(Phase::Running {
                run,
                input_revision: None,
                debounce_milliseconds: 300.0,
                wait_milliseconds: 0.0,
                progress_message: stage.map(str::to_owned),
            })
        };
        state.accept_activity(running(1, None), start);
        assert_eq!(state.message().as_deref(), Some("Analyzing saved inputs…"));
        let stage = Some("Extracting TypeScript");
        state.accept_activity(running(1, stage), start + second);
        assert_eq!(state.message().as_deref(), stage);
        state.accept_activity(running(1, stage), start + second * 2);
        assert_eq!(state.stage_elapsed(start + second * 3), Some(second * 2));
        state.accept_activity(running(2, stage), start + second * 3);
        assert_eq!(
            state.stage_elapsed(start + second * 3),
            Some(Duration::ZERO)
        );
        state.accept_activity(Activity::new(Phase::Watching), start + second * 4);
        assert_eq!(state.stage_elapsed(start + second * 4), None);
    }

    #[test]
    fn opt_in_follows_primary_project_but_not_roots_or_generation() {
        let mut authorization = Authorization::default();
        let mut loaded = descriptor("/loaded");
        assert!(!authorization.enabled(&loaded.project));
        authorization.toggle(&loaded);
        loaded.roots.push(("shared".into(), "/shared".into()));
        loaded.generation = Some("a".repeat(64));
        authorization.reconcile(Some(&loaded));
        assert!(authorization.enabled(&loaded.project));
        assert!(authorization.adoption_due(&loaded, &"b".repeat(64)));
        assert!(!authorization.adoption_due(&loaded, &"b".repeat(64)));
        authorization.reconcile(Some(&descriptor("/another")));
        assert!(!authorization.enabled(&loaded.project));
        authorization.reconcile(Some(&loaded));
        assert!(
            !authorization.enabled(&loaded.project),
            "Returning does not silently restore authorization"
        );
    }
}
