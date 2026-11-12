//! Filesystem configuration boundary: native priority and race-safe one-time import.
use super::{Configuration, Selection};
use anyhow::{Context, Result, ensure};
use std::{io::Write, path::Path};

pub struct LoadedConfiguration {
    pub config: Configuration,
    pub origin: String,
}

pub fn load(root: &Path, selection: Selection) -> Result<LoadedConfiguration> {
    load_with_policy(root, selection, ImportPolicy::Persist)
}

/// Observe repository rules, including review-format conversion, without writing
/// imported configuration. Use this boundary for read-only graph queries.
pub fn load_readonly(root: &Path, selection: Selection) -> Result<LoadedConfiguration> {
    load_with_policy(root, selection, ImportPolicy::Observe)
}

#[derive(Clone, Copy)]
enum ImportPolicy {
    Observe,
    Persist,
}

fn load_with_policy(
    root: &Path,
    selection: Selection,
    policy: ImportPolicy,
) -> Result<LoadedConfiguration> {
    let normalized = root.canonicalize().or_else(|_| std::path::absolute(root))?;
    let root = normalized.as_path();
    let (mut config, origin) = match selection {
        Selection::None => (
            Configuration::default(),
            "None · choose a preset or add .codecallnavigator/layers.yml".into(),
        ),
        Selection::Spring => (
            super::import::convert(super::import::preset("spring")?)?,
            "Spring preset".into(),
        ),
        Selection::React => (
            super::import::convert(super::import::preset("react")?)?,
            "React preset".into(),
        ),
        Selection::Repository => repository(root, policy)?,
    };
    config.root = root.to_path_buf();
    Ok(LoadedConfiguration { config, origin })
}
fn repository(root: &Path, policy: ImportPolicy) -> Result<(Configuration, String)> {
    let native = root.join(".codecallnavigator/layers.yml");
    if !native.exists() {
        let review = root.join(".interactive-review/layers.yml");
        ensure!(
            review.is_file(),
            "Add .codecallnavigator/layers.yml or select Spring or React to configure layers"
        );
        let config = super::import::read(&review)?;
        if matches!(policy, ImportPolicy::Observe) {
            return Ok((config, review.display().to_string()));
        }
        let text = serde_yaml::to_string(&config)?;
        let directory = native.parent().unwrap();
        std::fs::create_dir_all(directory)?;
        let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
        temporary.write_all(text.as_bytes())?;
        temporary.as_file().sync_all()?;
        match temporary.persist_noclobber(&native) {
            Ok(_) => {}
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.error.into()),
        }
    }
    let text = std::fs::read_to_string(&native)
        .with_context(|| format!("Cannot read {}", native.display()))?;
    Ok((super::parse(&text)?, native.display().to_string()))
}
