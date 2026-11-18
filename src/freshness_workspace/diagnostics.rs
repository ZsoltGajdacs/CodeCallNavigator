//! Presentation of CLI-owned observations; this module never inspects files.
use super::{Status, Workspace};
use crate::engine_client::Descriptor;
use codecallnavigator::artifact::verification::{InputStatus, Reason};
use std::path::Path;

impl Workspace {
    fn visible_reasons(&self) -> impl Iterator<Item = &Reason> {
        self.last
            .iter()
            .flat_map(|observation| &observation.reasons)
            .filter(|reason| reason.code != "unmapped_input")
    }

    pub(crate) fn display_status(&self) -> &Status {
        let unmapped_only = matches!(self.status, Status::Observed(InputStatus::Unverifiable))
            && self.last.as_ref().is_some_and(|last| {
                last.contract_valid
                    && last.coverage.complete
                    && last.omitted_reasons == 0
                    && !last.reasons.is_empty()
                    && self.visible_reasons().next().is_none()
            });
        if unmapped_only {
            &Status::Idle
        } else {
            &self.status
        }
    }

    pub(crate) fn message(&self, loaded: Option<&str>) -> Option<String> {
        if matches!(self.status, Status::Observed(_)) && self.new_generation(loaded).is_some() {
            return Some("A newer saved graph is available".into());
        }
        match self.display_status() {
            Status::Idle => None,
            Status::Checking => Some("Checking saved-input freshness…".into()),
            Status::Unsupported => {
                Some("Continuous freshness unavailable · update the CLI or refresh manually".into())
            }
            Status::Failed(error) => Some(format!("Freshness check failed · {error}")),
            Status::Observed(InputStatus::Match) => {
                Some("Recorded inputs match · observed scope only".into())
            }
            Status::Observed(InputStatus::Stale) => {
                Some("Source files changed · refresh to update this graph".into())
            }
            Status::Observed(InputStatus::Unverifiable) => Some(format!(
                "Saved inputs are unverifiable · {}",
                self.reason_context()
            )),
        }
    }

    pub(crate) fn attention(&self, loaded: Option<&str>) -> Option<String> {
        if matches!(self.status, Status::Observed(InputStatus::Match))
            && self.new_generation(loaded).is_none()
        {
            return None;
        }
        self.message(loaded)
    }

    fn reason_context(&self) -> String {
        let Some(reason) = self.visible_reasons().next() else {
            if self
                .last
                .as_ref()
                .is_some_and(|last| last.omitted_reasons > 0)
            {
                return "open details for the full verification command".into();
            }
            return "no complete input manifest".into();
        };
        let context = match &reason.path {
            Some(path) => format!("{}: {path}", reason.code),
            None => reason.code.clone(),
        };
        format!("{context} · open details for the cause and recovery")
    }
}

pub(crate) struct Details {
    pub summary: String,
    pub entries: Vec<String>,
    pub omitted: usize,
    pub command: String,
}

impl Details {
    pub fn from_workspace(
        workspace: &Workspace,
        descriptor: &Descriptor,
        executable: Option<&Path>,
    ) -> Option<Self> {
        let observation = workspace.last.as_ref();
        let mut entries = workspace
            .visible_reasons()
            .map(Reason::describe)
            .collect::<Vec<_>>();
        if let Some(error) = workspace.automatic.details()
            && !entries.iter().any(|entry| entry == &error)
        {
            if workspace.automatic.activity.as_ref().is_some_and(|activity| {
                matches!(&activity.phase, codecallnavigator::refresh::Phase::Unavailable { error } if error.code == "incomplete_observation")
            }) {
                // Trigger observations may contain different blockers. Only
                // remove a public description when its complete text survives
                // verbatim in the coordinator report; retain truncated matches.
                entries.retain(|entry| !error.contains(entry.as_str()));
            }
            entries.push(error);
        }
        let omitted = observation.map_or(0, |last| last.omitted_reasons);
        if entries.is_empty() && omitted == 0 {
            return None;
        }
        let summary = if observation.is_some_and(|last| !last.coverage.complete) {
            "Required inputs could not be checked completely. Automatic refresh is blocked; the previous graph remains available."
        } else {
            "The engine reported input conditions that need attention. The saved graph's freshness is shown above."
        };
        Some(Self {
            summary: summary.into(),
            entries,
            omitted,
            command: verify_command(descriptor, executable),
        })
    }

    pub fn copy_text(&self) -> String {
        let mut lines = vec![self.summary.clone()];
        lines.extend(self.entries.iter().cloned());
        if self.omitted > 0 {
            lines.push(format!(
                "{} additional diagnostics omitted from this bounded response.",
                self.omitted
            ));
        }
        lines.push(format!(
            "Full read-only verification report:\n{}",
            self.command
        ));
        lines.push("Retry freshness checks the inputs again; it does not rebuild the graph. Refresh reruns analysis.".into());
        lines.join("\n\n")
    }
}

