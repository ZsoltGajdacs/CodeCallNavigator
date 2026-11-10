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

    fn local(&self, text: &str) -> Result<String> {
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

    pub fn project(&self, value: &mut Value, hydrate: bool) {
        for (name, root) in &self.roots {
            let Some(base) = root.file_name().and_then(|base| base.to_str()) else { continue; };
            let portable = format!("@{name}");
            let (from, to) = if hydrate { (portable.as_str(), base) } else { (base, portable.as_str()) };
            if let Some(rewritten) = project_name(value.as_str().unwrap_or_default(), from, to) {
                *value = rewritten.into();
                return;
            }
        }
    }
}

fn project_name(value: &str, from: &str, to: &str) -> Option<String> {
    if value == from { return Some(to.into()); }
    value.strip_prefix(&format!("{from}:")).map(|suffix| format!("{to}:{suffix}"))
}

fn replace_root(text: &str, mapping: &(String, PathBuf)) -> String {
    let (name, root) = mapping;
    let root = root.to_string_lossy();
    let mut result = String::new();
    let mut copied = 0;
    for (start, _) in text.match_indices(root.as_ref()) {
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
