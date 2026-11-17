//! Presentation of CLI-owned observations; this module never inspects files.
use super::Workspace;
use crate::engine_client::Descriptor;
use std::path::Path;

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
        let mut entries = observation
            .into_iter()
            .flat_map(|last| &last.reasons)
            .map(|reason| reason.describe())
            .collect::<Vec<_>>();
        if let Some(error) = workspace.automatic.details()
            && !entries.iter().any(|entry| entry == error)
        {
            if workspace.automatic.activity.as_ref().is_some_and(|activity| {
                matches!(&activity.phase, codecallnavigator::refresh::Phase::Unavailable { error } if error.code == "incomplete_observation")
            }) {
                // Trigger observations may contain different blockers. Only
                // remove a public description when its complete text survives
                // verbatim in the coordinator report; retain truncated matches.
                entries.retain(|entry| !error.contains(entry.as_str()));
            }
            entries.push(error.into());
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
    use codecallnavigator::artifact::verification::{InputStatus, Reason};

    #[test]
    fn details_preserve_causes_omissions_and_shell_quoted_project() {
        let mut workspace = Workspace::default();
        let mut observation = crate::freshness_workspace::fixture(InputStatus::Stale, "generation");
        observation.coverage.complete = false;
        observation.reasons.push(Reason::new(
            "unreadable_input",
            Some("@repo/required".into()),
        ));
        observation.omitted_reasons = 3;
        observation.reasons_total = 4;
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
        let repeated = workspace.last.as_ref().unwrap().reasons[0].describe();
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
    }
}
