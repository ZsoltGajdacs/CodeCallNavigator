//! Pure activity copy and emphasis; lifecycle ownership remains with jobs/freshness.
use crate::freshness_workspace::Status;
use codecallnavigator::artifact::verification::InputStatus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Work {
    Idle,
    Discovery,
    Analysis,
    Layout,
    Reload,
    Automatic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tone {
    Neutral,
    Current,
    Attention,
}

pub(super) struct Input<'a> {
    pub work: Work,
    pub has_graph: bool,
    pub warning: &'a str,
    pub status: &'a str,
    pub freshness: &'a Status,
    pub freshness_message: Option<&'a str>,
    pub newer_graph: bool,
}

pub(super) struct Presentation {
    pub title: &'static str,
    pub detail: String,
    pub busy: bool,
    pub tone: Tone,
}

pub(super) fn present(input: Input<'_>) -> Presentation {
    let (title, tone) = heading(&input);
    let detail = if !input.warning.is_empty() {
        input.warning
    } else if input.work != Work::Idle {
        input.status
    } else {
        input.freshness_message.unwrap_or(input.status)
    };
    Presentation {
        title,
        detail: detail.into(),
        busy: input.work != Work::Idle || matches!(input.freshness, Status::Checking),
        tone,
    }
}

fn heading(input: &Input<'_>) -> (&'static str, Tone) {
    if !input.warning.is_empty() {
        return ("Graph needs attention", Tone::Attention);
    }
    match input.work {
        Work::Discovery => return ("Finding builds", Tone::Neutral),
        Work::Analysis | Work::Automatic | Work::Layout => {
            return ("Updating graph", Tone::Neutral);
        }
        Work::Reload => return ("Loading saved graph", Tone::Neutral),
        Work::Idle => {}
    }
    if !input.has_graph {
        return ("Ready to explore", Tone::Neutral);
    }
    if input.newer_graph {
        return ("New graph available", Tone::Attention);
    }
    match input.freshness {
        Status::Observed(InputStatus::Match) => ("Graph up to date", Tone::Current),
        Status::Observed(InputStatus::Stale) => ("Graph out of date", Tone::Attention),
        Status::Observed(InputStatus::Unverifiable) => ("Freshness unknown", Tone::Attention),
        Status::Checking => ("Checking graph", Tone::Neutral),
        Status::Failed(_) | Status::Unsupported => ("Freshness unavailable", Tone::Attention),
        Status::Idle => ("Graph ready", Tone::Neutral),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(freshness: &Status) -> Input<'_> {
        Input {
            work: Work::Idle,
            has_graph: true,
            warning: "",
            status: "Last successful graph is preserved",
            freshness,
            freshness_message: None,
            newer_graph: false,
        }
    }

    #[test]
    fn idle_requires_verified_matching_inputs_before_claiming_current() {
        assert_eq!(present(input(&Status::Idle)).title, "Graph ready");
        let checking = present(input(&Status::Checking));
        assert_eq!(checking.tone, Tone::Neutral);
        assert!(checking.busy);
        let current = present(input(&Status::Observed(InputStatus::Match)));
        assert_eq!(current.title, "Graph up to date");
        assert_eq!(current.tone, Tone::Current);
        assert!(!current.busy);
        for status in [
            Status::Observed(InputStatus::Stale),
            Status::Observed(InputStatus::Unverifiable),
            Status::Failed("offline".into()),
            Status::Unsupported,
        ] {
            assert_eq!(present(input(&status)).tone, Tone::Attention);
        }
    }

    #[test]
    fn pending_work_retains_actual_phase_and_never_reports_success() {
        let freshness = Status::Observed(InputStatus::Match);
        for work in [Work::Analysis, Work::Layout, Work::Reload, Work::Automatic] {
            let mut data = input(&freshness);
            data.work = work;
            data.status = "Compiling Java sources…";
            data.freshness_message = Some("Inputs match");
            let result = present(data);
            assert!(result.busy);
            assert_eq!(result.detail, "Compiling Java sources…");
            assert_eq!(result.tone, Tone::Neutral);
        }
    }

    #[test]
    fn warnings_and_new_generations_override_success() {
        let freshness = Status::Observed(InputStatus::Match);
        let mut newer = input(&freshness);
        newer.newer_graph = true;
        assert_eq!(present(newer).title, "New graph available");
        let mut failed = input(&freshness);
        failed.warning = "Analysis failed; the previous graph is preserved";
        let result = present(failed);
        assert_eq!(result.title, "Graph needs attention");
        assert_eq!(
            result.detail,
            "Analysis failed; the previous graph is preserved"
        );
        assert_eq!(result.tone, Tone::Attention);
    }

    #[test]
    fn discovery_and_initial_empty_states_do_not_claim_a_graph_exists() {
        let mut data = input(&Status::Idle);
        data.has_graph = false;
        assert_eq!(present(data).title, "Ready to explore");
        let mut discovery = input(&Status::Idle);
        discovery.work = Work::Discovery;
        discovery.status = "Finding solutions, Java builds and TypeScript projects…";
        let result = present(discovery);
        assert_eq!(result.title, "Finding builds");
        assert!(result.busy);
        assert_eq!(
            result.detail,
            "Finding solutions, Java builds and TypeScript projects…"
        );
    }
}
