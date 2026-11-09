use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildTarget {
    pub path: PathBuf,
    pub language: String,
    pub kind: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuildIdentity {
    pub path: PathBuf,
    pub kind: String,
}

impl BuildTarget {
    pub fn identity(&self) -> BuildIdentity {
        BuildIdentity {
            path: self.path.clone(),
            kind: self.kind.clone(),
        }
    }
}

pub(crate) fn excluded_directory(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".gradle"
            | ".idea"
            | ".serena"
            | ".backlog"
            | "vendor"
            | "node_modules"
            | "build"
            | "obj"
            | "bin"
            | "target"
            | ".godot"
    )
}

pub fn discover(root: &Path) -> Result<Vec<BuildTarget>> {
    let root = root
        .canonicalize()
        .context("Cannot open the selected project folder")?;
    anyhow::ensure!(root.is_dir(), "Select a project folder");
    let mut targets = Vec::new();
    let mut gradle_roots = BTreeSet::new();
    discover_directory(&root, &mut targets, &mut gradle_roots)?;
    for path in independent_gradle_roots(&gradle_roots) {
        targets.push(BuildTarget {
            label: display_name(&path),
            path,
            language: "java".into(),
            kind: "gradle".into(),
        });
    }
    targets.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(targets)
}

/// Shared UI/CLI defaults: solutions own C# projects; outer Maven POMs own nested selections.
pub fn default_selection(targets: &[BuildTarget]) -> BTreeSet<BuildIdentity> {
    let has_solution = targets.iter().any(|target| target.kind == "solution");
    targets
        .iter()
        .filter(|target| {
            if target.kind == "typescript" {
                return default_typescript_target(target, targets);
            }
            if target.kind == "project" {
                return !has_solution;
            }
            target.kind != "maven"
                || !targets.iter().any(|other| {
                    (other.kind == "maven"
                        && other.path != target.path
                        && target.path.starts_with(&other.path))
                        || (other.kind == "gradle" && other.path == target.path)
                })
        })
        .map(BuildTarget::identity)
        .collect()
}

fn default_typescript_target(target: &BuildTarget, targets: &[BuildTarget]) -> bool {
    target
        .path
        .file_name()
        .is_some_and(|name| name == "tsconfig.json")
        || !targets.iter().any(|other| {
            other.kind == "typescript"
                && other.path.parent() == target.path.parent()
                && other
                    .path
                    .file_name()
                    .is_some_and(|name| name == "tsconfig.json")
        })
}

pub fn target_from_file(path: &Path) -> Result<BuildTarget> {
    let path = path
        .canonicalize()
        .context("Cannot open selected build file")?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    if is_typescript_config(&name) {
        return Ok(typescript_target(path));
    }
    if name == "pom.xml" {
        let root = path
            .parent()
            .context("POM has no parent directory")?
            .to_path_buf();
        return Ok(BuildTarget {
            label: format!("{} (Maven)", display_name(&root)),
            path: root,
            language: "java".into(),
            kind: "maven".into(),
        });
    }
    let kind = match path.extension().and_then(|extension| extension.to_str()) {
        Some("csproj") => "project",
        Some("sln" | "slnx") => "solution",
        _ => anyhow::bail!("Unsupported build file: {}", path.display()),
    };
    Ok(BuildTarget {
        label: name.into(),
        path,
        language: "csharp".into(),
        kind: kind.into(),
    })
}

fn has_gradle_settings(path: &Path) -> bool {
    path.join("settings.gradle").is_file() || path.join("settings.gradle.kts").is_file()
}

fn independent_gradle_roots(candidates: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    candidates
        .iter()
        .filter(|path| {
            has_gradle_settings(path)
                || !candidates.iter().any(|ancestor| {
                    ancestor != *path && path.starts_with(ancestor) && has_gradle_settings(ancestor)
                })
        })
        .cloned()
        .collect()
}

