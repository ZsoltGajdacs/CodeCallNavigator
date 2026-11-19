//! Saved observations forward one selected session; the child owns refresh scheduling.
use super::session_child::{Notifications, SessionChild};
use codecallnavigator_core::{
    artifact,
    freshness::FreshnessObservation,
    query::{self, Error},
    refresh::{self, Activity, Phase, SessionRequest},
};

pub(super) struct Plan {
    pub request: SessionRequest,
    pub selection: String,
    pub trigger: Option<artifact::verification::Observation>,
}

pub(super) struct Coordinator {
    child: Option<SessionChild>,
    selection: Option<String>,
    validated_generation: Option<String>,
    activity: Activity,
    publication: Option<Activity>,
    terminal: Option<Error>,
    public_revision: Option<String>,
}

impl Default for Coordinator {
    fn default() -> Self {
        Self {
            child: None,
            selection: None,
            validated_generation: None,
            activity: Activity {
                version: refresh::VERSION,
                runs_started: 0,
                phase: Phase::Watching,
            },
            publication: None,
            terminal: None,
            public_revision: None,
        }
    }
}

impl Coordinator {
    pub fn activity(&self) -> Activity {
        self.activity.clone()
    }

    pub fn has_running_activity(&self) -> bool {
        self.publication.is_none() && matches!(self.activity.phase, Phase::Running { .. })
    }

    pub fn publication(&self) -> Option<&Activity> {
        self.publication.as_ref()
    }

    pub fn publication_delivered(&mut self) {
        self.publication = None;
    }

    pub fn take_terminal(&mut self) -> Option<Error> {
        self.terminal.take()
    }

    pub fn observe(
        &mut self,
        observation: &FreshnessObservation,
        plan: impl FnOnce() -> query::Result<Plan>,
    ) {
        self.public_revision = observation.input_revision.clone();
        self.poll();
        if self.terminal.is_some() {
            return;
        }
        // The child owns input scheduling. Re-resolve its saved selection only
        // when different graph bytes are observed, not for every input probe.
        if self.child.is_some()
            && self.validated_generation.as_ref() == Some(&observation.generation)
        {
            return;
        }
        let plan = match plan() {
            Ok(plan) => plan,
            Err(error) => {
                self.unavailable(error);
                return;
            }
        };
        if self.selection.as_ref() == Some(&plan.selection) {
            self.validated_generation = Some(observation.generation.clone());
            return;
        }
        if !scheduling_complete(observation, plan.trigger.as_ref()) {
            self.unavailable(auto_error(
                "incomplete_observation",
                incomplete_details(observation, plan.trigger.as_ref()),
            ));
            return;
        }
        self.start(
            &plan.request,
            Some((plan.selection, &observation.generation)),
        );
    }

    pub fn start_explicit(&mut self, request: &SessionRequest) {
        self.start(request, None);
    }

    fn start(&mut self, request: &SessionRequest, binding: Option<(String, &str)>) {
        if !self.shutdown() {
            return;
        }
        self.selection = binding.as_ref().map(|(selection, _)| selection.clone());
        match SessionChild::start(request) {
            Ok(child) => {
                self.validated_generation = binding.map(|(_, generation)| generation.into());
                self.child = Some(child);
            }
            Err(error)
                if error.is::<codecallnavigator_engine::process_ownership::CleanupFailure>() =>
            {
                self.terminal = Some(auto_error("cleanup_unconfirmed", format!("{error:#}")));
            }
            Err(error) => {
                self.unavailable(auto_error(
                    "analysis_session_start_failed",
                    format!("{error:#}"),
                ));
            }
        }
    }

    pub fn poll(&mut self) {
        let Some(child) = &mut self.child else {
            return;
        };
        match child.poll() {
            Ok(notices) => self.forward(notices),
            Err(error) => {
                self.terminal = Some(auto_error("analysis_session_failed", format!("{error:#}")))
            }
        }
    }