fn verify_command(descriptor: &Descriptor, executable: Option<&Path>) -> String {
    let executable = executable.map_or_else(
        || "ccnavigator".into(),
        |path| shell_quote(&path.to_string_lossy()),
    );
    let mut command = format!(
        "{executable} verify --project {}",
        shell_quote(&descriptor.project.to_string_lossy())
    );
    for (name, path) in &descriptor.roots {
        command.push_str(" --root ");
        command.push_str(&shell_quote(&format!("{name}={}", path.display())));
    }
    command
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_preserve_causes_omissions_and_shell_quoted_project() {
        let mut workspace = Workspace::default();
        let mut observation = crate::freshness_workspace::fixture(InputStatus::Stale, "generation");
        observation.coverage.complete = false;
        observation
            .reasons
            .push(Reason::new("unmapped_input", None));
        observation.reasons.push(Reason::new(
            "unreadable_input",
            Some("@repo/required".into()),
        ));
        observation.omitted_reasons = 3;
        observation.reasons_total = 5;
        workspace.accept(observation);
        let descriptor = Descriptor {
            project: "/project/it's $(literal)".into(),
            roots: vec![("shared".into(), "/dependency/it's $(literal)".into())],
            ..Default::default()
        };
        let details = Details::from_workspace(&workspace, &descriptor, None).unwrap();
        let copied = details.copy_text();
        assert!(copied.contains("Automatic refresh is blocked"));
        assert!(copied.contains("@repo/required"));
        assert!(workspace.is_stale());
        assert!(!copied.contains("unmapped_input"));
        assert!(copied.contains("3 additional diagnostics"));
        assert!(copied.contains("'/project/it'\\''s $(literal)'"));
        assert!(copied.contains("--root 'shared=/dependency/it'\\''s $(literal)'"));
        assert!(copied.contains("does not rebuild"));
        let matching = Details::from_workspace(
            &workspace,
            &descriptor,
            Some(Path::new("/app/it's $(literal)/engine")),
        )
        .unwrap();
        assert!(
            matching
                .command
                .starts_with("'/app/it'\\''s $(literal)/engine' verify --project")
        );
        assert!(!matching.command.starts_with("ccnavigator"));
        let repeated = workspace.last.as_ref().unwrap().reasons[1].describe();
        workspace.automatic.activity = Some(codecallnavigator::refresh::Activity {
            version: 1,
            runs_started: 0,
            phase: codecallnavigator::refresh::Phase::Unavailable {
                error: codecallnavigator::query::Error::new(
                    "incomplete_observation",
                    format!("{repeated}\nTrigger-only ownership blocker"),
                ),
            },
        });
        let deduplicated = Details::from_workspace(&workspace, &descriptor, None)
            .unwrap()
            .copy_text();
        assert_eq!(deduplicated.matches(&repeated).count(), 1);
        assert!(deduplicated.contains("Trigger-only ownership blocker"));

        assert_unmapped_input_presentation(&descriptor);
    }

    fn assert_unmapped_input_presentation(descriptor: &Descriptor) {
        let mut workspace = Workspace::default();
        let mut observation =
            crate::freshness_workspace::fixture(InputStatus::Unverifiable, "generation");
        observation
            .reasons
            .push(Reason::new("unmapped_input", None));
        observation.reasons_total = 1;
        workspace.accept(observation.clone());
        assert!(workspace.message(Some("generation")).is_none());
        assert!(workspace.attention(Some("generation")).is_none());
        assert_eq!(
            workspace.attention(Some("older-generation")).as_deref(),
            Some("A newer saved graph is available")
        );
        assert!(matches!(workspace.display_status(), Status::Idle));
        assert!(matches!(
            workspace.status,
            Status::Observed(InputStatus::Unverifiable)
        ));
        assert!(Details::from_workspace(&workspace, descriptor, None).is_none());
        assert_eq!(
            workspace.last.as_ref().unwrap().reasons,
            observation.reasons
        );

        observation.omitted_reasons = 1;
        observation.reasons_total = 2;
        workspace.accept(observation);
        let message = workspace.message(Some("generation")).unwrap();
        assert!(!message.contains("unmapped_input"));
        let details = Details::from_workspace(&workspace, descriptor, None).unwrap();
        assert!(details.entries.is_empty());
        assert_eq!(details.omitted, 1);

        let mut mixed = workspace.last.take().unwrap();
        mixed.omitted_reasons = 0;
        mixed
            .reasons
            .push(Reason::new("unavailable_root", Some("@shared/".into())));
        workspace.accept(mixed);
        let message = workspace.message(Some("generation")).unwrap();
        assert!(message.contains("unavailable_root: @shared/"));
        assert!(!message.contains("unmapped_input"));
    }
}