fn discover_directory(
    root: &Path,
    targets: &mut Vec<BuildTarget>,
    gradle: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(root).with_context(|| format!("Cannot read {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type()?;
        if kind.is_dir() && !excluded_directory(&name) {
            discover_directory(&path, targets, gradle)?;
        } else if kind.is_file() {
            discover_file(&path, &name, targets, gradle);
        }
    }
    Ok(())
}

fn discover_file(
    path: &Path,
    name: &str,
    targets: &mut Vec<BuildTarget>,
    gradle: &mut BTreeSet<PathBuf>,
) {
    if is_typescript_config(name) && !typescript_output_path(path) {
        targets.push(typescript_target(path.into()));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let kind = match extension {
        "sln" | "slnx" => "solution",
        "csproj" => "project",
        _ => "",
    };
    if !kind.is_empty() {
        targets.push(BuildTarget {
            path: path.into(),
            language: "csharp".into(),
            kind: kind.into(),
            label: name.into(),
        });
    }
    if name == "pom.xml" {
        let root = path.parent().unwrap_or(Path::new("."));
        targets.push(BuildTarget {
            path: root.into(),
            language: "java".into(),
            kind: "maven".into(),
            label: format!("{} (Maven)", root.display()),
        });
    }
    if matches!(
        name,
        "settings.gradle" | "settings.gradle.kts" | "build.gradle" | "build.gradle.kts"
    ) {
        gradle.insert(path.parent().unwrap_or(Path::new(".")).into());
    }
}

fn typescript_output_path(path: &Path) -> bool {
    path.ancestors().skip(1).any(|parent| {
        parent
            .file_name()
            .is_some_and(|name| matches!(name.to_str(), Some(".next" | "dist" | "coverage")))
    })
}

fn is_typescript_config(name: &str) -> bool {
    name == "tsconfig.json" || (name.starts_with("tsconfig.") && name.ends_with(".json"))
}

fn typescript_target(path: PathBuf) -> BuildTarget {
    BuildTarget {
        label: format!(
            "{} ({})",
            display_name(path.parent().unwrap_or(Path::new("."))),
            path.file_name().unwrap_or_default().to_string_lossy()
        ),
        path,
        language: "typescript".into(),
        kind: "typescript".into(),
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_solution_projects_and_deduplicates_gradle_files() {
        let directory = tempfile::tempdir().unwrap();
        for name in [
            "App.sln",
            "App.csproj",
            "settings.gradle.kts",
            "build.gradle.kts",
            "pom.xml",
        ] {
            fs::write(directory.path().join(name), "").unwrap();
        }
        fs::create_dir(directory.path().join("vendor")).unwrap();
        fs::write(directory.path().join("vendor/Hidden.csproj"), "").unwrap();
        let targets = discover(directory.path()).unwrap();
        assert_eq!(targets.len(), 4);
        assert_eq!(
            targets
                .iter()
                .filter(|target| target.kind == "gradle")
                .count(),
            1
        );
        assert!(
            targets
                .iter()
                .all(|target| !target.path.ends_with("Hidden.csproj"))
        );
    }

    #[test]
    fn maven_discovery_and_selection_keep_nested_projects_available() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("module")).unwrap();
        for name in ["pom.xml", "module/pom.xml", "App.csproj", "App.sln"] {
            fs::write(directory.path().join(name), "").unwrap();
        }
        let targets = discover(directory.path()).unwrap();
        let selected = default_selection(&targets);
        let maven: Vec<_> = targets
            .iter()
            .filter(|target| target.kind == "maven")
            .collect();
        assert_eq!(maven.len(), 2);
        assert!(selected.contains(&BuildIdentity {
            path: directory.path().canonicalize().unwrap(),
            kind: "maven".into()
        }));
        assert!(!selected.contains(&BuildIdentity {
            path: directory.path().join("module"),
            kind: "maven".into()
        }));
        assert!(!selected.contains(&BuildIdentity {
            path: directory.path().join("App.csproj"),
            kind: "project".into()
        }));
        assert_eq!(
            target_from_file(&directory.path().join("pom.xml"))
                .unwrap()
                .kind,
            "maven"
        );
        assert!(target_from_file(&directory.path().join("unknown.xml")).is_err());
    }

    #[test]
    fn mixed_tool_defaults_distinguish_same_directory_targets() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("nested")).unwrap();
        for file in ["pom.xml", "nested/pom.xml", "nested/build.gradle"] {
            fs::write(root.path().join(file), "").unwrap();
        }
        let targets = discover(root.path()).unwrap();
        let selected = default_selection(&targets);
        assert_eq!(selected.len(), 2);
        assert!(selected.iter().any(|identity| identity.kind == "gradle"));
        assert!(!selected.contains(&BuildIdentity {
            path: root.path().join("nested"),
            kind: "maven".into()
        }));
    }

    #[test]
    fn groups_subprojects_under_settings_but_keeps_independent_nested_builds() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("settings.gradle.kts"), "").unwrap();
        fs::create_dir_all(directory.path().join("module/independent")).unwrap();
        fs::write(directory.path().join("module/build.gradle.kts"), "").unwrap();
        fs::write(
            directory.path().join("module/independent/settings.gradle"),
            "",
        )
        .unwrap();
        let targets = discover(directory.path()).unwrap();
        assert_eq!(targets.len(), 2);
        assert!(
            targets
                .iter()
                .all(|target| !target.path.ends_with("module"))
        );
    }
}