    fn forward(&mut self, notices: Notifications) {
        if let Some(activity) = notices.activity {
            self.activity = public_activity(activity, &self.public_revision);
        }
        if let Some(publication) = notices.publication {
            self.publication = Some(public_activity(publication, &self.public_revision));
        }
        if notices.terminal.is_some() {
            self.terminal = notices.terminal;
        }
        if notices.exited && self.terminal.is_none() {
            self.terminal = Some(auto_error(
                "analysis_session_exited",
                "Analysis session ended while its observer was active",
            ));
        }
    }

    fn unavailable(&mut self, error: Error) {
        // Invalidating a live binding permits a later valid selection to open a
        // new session. A failed start remains suppressed for this binding.
        if self.child.is_some() {
            self.selection = None;
            self.validated_generation = None;
        }
        if !self.shutdown() {
            return;
        }
        self.activity.phase = Phase::Unavailable { error };
    }

    pub fn cancel(&mut self) {
        if let Some(child) = &mut self.child
            && let Err(error) = child.cancel_attempt()
        {
            self.terminal = Some(auto_error(
                "analysis_session_control_failed",
                format!("{error:#}"),
            ));
        }
    }

    pub fn shutdown(&mut self) -> bool {
        let Some(mut child) = self.child.take() else {
            return true;
        };
        if let Err(error) = child.shutdown() {
            let code = if error.is::<codecallnavigator_engine::process_ownership::CleanupFailure>()
            {
                "cleanup_unconfirmed"
            } else {
                "analysis_session_failed"
            };
            self.terminal = Some(auto_error(code, format!("{error:#}")));
            return false;
        }
        true
    }
}

