//! Session authorization and presentation for the loaded project's engine coordinator.
use crate::engine_client::Descriptor;
use codecallnavigator::refresh::{Activity, Phase};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Authorization {
    project: Option<PathBuf>,
    pub activity: Option<Activity>,
    adoption_attempt: Option<String>,
}

impl Authorization {
    pub fn enabled(&self, project: &Path) -> bool {
        self.project.as_deref() == Some(project)
    }

    pub fn toggle(&mut self, descriptor: &Descriptor) {
        self.project = (!self.enabled(&descriptor.project)).then(|| descriptor.project.clone());
        self.activity = None;
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

    pub fn can_cancel(&self) -> bool {
        self.activity.as_ref().is_some_and(|activity| {
            matches!(
                activity.phase,
                Phase::Running { .. } | Phase::Busy { .. } | Phase::Debouncing { .. }
            )
        })
    }

    pub fn details(&self) -> Option<&str> {
        match &self.activity.as_ref()?.phase {
            Phase::Failed { error, .. } | Phase::Unavailable { error } => Some(&error.message),
            _ => None,
        }
    }

    pub fn message(&self) -> Option<String> {
        let activity = self.activity.as_ref()?;
        Some(match &activity.phase {
            Phase::Watching => "Automatic refresh is watching saved inputs".into(),
            Phase::Debouncing { .. } => "Automatic refresh is waiting for edits to settle…".into(),
            Phase::Running { .. } => {
                "Automatically analyzing saved changes · previous graph remains available".into()
            }
            Phase::Busy { .. } => "Automatic refresh is waiting for another analysis".into(),
            Phase::Published { .. } => "Automatic refresh published a saved graph".into(),
            Phase::Failed { .. } => {
                "Automatic refresh failed · waiting for another saved edit".into()
            }
            Phase::Cancelled { reason, .. } => {
                format!("Automatic refresh cancelled · {}", cancel_message(*reason))
            }
            Phase::Unavailable { error } => {
                format!("Automatic refresh unavailable · {}", error.message)
            }
        })
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
