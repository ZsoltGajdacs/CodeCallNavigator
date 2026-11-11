use super::Roots;
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

pub(super) struct Paths {
    roots: Vec<(String, PathBuf)>,
}

pub(super) struct ProjectOwner<'a> {
    pub build: &'a str,
    pub directory: bool,
}

impl ProjectOwner<'_> {
    fn directory_of<'p>(&self, path: &'p Path) -> Result<&'p Path> {
        if self.directory {
            Ok(path)
        } else {
            path.parent().context("Build has no project directory")
        }
    }
}

impl Paths {
    pub fn new(roots: &Roots) -> Result<Self> {
        ensure!(!roots.is_empty(), "Declare at least one logical root");
        let mut names = HashSet::new();
        let mut normalized = Vec::new();
        for (name, path) in roots {
            ensure!(
                !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-')),
                "Invalid logical root name: {name}"
            );
            ensure!(names.insert(name), "Duplicate logical root: {name}");
            let path = std::path::absolute(path)?;
            normalized.push((name.clone(), path));
        }
        normalized.sort_by_key(|(_, path)| std::cmp::Reverse(path.components().count()));
        Ok(Self { roots: normalized })
    }

    pub fn portable(&self, text: &str) -> Result<String> {
        if text.is_empty() {
            return Ok(String::new());
        }
        let path = Path::new(text);
        ensure!(path.is_absolute(), "Expected absolute graph path: {text}");
        for (name, root) in &self.roots {
            if let Ok(relative) = path.strip_prefix(root) {
                ensure!(
                    !relative.components().any(|c| c == Component::ParentDir),
                    "Path escapes logical root: {text}"
                );
                return Ok(format!(
                    "@{name}/{}",
                    relative.to_string_lossy().replace('\\', "/")
                ));
            }
        }
        anyhow::bail!("Graph path is outside declared roots: {text}")
    }

    pub(super) fn local(&self, text: &str) -> Result<String> {
        if text.is_empty() {
            return Ok(String::new());
        }
        let (name, relative) = text
            .strip_prefix('@')
            .and_then(|s| s.split_once('/'))
            .context("Invalid portable graph path")?;
        ensure!(
            !Path::new(relative).is_absolute()
                && !Path::new(relative)
                    .components()
                    .any(|c| c == Component::ParentDir),
            "Portable graph path escapes its root"
        );
        let (_, root) = self
            .roots
            .iter()
            .find(|(n, _)| n == name)
            .with_context(|| format!("Missing local mapping for logical root {name}"))?;
        Ok(root.join(relative).to_string_lossy().into_owned())
    }

    pub fn rewrite(&self, value: &mut Value, hydrate: bool) -> Result<()> {
        let text = value.as_str().context("Graph path must be a string")?;
        *value = if hydrate {
            self.local(text)?
        } else {
            self.portable(text)?
        }
        .into();
        Ok(())
    }

    /// Used only for analyzer-owned identities/diagnostics, never source text.
    pub fn embedded(&self, text: &str) -> String {
        let mut result = text.to_owned();
        for mapping in &self.roots {
            result = replace_root(&result, mapping);
        }
        result
    }

    pub fn project(&self, value: &mut Value, owner: ProjectOwner<'_>, hydrate: bool) -> Result<()> {
        if owner.build.is_empty() {
            return Ok(());
        }
        let build = Path::new(owner.build);
        let directory = owner.directory_of(build)?;
        let local = if hydrate {
            self.local(owner.build)?
        } else {
            owner.build.into()
        };
        let local = Path::new(&local);
        let local_directory = owner.directory_of(local)?;
        let Some(basename) = local_directory.file_name().and_then(|name| name.to_str()) else {
            return Ok(());
        };
        let portable = if hydrate {
            directory.to_string_lossy().into_owned()
        } else {
            self.portable(&directory.to_string_lossy())?
        };
        let marker = portable.trim_end_matches('/');
        let (from, to) = if hydrate {
            (marker, basename)
        } else {
            (basename, marker)
        };
        if let Some(project) = project_name(value.as_str().unwrap_or_default(), from, to) {
            *value = project.into();
        }
        Ok(())
    }
}

