//! Evaluation contexts matching CallAtlas's pinned Roslyn 5.9.0 .NET BuildHost.
//!
//! Revisit with the CallAtlas Roslyn dependency. Source: ProjectBuildManager.cs,
//! commit 35d9211b841e7613c1d2f8f5af6d628ace696c4c, defaults and inner builds.
use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Command;

const DESIGN_PROPERTIES: [(&str, &str); 8] = [
    ("DesignTimeBuild", "True"),
    (
        "NonExistentFile",
        "__NonExistentSubDir__\\__NonExistentFile__",
    ),
    ("BuildProjectReferences", "False"),
    ("BuildingProject", "False"),
    ("ProvideCommandLineArgs", "True"),
    ("SkipCompilerExecution", "True"),
    ("ContinueOnError", "ErrorAndContinue"),
    ("ShouldUnsetParentConfigurationAndPlatform", "False"),
];

/// CLI property values are literals, not additional MSBuild assignments.
/// Escape percent first so existing escape-looking filename text stays literal.
pub(super) fn literal_property(command: &mut Command, name: &str, value: &str) {
    let encoded = value
        .replace('%', "%25")
        .replace(';', "%3B")
        .replace(',', "%2C");
    command.arg(format!("-property:{name}={encoded}"));
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::analyzers::csharp) struct EvaluationContext {
    design_time: bool,
    target_framework: Option<String>,
}

impl EvaluationContext {
    pub fn roots() -> [Self; 2] {
        [
            Self {
                design_time: false,
                target_framework: None,
            },
            Self {
                design_time: true,
                target_framework: None,
            },
        ]
    }

    pub fn is_configured_normal(&self) -> bool {
        !self.design_time && self.target_framework.is_none()
    }

    pub fn apply(&self, command: &mut Command) {
        if self.design_time {
            for (name, value) in DESIGN_PROPERTIES {
                literal_property(command, name, value);
            }
        }
        if let Some(framework) = &self.target_framework {
            literal_property(command, "TargetFramework", framework);
        }
    }

    pub fn inner_builds(&self, metadata: &Value) -> Result<Vec<Self>> {
        let selected = metadata
            .pointer("/Properties/TargetFramework")
            .and_then(Value::as_str)
            .context("MSBuild omitted TargetFramework")?;
        let multiple = metadata
            .pointer("/Properties/TargetFrameworks")
            .and_then(Value::as_str)
            .context("MSBuild omitted TargetFrameworks")?;
        if !selected.is_empty() || multiple.is_empty() {
            return Ok(Vec::new());
        }
        Ok(multiple
            .split(';')
            .map(|framework| Self {
                design_time: self.design_time,
                target_framework: Some(framework.into()),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dynamic_property_values_preserve_percent_before_escaping_separators() {
        let mut command = Command::new("dotnet");
        let context = EvaluationContext {
            design_time: true,
            target_framework: Some("net11.0%3B;DesignTimeBuild=False,part%".into()),
        };
        context.apply(&mut command);
        assert!(
            command.get_args().any(|arg| arg
                == "-property:TargetFramework=net11.0%253B%3BDesignTimeBuild=False%2Cpart%25")
        );
    }

    #[test]
    fn pinned_net_design_globals_do_not_invent_visual_studio_properties() {
        let mut command = Command::new("dotnet");
        EvaluationContext::roots()[1].apply(&mut command);
        let arguments = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(
            arguments
                .iter()
                .any(|arg| arg == "-property:DesignTimeBuild=True")
        );
        assert!(arguments.iter().any(
            |arg| arg == "-property:NonExistentFile=__NonExistentSubDir__\\__NonExistentFile__"
        ));
        assert!(
            !arguments
                .iter()
                .any(|arg| arg.contains("BuildingInsideVisualStudio"))
        );
    }

    #[test]
    fn explicit_framework_wins_otherwise_all_inner_builds_are_selected() {
        let root = EvaluationContext::roots()[1].clone();
        assert!(root.inner_builds(&json!({"Properties":{"TargetFramework":"net11.0","TargetFrameworks":"net10.0;net11.0"}})).unwrap().is_empty());
        let inner = root
            .inner_builds(
                &json!({"Properties":{"TargetFramework":"","TargetFrameworks":"net10.0;net11.0"}}),
            )
            .unwrap();
        assert_eq!(inner.len(), 2);
        assert_eq!(inner[0].target_framework.as_deref(), Some("net10.0"));
        assert!(inner.iter().all(|context| context.design_time));
    }
}
