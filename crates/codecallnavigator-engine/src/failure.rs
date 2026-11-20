//! Shared analysis failure classification and bounded public diagnostics.
use crate::process_ownership::CleanupFailure;
use anyhow::Error;
use codecallnavigator_core::{
    artifact::{GenerationChanged, LockBusy},
    fingerprint::InputsChanged,
    protocol::{AnalysisFailureCode, Event, TargetFailure},
    refresh::Phase,
};

#[derive(Debug)]
struct Attribution {
    target: TargetFailure,
    diagnostic: String,
}

impl std::fmt::Display for Attribution {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:?} failed for {} ({})",
            self.target.phase,
            self.target.target.path.display(),
            self.target.target.kind
        )?;
        if let Some(logs) = &self.target.logs {
            write!(formatter, "\nBuild logs: {}", logs.display())?;
        }
        Ok(())
    }
}
impl std::error::Error for Attribution {}

fn classify(error: &Error, cancelled: bool) -> AnalysisFailureCode {
    if error.is::<LockBusy>() {
        AnalysisFailureCode::Busy
    } else if error.is::<GenerationChanged>() {
        AnalysisFailureCode::GenerationChanged
    } else if error.is::<InputsChanged>() {
        AnalysisFailureCode::InputsChanged
    } else if cancelled {
        AnalysisFailureCode::Cancelled
    } else {
        AnalysisFailureCode::Failed
    }
}

pub(crate) fn attribute(error: Error, target: TargetFailure, cancelled: bool) -> Error {
    if classify(&error, cancelled) != AnalysisFailureCode::Failed || error.is::<CleanupFailure>() {
        return error;
    }
    if error.is::<Attribution>() {
        return error;
    }
    // Keep the native diagnostic first on the wire. Target/phase/log ownership
    // is rendered independently and must not consume its diagnostic budget.
    let diagnostic = format!("{error:#}");
    error.context(Attribution { target, diagnostic })
}

pub struct Description {
    code: AnalysisFailureCode,
    message: String,
    target: Option<TargetFailure>,
}

pub fn describe(error: &Error, cancelled: bool) -> Description {
    let code = classify(error, cancelled);
    let target = if code == AnalysisFailureCode::Failed && !error.is::<CleanupFailure>() {
        error
            .downcast_ref::<Attribution>()
            .map(|value| value.target.clone())
    } else {
        None
    };
    let message = error
        .downcast_ref::<Attribution>()
        .map_or_else(|| format!("{error:#}"), |value| value.diagnostic.clone());
    Description {
        code,
        message: bounded_message(&message),
        target,
    }
}

impl Description {
    pub fn event(self, typed: bool) -> Event {
        Event::Failed {
            message: self.message,
            code: typed.then_some(self.code),
            target: typed.then_some(self.target).flatten(),
        }
    }

    pub fn phase(
        self,
        run: u64,
        input_revision: Option<String>,
        analysis_milliseconds: f64,
    ) -> Phase {
        Phase::Failed {
            run,
            input_revision,
            analysis_milliseconds,
            target: self.target,
            error: codecallnavigator_core::query::Error::new(self.code.as_str(), self.message),
        }
    }
}

fn bounded_message(message: &str) -> String {
    let mut chars = message.chars();
    let mut bounded = chars.by_ref().take(4096).collect::<String>();
    if chars.next().is_some() {
        bounded.push_str(" … [truncated]");
    }
    bounded
}

#[cfg(test)]
mod tests {
    use super::*;
    use codecallnavigator_core::{
        artifact::LockOwner, discovery::BuildTarget, protocol::AnalysisPhase,
    };

    fn build() -> BuildTarget {
        BuildTarget {
            path: "/project/pom.xml".into(),
            language: "java".into(),
            kind: "maven".into(),
            label: "service".into(),
        }
    }

    fn target(phase: AnalysisPhase) -> TargetFailure {
        TargetFailure {
            target: build().identity(),
            phase,
            logs: Some("/logs/attempt".into()),
        }
    }

    #[test]
    fn one_shot_and_activity_failures_share_operational_context() {
        let error = attribute(
            anyhow::anyhow!("native diagnostic {}", "界".repeat(5000)),
            target(AnalysisPhase::Compilation),
            false,
        );
        let event = describe(&error, false).event(true);
        let phase = describe(&error, false).phase(2, Some("revision".into()), 42.0);
        let Event::Failed {
            message,
            code,
            target,
        } = event
        else {
            panic!("failure event expected")
        };
        let Phase::Failed {
            error,
            target: activity_target,
            ..
        } = phase
        else {
            panic!("failure phase expected")
        };
        assert_eq!(code, Some(AnalysisFailureCode::Failed));
        assert_eq!(error.code, "failed");
        assert_eq!(message, error.message);
        assert!(message.starts_with("native diagnostic"));
        assert!(message.ends_with("[truncated]"));
        assert!(message.chars().count() < 4120);
        assert_eq!(target, activity_target);
        let target = target.unwrap();
        assert_eq!(target.target, build().identity());
        assert_eq!(target.phase, AnalysisPhase::Compilation);
        assert_eq!(target.logs, Some("/logs/attempt".into()));
        let encoded = serde_json::to_value(&target).unwrap();
        assert_eq!(encoded["phase"], "compilation");
        assert_eq!(
            serde_json::from_value::<TargetFailure>(encoded).unwrap(),
            target
        );
    }

    #[test]
    fn typed_nonoperational_failures_do_not_inherit_target_context() {
        let cases: Vec<(Error, bool, AnalysisFailureCode)> = vec![
            (
                LockBusy {
                    owner: LockOwner::Analysis,
                }
                .into(),
                false,
                AnalysisFailureCode::Busy,
            ),
            (
                GenerationChanged.into(),
                false,
                AnalysisFailureCode::GenerationChanged,
            ),
            (
                InputsChanged::new("changed", "/project/App.java", "content changed").into(),
                false,
                AnalysisFailureCode::InputsChanged,
            ),
            (
                anyhow::anyhow!("cancelled"),
                true,
                AnalysisFailureCode::Cancelled,
            ),
        ];
        for (error, cancelled, expected) in cases {
            let error = attribute(error, target(AnalysisPhase::Preparation), cancelled);
            let Event::Failed { code, target, .. } = describe(&error, cancelled).event(true) else {
                panic!("failure expected")
            };
            assert_eq!(code, Some(expected));
            assert!(target.is_none());
        }
        let error = anyhow::anyhow!("global failure");
        let encoded = serde_json::to_value(describe(&error, false).event(false)).unwrap();
        assert!(encoded.get("target").is_none());
        assert!(encoded.get("code").is_none());
    }
}
