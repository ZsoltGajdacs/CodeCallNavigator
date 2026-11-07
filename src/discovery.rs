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
    if matches!(
        name,
        "settings.gradle" | "settings.gradle.kts" | "build.gradle" | "build.gradle.kts"
    ) {
        gradle.insert(path.parent().unwrap_or(Path::new(".")).into());
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
        assert_eq!(targets.len(), 3);
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
