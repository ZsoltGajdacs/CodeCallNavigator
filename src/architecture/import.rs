//! Translate review-app YAML at the boundary; native matching stays format-independent.
use super::{Configuration, LanguageRules, LayerDefinition, Matcher};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Review {
    extends: Vec<String>,
    repositories: Vec<String>,
    java: Option<ReviewLanguage>,
    ts: Option<ReviewLanguage>,
    // Coverage is review-only and is reported rather than discarded.
    coverage: Option<serde_yaml::Value>,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ReviewLanguage {
    tests: Vec<ReviewMatcher>,
    other: Vec<ReviewMatcher>,
    layers: Vec<ReviewLayer>,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ReviewLayer {
    name: String,
    tab: Option<String>,
    flow: Option<String>,
    support: bool,
    beside: Option<String>,
    r#match: Vec<ReviewMatcher>,
}
#[derive(Default, Deserialize)]
#[serde(default)]
struct ReviewMatcher {
    #[serde(flatten)]
    fields: BTreeMap<String, serde_yaml::Value>,
}

fn matcher(raw: ReviewMatcher) -> Result<Matcher> {
    let mut fields = serde_yaml::Mapping::new();
    for (name, value) in raw.fields {
        let native = match name.as_str() {
            "package" => "namespace",
            "methodAnnotation" => "method_annotation",
            "implements" => "base_type",
            "implementsAnnotated" => "annotated_interface",
            "path" | "suffix" | "annotation" | "interface" | "kind" | "content" => name.as_str(),
            _ => anyhow::bail!("Unsupported review matcher field: {name}"),
        };
        fields.insert(native.into(), value);
    }
    let mut matcher: Matcher = serde_yaml::from_value(serde_yaml::Value::Mapping(fields))?;
    if !matcher.path.is_empty() {
        matcher.path_base = super::config::PathBase::Build;
    }
    Ok(matcher)
}
fn language(raw: ReviewLanguage) -> Result<LanguageRules> {
    let convert =
        |rules: Vec<ReviewMatcher>| rules.into_iter().map(matcher).collect::<Result<Vec<_>>>();
    Ok(LanguageRules {
        tests: convert(raw.tests)?,
        other: convert(raw.other)?,
        layers: raw
            .layers
            .into_iter()
            .map(|l| {
                Ok(LayerDefinition {
                    name: l.name,
                    tab: l.tab,
                    flow: l.flow,
                    support: l.support,
                    beside: l.beside,
                    matchers: convert(l.r#match)?,
                })
            })
            .collect::<Result<_>>()?,
    })
}

pub fn preset(name: &str) -> Result<&'static str> {
    match name {
        "spring" => Ok(include_str!("presets/spring.yml")),
        "react" => Ok(include_str!("presets/react.yml")),
        _ => anyhow::bail!("Unknown layer preset: {name}"),
    }
}

pub fn read(path: &Path) -> Result<Configuration> {
    read_recursive(path, &mut Vec::new())
}
fn read_recursive(path: &Path, seen: &mut Vec<PathBuf>) -> Result<Configuration> {
    let path = path
        .canonicalize()
        .with_context(|| format!("Cannot read review configuration {}", path.display()))?;
    ensure!(
        !seen.contains(&path),
        "Layer configuration inheritance cycle at {}",
        path.display()
    );
    seen.push(path.clone());
    let text = std::fs::read_to_string(&path)?;
    let raw: Review = serde_yaml::from_str(&text)?;
    let mut config = Configuration::default();
    for parent in &raw.extends {
        let inherited = match parent.as_str() {
            "spring" | "react" => convert(preset(parent)?)?,
            _ => read_recursive(&path.parent().unwrap().join(parent), seen)?,
        };
        config.languages.extend(inherited.languages);
    }
    apply(&mut config, raw)?;
    seen.pop();
    super::config::validate(&config)?;
    super::rules::validate_patterns(&config)?;
    Ok(config)
}
pub fn convert(text: &str) -> Result<Configuration> {
    let raw: Review = serde_yaml::from_str(text)?;
    ensure!(
        raw.extends.is_empty(),
        "Preset inheritance must be loaded from a repository file"
    );
    let mut config = Configuration::default();
    apply(&mut config, raw)?;
    super::config::validate(&config)?;
    super::rules::validate_patterns(&config)?;
    Ok(config)
}
fn apply(config: &mut Configuration, raw: Review) -> Result<()> {
    ensure!(
        raw.coverage.is_none(),
        "Review coverage configuration is not supported by layer import; move it out of layers.yml before importing"
    );
    let _ = raw.repositories; // Repository selectors are replaced by native file ownership.
    if let Some(java) = raw.java {
        config.languages.insert("java".into(), language(java)?);
    }
    if let Some(ts) = raw.ts {
        config.languages.insert("typescript".into(), language(ts)?);
    }
    Ok(())
}