/// Validate logical path syntax without requiring a local checkout mapping.
pub(super) fn validate_field(key: &str, value: &Value) -> Result<()> {
    match key {
        "file" | "build" => validate_value(value),
        "roots" | "folder_project_builds" | "identity_roots" => value
            .as_array()
            .context("Portable roots must be an array")?
            .iter()
            .try_for_each(validate_value),
        _ => Ok(()),
    }
}
fn validate_value(value: &Value) -> Result<()> {
    let text = value
        .as_str()
        .context("Portable graph path must be a string")?;
    if text.is_empty() {
        return Ok(());
    }
    let (name, relative) = text
        .strip_prefix('@')
        .and_then(|path| path.split_once('/'))
        .context("Graph path must use a logical root")?;
    ensure!(
        !name.is_empty()
            && name.chars().all(
                |character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
            ),
        "Invalid portable logical root"
    );
    ensure!(
        Path::new(relative)
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir)),
        "Portable graph path escapes its root"
    );
    Ok(())
}

/// Restore UI paths relative to the selected TypeScript build; export logical paths.
pub(super) struct SourceDisplay {
    from: String,
    to: String,
}
impl SourceDisplay {
    pub fn rewrite(&self, key: &str, value: &mut Value) {
        let (from, to) = match key {
            "container" => (self.from.clone(), self.to.clone()),
            "namespace" => (dirname(&self.from), dirname(&self.to)),
            _ => return,
        };
        if value.as_str() == Some(from.as_str()) {
            *value = to.into();
        }
    }
}
fn dirname(path: &str) -> String {
    Path::new(path)
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(
            || ".".into(),
            |parent| parent.to_string_lossy().into_owned(),
        )
}
impl Paths {
    pub fn source_display(
        &self,
        file: &str,
        build: &str,
        hydrate: bool,
    ) -> Result<Option<SourceDisplay>> {
        if file.is_empty() || build.is_empty() {
            return Ok(None);
        }
        let local_file = if hydrate {
            self.local(file)?
        } else {
            file.into()
        };
        let local_build = if hydrate {
            self.local(build)?
        } else {
            build.into()
        };
        let directory = Path::new(&local_build)
            .parent()
            .context("TypeScript config has no parent")?;
        let relative = relative_path(directory, Path::new(&local_file));
        let (from, to) = if hydrate {
            (file.into(), relative)
        } else {
            (relative, self.portable(file)?)
        };
        Ok(Some(SourceDisplay { from, to }))
    }
}
fn relative_path(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let target: Vec<_> = to.components().collect();
    let shared = from
        .iter()
        .zip(&target)
        .take_while(|(left, right)| left == right)
        .count();
    if shared == 0 {
        return to.to_string_lossy().into_owned();
    }
    let mut relative = PathBuf::new();
    for _ in &from[shared..] {
        relative.push("..");
    }
    for component in &target[shared..] {
        relative.push(component.as_os_str());
    }
    if relative.as_os_str().is_empty() {
        ".".into()
    } else {
        relative.to_string_lossy().into_owned()
    }
}

fn project_name(value: &str, from: &str, to: &str) -> Option<String> {
    if value == from {
        return Some(to.into());
    }
    value
        .strip_prefix(&format!("{from}:"))
        .map(|suffix| format!("{to}:{suffix}"))
}

fn replace_root(text: &str, mapping: &(String, PathBuf)) -> String {
    let (name, path) = mapping;
    let mut spelling = path.to_string_lossy().into_owned();
    let mut result = text.to_owned();
    while spelling.len() <= text.len() {
        result = replace_spelling(
            &result,
            RootSpelling {
                alias: name,
                text: &spelling,
            },
        );
        let encoded = serde_json::to_string(&spelling).expect("String root serialization");
        let escaped = encoded[1..encoded.len() - 1].to_owned();
        if escaped == spelling {
            break;
        }
        spelling = escaped;
    }
    result
}
struct RootSpelling<'a> {
    alias: &'a str,
    text: &'a str,
}
fn replace_spelling(text: &str, spelling: RootSpelling<'_>) -> String {
    let RootSpelling {
        alias: name,
        text: root,
    } = spelling;
    let mut result = String::new();
    let mut copied = 0;
    for (start, _) in text.match_indices(root) {
        let end = start + root.len();
        let suffix = text[end..].chars().next();
        if suffix.is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) {
            continue;
        }
        result.push_str(&text[copied..start]);
        result.push_str(&format!("@{name}"));
        if suffix != Some('/') {
            result.push('/');
        }
        copied = end;
    }
    result.push_str(&text[copied..]);
    result
}
