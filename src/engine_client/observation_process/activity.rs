//! Validation of optional coordinator activity at the desktop wire boundary.
use super::digest;
use anyhow::{Result, ensure};
use codecallnavigator::{
    query::Error,
    refresh::{Activity, Phase},
};

pub(super) fn validate(activity: &Activity) -> Result<()> {
    ensure!(
        activity.version == codecallnavigator::refresh::VERSION,
        "Unsupported refresh activity version"
    );
    ensure!(
        run_identity(&activity.phase).is_none_or(|run| run <= activity.runs_started),
        "Refresh run exceeds actual start counter"
    );
    match &activity.phase {
        Phase::Watching => Ok(()),
        Phase::Unavailable { error } => failure(error),
        Phase::Debouncing { input_revision, .. } => observed_revision(input_revision),
        Phase::Running {
            run,
            input_revision,
            debounce_milliseconds,
            wait_milliseconds,
        } => {
            running(Some(*run), input_revision, *wait_milliseconds)?;
            duration(*debounce_milliseconds)
        }
        Phase::Busy {
            run,
            input_revision,
            wait_milliseconds,
            ..
        } => running(*run, input_revision, *wait_milliseconds),
        Phase::Published {
            run,
            input_revision,
            generation,
            analysis_milliseconds,
            ..
        } => {
            running(Some(*run), input_revision, *analysis_milliseconds)?;
            revision(generation)
        }
        Phase::Failed {
            run,
            input_revision,
            error,
            analysis_milliseconds,
        } => {
            running(Some(*run), input_revision, *analysis_milliseconds)?;
            failure(error)
        }
        Phase::Cancelled {
            run,
            input_revision,
            analysis_milliseconds,
            ..
        } => running(*run, input_revision, *analysis_milliseconds),
    }
}

fn running(run: Option<u64>, input_revision: &Option<String>, milliseconds: f64) -> Result<()> {
    ensure!(
        run.is_none_or(|run| run > 0),
        "Invalid refresh run identity"
    );
    observed_revision(input_revision)?;
    duration(milliseconds)
}

fn observed_revision(value: &Option<String>) -> Result<()> {
    ensure!(
        value.as_deref().is_none_or(digest),
        "Invalid refresh input revision"
    );
    Ok(())
}

fn revision(value: &str) -> Result<()> {
    ensure!(
        digest(value),
        "Invalid refresh generation or input revision"
    );
    Ok(())
}

fn duration(value: f64) -> Result<()> {
    ensure!(
        value.is_finite() && value >= 0.0,
        "Invalid refresh duration"
    );
    Ok(())
}

fn failure(error: &Error) -> Result<()> {
    ensure!(
        !error.code.is_empty() && !error.message.is_empty(),
        "Malformed refresh failure"
    );
    Ok(())
}

fn run_identity(phase: &Phase) -> Option<u64> {
    match phase {
        Phase::Running { run, .. } | Phase::Published { run, .. } | Phase::Failed { run, .. } => {
            Some(*run)
        }
        Phase::Busy { run, .. } | Phase::Cancelled { run, .. } => *run,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_full_input_revision_remains_null_in_activity() {
        let mut value = serde_json::json!({
            "version":1,"runs_started":1,"state":"running","run":1,
            "input_revision":null,"debounce_milliseconds":300.0,"wait_milliseconds":0.0
        });
        let activity: Activity = serde_json::from_value(value.clone()).unwrap();
        validate(&activity).unwrap();
        assert!(serde_json::to_value(activity).unwrap()["input_revision"].is_null());
        value["input_revision"] = "".into();
        assert!(validate(&serde_json::from_value::<Activity>(value).unwrap()).is_err());
    }
}