impl Drop for Coordinator {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

// Public observations may have no complete revision while the session has a
// valid private source trigger. Never expose that trigger as public coverage.
fn public_activity(mut activity: Activity, revision: &Option<String>) -> Activity {
    match &mut activity.phase {
        Phase::Debouncing { input_revision, .. }
        | Phase::Running { input_revision, .. }
        | Phase::Busy { input_revision, .. }
        | Phase::Published { input_revision, .. }
        | Phase::Failed { input_revision, .. }
        | Phase::Cancelled { input_revision, .. } => *input_revision = revision.clone(),
        _ => {}
    }
    activity
}

fn incomplete_details(
    observation: &FreshnessObservation,
    trigger: Option<&artifact::verification::Observation>,
) -> String {
    let reasons = trigger.map_or(observation.reasons.as_slice(), |value| {
        value.report.reasons.as_slice()
    });
    let blockers = reasons
        .iter()
        .filter(|reason| reason.is_blocker() && reason.code != "unmapped_input")
        .collect::<Vec<_>>();
    let mut message = String::from(
        "Automatic refresh is blocked because required input coverage is incomplete. The previous graph remains available.\n",
    );
    let mut retained = 0;
    for reason in &blockers {
        let detail = reason.describe();
        // Leave room for the omitted-count and recovery explanation within the
        // activity error limit. Full reasons remain available through verify.
        if message.chars().count() + detail.chars().count() > 3400 {
            break;
        }
        message.push_str(&detail);
        message.push('\n');
        retained += 1;
    }
    let total = blockers.len();
    let omitted = total - retained
        + if trigger.is_none() {
            observation.omitted_reasons
        } else {
            0
        };
    if omitted > 0 {
        message.push_str(&format!(
            "{omitted} further reasons omitted. Use CLI verify for the complete report.\n"
        ));
    }
    if total == 0 {
        message.push_str("The recorded input inventory or graph contract is incomplete. Perform a manual Refresh with a compatible engine to regenerate it.\n");
    }
    message.push_str("Retry freshness checks observation again; it does not rebuild the graph.");
    message
}

fn scheduling_complete(
    observation: &FreshnessObservation,
    trigger: Option<&artifact::verification::Observation>,
) -> bool {
    observation.contract_valid
        && !observation
            .reasons
            .iter()
            .any(|reason| reason.code == "legacy_input_policy")
        && trigger.map_or(observation.coverage.complete, |trigger| trigger.complete)
}

fn auto_error(code: &str, message: impl Into<String>) -> Error {
    let message = message.into();
    let mut bounded = message.chars().take(4096).collect::<String>();
    if bounded.len() < message.len() {
        bounded.push_str(" … [truncated]");
    }
    Error::new(code, bounded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codecallnavigator_core::{
        artifact::verification::{InputStatus, Observation, Report},
        freshness::{Coverage, CoverageKind, Mode},
    };

    fn incomplete_public() -> FreshnessObservation {
        FreshnessObservation {
            version: 1,
            generation: "generation".into(),
            input_revision: Some("full".into()),
            contract_valid: true,
            input_status: InputStatus::Stale,
            reasons: vec![],
            reasons_total: 0,
            omitted_reasons: 0,
            coverage: Coverage {
                kind: CoverageKind::ObservedInputs,
                complete: false,
                certifies_build: false,
            },
            mode: Mode::default(),
            verification_milliseconds: 0.0,
        }
    }

    #[test]
    fn incomplete_coverage_explains_required_path_and_recovery_without_starting_analysis() {
        let mut observation = incomplete_public();
        observation
            .reasons
            .push(artifact::verification::Reason::new("unmapped_input", None));
        observation
            .reasons
            .push(artifact::verification::Reason::new(
                "unreadable_input",
                Some("@repo/.required/source.java".into()),
            ));
        observation.reasons_total = 2;
        let details = incomplete_details(&observation, None);
        assert!(details.contains("@repo/.required/source.java"));
        assert!(details.contains("permissions"));
        assert!(details.contains("does not rebuild"));
        assert!(details.contains("previous graph remains available"));
        assert!(!details.contains("unmapped_input"));
        assert!(!details.contains("further reasons omitted"));
    }

    #[test]
    fn validated_trigger_coverage_survives_incomplete_public_compiler_outputs() {
        let mut public = incomplete_public();
        let mut trigger = Observation {
            report: Report {
                version: 1,
                contract_valid: true,
                input_status: InputStatus::Match,
                digest: "generation".into(),
                reasons: vec![],
            },
            input_revision: Some("trigger".into()),
            complete: true,
        };
        assert!(scheduling_complete(&public, Some(&trigger)));
        assert!(!scheduling_complete(&public, None));
        trigger.complete = false;
        assert!(!scheduling_complete(&public, Some(&trigger)));
        trigger.complete = true;
        public.contract_valid = false;
        assert!(!scheduling_complete(&public, Some(&trigger)));
    }

    #[test]
    fn pending_source_changes_keep_nullable_public_revision_and_valid_trigger() {
        let activity = Activity {
            version: 1,
            runs_started: 2,
            phase: Phase::Debouncing {
                input_revision: Some("private-trigger".into()),
                remaining_milliseconds: 200,
            },
        };
        let mapped = public_activity(activity, &None);
        assert_eq!(mapped.runs_started, 2);
        assert!(matches!(
            mapped.phase,
            Phase::Debouncing {
                input_revision: None,
                remaining_milliseconds: 200
            }
        ));
        assert_eq!(
            serde_json::to_value(mapped).unwrap()["input_revision"],
            serde_json::Value::Null
        );
    }

    #[test]
    fn published_metrics_survive_observing_their_own_generation() {
        let published = Activity {
            version: 1,
            runs_started: 1,
            phase: Phase::Published {
                run: 1,
                input_revision: Some("private".into()),
                generation: "new".into(),
                roots: Vec::new(),
                analysis_milliseconds: 123.0,
                metrics: serde_json::json!({"complete":true}),
            },
        };
        let watching = Activity {
            version: 1,
            runs_started: 1,
            phase: Phase::Watching,
        };
        let mut coordinator = Coordinator::default();
        coordinator.forward(Notifications {
            activity: Some(watching),
            publication: Some(published),
            ..Default::default()
        });
        assert!(matches!(coordinator.activity().phase, Phase::Watching));
        assert!(
            matches!(&coordinator.publication().unwrap().phase, Phase::Published { generation, analysis_milliseconds: 123.0, input_revision: None, metrics, .. } if generation == "new" && metrics["complete"] == true)
        );
        coordinator.publication_delivered();
        assert!(coordinator.publication().is_none());
    }
}
