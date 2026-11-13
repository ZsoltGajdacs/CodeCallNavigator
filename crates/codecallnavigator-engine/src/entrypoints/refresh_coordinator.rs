//! Engine-owned opt-in refresh scheduling over fresh, owned one-shot processes.
use super::{
    refresh_child::{Finished, Job, Outcome},
    refresh_policy::{Key, Pending, Policy},
};
use codecallnavigator_core::{
    artifact::{self, CoordinatorLease},
    freshness::FreshnessObservation,
    protocol::{self, AnalysisFailureCode},
    query::{self, Error},
    refresh::{self, Activity, CancelReason, Phase},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub(super) struct Plan {
    pub request: protocol::Request,
    pub generation: String,
    pub selection: String,
    pub project: PathBuf,
    pub trigger: Option<artifact::verification::Observation>,
}
struct Active {
    job: Job,
    pending: Pending,
    run: u64,
}

pub(super) struct Coordinator {
    lease: Option<CoordinatorLease>,
    policy: Policy,
    active: Option<Active>,
    runs: u64,
    phase: Phase,
    retry_at: Instant,
    backoff: u64,
    baseline: Option<(String, String)>,
    observed_revision: Option<String>,
}

impl Default for Coordinator {
    fn default() -> Self {
        Self {
            lease: None,
            policy: Policy::default(),
            active: None,
            runs: 0,
            phase: Phase::Watching,
            retry_at: Instant::now(),
            backoff: 1000,
            baseline: None,
            observed_revision: None,
        }
    }
}

impl Coordinator {
    pub fn activity(&self) -> Activity {
        Activity {
            version: refresh::VERSION,
            runs_started: self.runs,
            phase: self.phase.clone(),
        }
    }

    pub fn observe(&mut self, observation: &FreshnessObservation, plan: query::Result<Plan>) {
        self.poll();
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                self.unavailable(error);
                return;
            }
        };
        let revision = plan
            .trigger
            .as_ref()
            .map_or(&observation.input_revision, |trigger| {
                &trigger.input_revision
            })
            .clone();
        let complete = scheduling_complete(observation, plan.trigger.as_ref());
        let Some(revision) = revision.filter(|_| complete) else {
            self.unavailable(auto_error(
                "incomplete_observation",
                "Automatic refresh requires readable recorded input scopes",
            ));
            return;
        };
        let stale = plan
            .trigger
            .as_ref()
            .map_or(observation.input_status, |trigger| {
                trigger.report.input_status
            })
            == artifact::verification::InputStatus::Stale;
        let key = Key {
            generation: plan.generation.clone(),
            selection: plan.selection,
            revision,
        };
        let changed = self.update_inputs(key, observation, stale);
        if self.active.is_some() {
            return;
        }
        if !self.acquire_lease(&plan.project) {
            return;
        }
        if self.policy.pending.is_none() {
            self.settled(changed, &observation.generation);
            return;
        }
        self.launch(plan.request);
    }

    fn settled(&mut self, changed: bool, observed_generation: &str) {
        let own_publication = matches!(&self.phase, Phase::Published { generation, .. } if generation == observed_generation);
        if own_publication {
            return;
        }
        if changed || matches!(self.phase, Phase::Published { .. }) {
            self.phase = Phase::Watching;
        }
    }

    fn update_inputs(&mut self, key: Key, observation: &FreshnessObservation, stale: bool) -> bool {
        self.observed_revision = observation.input_revision.clone();
        if let Some(active) = &self.active
            && key.supersedes(&active.pending.key)
        {
            active.job.cancel(CancelReason::Superseded);
        }
        if self
            .baseline
            .as_ref()
            .is_none_or(|(generation, _)| generation != &key.generation)
        {
            self.baseline = Some((key.generation.clone(), key.revision.clone()));
        }
        let changed_bytes = self
            .baseline
            .as_ref()
            .is_some_and(|(_, revision)| revision != &key.revision);
        let changed = self
            .policy
            .observe(key, stale || changed_bytes, Instant::now());
        if let Some(pending) = &mut self.policy.pending {
            pending.observed_revision = self.observed_revision.clone();
        }
        changed
    }

    fn acquire_lease(&mut self, project: &std::path::Path) -> bool {
        if self.lease.is_some() {
            return true;
        }
        if Instant::now() < self.retry_at {
            return false;
        }
        match CoordinatorLease::acquire(project) {
            Ok(lease) => {
                self.lease = Some(lease);
                self.backoff = 1000;
                self.phase = Phase::Watching;
                true
            }
            Err(error) => {
                let code = if error.is::<artifact::LockBusy>() {
                    "coordinator_busy"
                } else {
                    "coordinator_lock_unavailable"
                };
                self.retry_at = Instant::now() + Duration::from_millis(self.backoff);
                self.backoff = (self.backoff * 2).min(5000);
                self.phase = Phase::Unavailable {
                    error: auto_error(code, format!("{error:#}")),
                };
                false
            }
        }
    }

    fn launch(&mut self, request: protocol::Request) {
        let now = Instant::now();
        let Some(pending) = self.policy.due(now) else {
            if let Some(pending) = &self.policy.pending {
                self.phase = Phase::Debouncing {
                    input_revision: pending.observed_revision.clone(),
                    remaining_milliseconds: refresh::DEBOUNCE_MILLISECONDS
                        .saturating_sub(now.duration_since(pending.changed).as_millis() as u64),
                };
            }
            return;
        };
        if now < self.retry_at {
            return;
        }
        let project = match &request {
            protocol::Request::Analyze { project, .. } => project,
            _ => unreachable!("automatic request is analysis"),
        };
        match artifact::AnalysisLock::acquire(project) {
            Ok(available) => drop(available),
            Err(error) if error.is::<artifact::LockBusy>() => {
                self.busy(&pending, None);
                return;
            }
            Err(error) => {
                self.unavailable(auto_error(
                    "analysis_lock_unavailable",
                    format!("{error:#}"),
                ));
                return;
            }
        }
        match Job::start(&request) {
            Ok(job) => {
                self.runs += 1;
                self.phase = Phase::Running {
                    run: self.runs,
                    input_revision: pending.observed_revision.clone(),
                    debounce_milliseconds: refresh::DEBOUNCE_MILLISECONDS as f64,
                    wait_milliseconds: pending.queue_milliseconds(now),
                };
                self.active = Some(Active {
                    job,
                    pending: self.policy.take().expect("due refresh"),
                    run: self.runs,
                });
            }
            Err(error) => {
                self.policy.suppress(pending.key.clone());
                self.phase = Phase::Unavailable {
                    error: auto_error("analysis_start_failed", format!("{error:#}")),
                };
            }
        }
    }

    fn busy(&mut self, pending: &Pending, run: Option<u64>) {
        self.retry_at = Instant::now() + Duration::from_millis(self.backoff);
        self.phase = Phase::Busy {
            run,
            input_revision: pending.observed_revision.clone(),
            retry_milliseconds: self.backoff,
            wait_milliseconds: pending.queue_milliseconds(Instant::now()),
        };
        self.backoff = (self.backoff * 2).min(5000);
    }

    fn poll(&mut self) {
        let Some(finished) = self.active.as_mut().and_then(|active| active.job.take()) else {
            return;
        };
        let active = self.active.take().expect("finished analysis");
        self.finish(active, finished);
    }

    fn finish(&mut self, active: Active, finished: Finished) {
        let revision = active.pending.observed_revision.clone();
        let duration = finished.milliseconds;
        match finished.outcome {
            Outcome::Published {
                generation,
                metrics,
            } => {
                self.policy.suppress(active.pending.key);
                self.backoff = 1000;
                self.phase = Phase::Published {
                    run: active.run,
                    input_revision: revision,
                    generation,
                    analysis_milliseconds: duration,
                    metrics,
                };
            }
            Outcome::Failed {
                code: AnalysisFailureCode::Busy,
                ..
            } => {
                self.policy.retry(active.pending.clone());
                self.busy(&active.pending, Some(active.run));
            }
            Outcome::Failed {
                code: AnalysisFailureCode::GenerationChanged | AnalysisFailureCode::InputsChanged,
                ..
            } => {
                self.policy.suppress(active.pending.key);
                self.phase = Phase::Cancelled {
                    run: Some(active.run),
                    input_revision: revision,
                    reason: CancelReason::Superseded,
                    analysis_milliseconds: duration,
                };
            }
            Outcome::Failed { code, message } => {
                self.policy.suppress(active.pending.key);
                self.phase = Phase::Failed {
                    run: active.run,
                    input_revision: revision,
                    error: auto_error(failure_name(code), message),
                    analysis_milliseconds: duration,
                };
            }
            Outcome::CleanupFailed(message) => {
                self.policy.suppress(active.pending.key);
                self.phase = Phase::Failed {
                    run: active.run,
                    input_revision: revision,
                    error: auto_error("cleanup_unconfirmed", message),
                    analysis_milliseconds: duration,
                };
            }
            Outcome::Cancelled(reason) => {
                self.phase = Phase::Cancelled {
                    run: Some(active.run),
                    input_revision: revision,
                    reason,
                    analysis_milliseconds: duration,
                };
            }
        }
    }

    fn unavailable(&mut self, error: Error) {
        if let Some(active) = &self.active {
            active.job.cancel(CancelReason::Superseded);
        }
        if self.active.is_none() {
            self.lease = None;
        }
        self.policy.unavailable();
        self.phase = Phase::Unavailable { error };
    }

    pub fn cancel(&mut self, reason: CancelReason) {
        let pending = self.policy.cancel();
        if let Some(active) = &self.active {
            active.job.cancel(reason);
        } else if pending.is_some() {
            self.phase = Phase::Cancelled {
                run: None,
                input_revision: self.observed_revision.clone(),
                reason,
                analysis_milliseconds: 0.0,
            };
        }
    }

    pub fn shutdown(&mut self) -> bool {
        let active = self.active.is_some();
        self.cancel(CancelReason::Shutdown);
        while self.active.is_some() {
            self.poll();
            if self.active.is_some() {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        self.lease = None;
        !active
            || !matches!(&self.phase, Phase::Failed { error, .. } if error.code == "cleanup_unconfirmed")
    }
}

impl Drop for Coordinator {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn scheduling_complete(
    observation: &FreshnessObservation,
    trigger: Option<&artifact::verification::Observation>,
) -> bool {
    observation.contract_valid
        && trigger.map_or(observation.coverage.complete, |trigger| trigger.complete)
}

fn failure_name(code: AnalysisFailureCode) -> &'static str {
    match code {
        AnalysisFailureCode::Busy => "analysis_busy",
        AnalysisFailureCode::InputsChanged => "inputs_changed",
        AnalysisFailureCode::GenerationChanged => "generation_changed",
        AnalysisFailureCode::Cancelled => "cancelled",
        AnalysisFailureCode::Failed => "analysis_failed",
    }
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

    fn queued_plan(
        project: &std::path::Path,
        public: &FreshnessObservation,
        trigger_revision: &str,
    ) -> Plan {
        Plan {
            project: project.into(),
            generation: public.generation.clone(),
            selection: "selected".into(),
            request: protocol::Request::Analyze {
                version: 1,
                project: project.into(),
                roots: vec![],
                targets: vec![],
                infer_linked_roots: false,
                failure_version: Some(1),
                expected_generation: Some(public.generation.clone()),
            },
            trigger: Some(Observation {
                report: Report {
                    version: 1,
                    contract_valid: true,
                    input_status: InputStatus::Stale,
                    digest: public.generation.clone(),
                    reasons: vec![],
                },
                input_revision: Some(trigger_revision.repeat(64)),
                complete: true,
            }),
        }
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
        let mut public = incomplete_public();
        public.input_revision = None;
        let project = tempfile::tempdir().unwrap();
        let mut coordinator = Coordinator::default();
        coordinator.observe(&public, Ok(queued_plan(project.path(), &public, "a")));
        assert!(matches!(
            coordinator.phase,
            Phase::Debouncing {
                input_revision: None,
                ..
            }
        ));
        coordinator.observe(&public, Ok(queued_plan(project.path(), &public, "b")));
        assert!(matches!(
            coordinator.phase,
            Phase::Debouncing {
                input_revision: None,
                ..
            }
        ));
        let pending = coordinator.policy.pending.as_ref().unwrap();
        assert_eq!(pending.key.revision, "b".repeat(64));
        assert!(
            coordinator
                .policy
                .due(Instant::now() + Duration::from_secs(1))
                .is_some()
        );
        assert_eq!(
            serde_json::to_value(coordinator.activity()).unwrap()["input_revision"],
            serde_json::Value::Null
        );
        assert_eq!(
            coordinator.runs, 0,
            "The isolated debounce transition starts no analyzer"
        );
    }

    #[test]
    fn idle_coordinator_retains_lease_before_any_refresh() {
        let project = tempfile::tempdir().unwrap();
        let mut first = Coordinator::default();
        let mut second = Coordinator::default();
        assert!(first.acquire_lease(project.path()));
        assert!(!second.acquire_lease(project.path()));
        assert!(
            matches!(second.phase, Phase::Unavailable { ref error } if error.code == "coordinator_busy")
        );
        assert_eq!(first.activity().runs_started, 0);
        first.shutdown();
        second.retry_at = Instant::now();
        assert!(second.acquire_lease(project.path()));
    }
    #[test]
    fn published_metrics_survive_observing_their_own_generation() {
        let mut coordinator = Coordinator::default();
        coordinator.phase = Phase::Published {
            run: 1,
            input_revision: Some("edit".into()),
            generation: "new".into(),
            analysis_milliseconds: 123.0,
            metrics: serde_json::json!({"complete":true}),
        };
        coordinator.settled(true, "new");
        assert!(matches!(
            coordinator.phase,
            Phase::Published {
                analysis_milliseconds: 123.0,
                ..
            }
        ));
        // An external generation can have been observed before the old child
        // finishes. Its repeated observation must clear that old terminal state.
        coordinator.settled(false, "external");
        assert!(matches!(coordinator.phase, Phase::Watching));
    }
}
