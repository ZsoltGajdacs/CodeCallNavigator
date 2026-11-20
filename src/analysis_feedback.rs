//! Analysis diagnostics and explicit recovery for the current build selection.
use crate::{
    app::State,
    engine_client::{Analysis, AnalysisFailure},
};
use codecallnavigator::{
    discovery::{BuildIdentity, BuildTarget},
    protocol::{AnalysisFailureCode, AnalysisPhase, GraphRetention, TargetFailure},
    refresh::Phase,
};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Selection {
    project: PathBuf,
    targets: Vec<BuildIdentity>,
}

impl Selection {
    pub(crate) fn capture(project: &Path, targets: &[BuildTarget]) -> Self {
        let mut targets: Vec<_> = targets.iter().map(BuildTarget::identity).collect();
        targets.sort_unstable();
        Self {
            project: project.into(),
            targets,
        }
    }

    pub(crate) fn matches(&self, state: &State) -> bool {
        state
            .selected_analysis()
            .is_some_and(|selected| self == &Self::for_analysis(&selected))
    }

    pub(crate) fn for_analysis(analysis: &Analysis) -> Self {
        Self::capture(&analysis.project, &analysis.targets)
    }
}

pub(crate) struct FailedSelection {
    pub selection: Selection,
    pub error: AnalysisFailure,
}

#[derive(Default)]
pub(crate) struct Results {
    pub failure: Option<FailedSelection>,
    pub retention: Option<GraphRetention>,
}

pub(crate) struct Feedback<'a> {
    project: Option<&'a Path>,
    pub diagnostic: &'a str,
    pub target: Option<&'a TargetFailure>,
    pub preserved_graph: bool,
}

impl Feedback<'_> {
    pub fn summary(&self) -> String {
        let mut message = self.target.map_or_else(
            || "Analysis did not complete.".into(),
            |failure| {
                let phase = phase_label(failure.phase);
                let path = &failure.target.path;
                let relative = self
                    .project
                    .and_then(|root| path.strip_prefix(root).ok())
                    .filter(|relative| !relative.as_os_str().is_empty());
                let language = if failure.target.kind == "typescript" && phase == "Analysis" {
                    "TypeScript analysis"
                } else {
                    phase
                };
                format!(
                    "{language} failed for {}.",
                    relative.unwrap_or(path).display()
                )
            },
        );
        message.push_str(if self.preserved_graph {
            " Showing the last successful graph."
        } else {
            " No graph was created."
        });
        message
    }

    pub fn excerpt(&self) -> String {
        self.diagnostic
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(|line| bounded(line.trim(), 240))
            .unwrap_or_default()
    }
}

pub(crate) fn phase_label(phase: AnalysisPhase) -> &'static str {
    match phase {
        AnalysisPhase::Preparation => "Preparation",
        AnalysisPhase::Compilation => "Compilation",
        AnalysisPhase::Extraction => "Analysis",
        AnalysisPhase::Validation => "Validation",
    }
}

pub(super) fn bounded(message: &str, limit: usize) -> String {
    let mut chars = message.chars();
    let mut excerpt: String = chars.by_ref().take(limit).collect();
    if chars.next().is_some() {
        excerpt.push('…');
    }
    excerpt
}

pub(crate) fn retention_message(retention: &GraphRetention) -> String {
    match retention {
        GraphRetention::Verified { commit, .. } => {
            format!("Saved a verified snapshot for {commit}.")
        }
        GraphRetention::WorkingOnly { reason } => format!("Saved working graph only. {reason}"),
    }
}

pub(crate) struct Retry {
    pub target: BuildIdentity,
    pub label: &'static str,
}

impl State {
    pub(crate) fn feedback(&self) -> Option<Feedback<'_>> {
        let manual = self
            .analysis_feedback
            .failure
            .as_ref()
            .filter(|failure| failure.selection.matches(self));
        let (diagnostic, target) = if let Some(manual) = manual {
            let target = manual
                .error
                .target
                .as_ref()
                .filter(|_| manual.error.code == Some(AnalysisFailureCode::Failed));
            (manual.error.message.as_str(), target)
        } else {
            let Phase::Failed { error, target, .. } =
                &self.freshness.automatic.activity.as_ref()?.phase
            else {
                return None;
            };
            if !self.automatic_refresh_enabled() {
                return None;
            }
            let target = target.as_ref().filter(|_| error.code == "failed");
            (error.message.as_str(), target)
        };
        Some(Feedback {
            project: self.project_open.folder.as_deref().map(Path::new),
            diagnostic,
            target,
            preserved_graph: self.graph.is_some(),
        })
    }

    pub(crate) fn retry_analysis(&self) -> Option<Retry> {
        if self.analysis_job_busy() || self.jobs.discovery.is_pending() {
            return None;
        }
        if self.freshness.automatic.can_cancel() {
            return None;
        }
        self.runtime.as_ref()?;
        let target = &self.feedback()?.target?.target;
        let selection = self.selected_analysis()?;
        let matching_manual = self
            .analysis_feedback
            .failure
            .as_ref()
            .is_some_and(|failure| failure.selection == Selection::for_analysis(&selection));
        if !matching_manual && !self.freshness.matches_selected_session(&selection) {
            return None;
        }
        let mut remaining = selection
            .targets
            .iter()
            .filter(|build| build.identity() != *target);
        let first = remaining.next()?;
        let label = if remaining.next().is_none() && first.language == "java" {
            "Analyze Java only"
        } else {
            "Retry without this build"
        };
        (selection
            .targets
            .iter()
            .filter(|build| build.identity() == *target)
            .count()
            == 1)
            .then(|| Retry {
                target: target.clone(),
                label,
            })
    }

    pub(crate) fn retry_without_build(&mut self, expected: &BuildIdentity) {
        if !self
            .retry_analysis()
            .is_some_and(|retry| retry.target == *expected)
        {
            return;
        }
        if let Some(index) = self
            .targets
            .iter()
            .position(|choice| choice.selected && choice.target.identity() == *expected)
        {
            self.close_overlay();
            self.toggle_build(index);
            self.analyze_selected();
        }
    }
}
