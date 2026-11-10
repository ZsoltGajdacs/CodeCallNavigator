//! Versioned native configuration, independent of UI and analyzer adapters.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Selection {
    #[default]
    None,
    Spring,
    React,
    Repository,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub version: u32,
    #[serde(skip)]
    pub root: std::path::PathBuf,
    #[serde(default)]
    pub languages: std::collections::BTreeMap<String, LanguageRules>,
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            version: 1,
            root: Default::default(),
            languages: Default::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LanguageRules {
    pub tests: Vec<Matcher>,
    pub other: Vec<Matcher>,
    pub layers: Vec<LayerDefinition>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayerDefinition {
    pub name: String,
    pub tab: Option<String>,
    pub flow: Option<String>,
    pub support: bool,
    pub beside: Option<String>,
    pub matchers: Vec<Matcher>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathBase {
    #[default]
    Repository,
    Build,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Matcher {
    pub path_base: PathBase,
    #[serde(deserialize_with = "strings")]
    pub path: Vec<String>,
    #[serde(deserialize_with = "strings")]
    pub namespace: Vec<String>,
    #[serde(deserialize_with = "strings")]
    pub suffix: Vec<String>,
    #[serde(deserialize_with = "strings")]
    pub annotation: Vec<String>,
    #[serde(deserialize_with = "strings")]
    pub method_annotation: Vec<String>,
    pub interface: Option<bool>,
    #[serde(deserialize_with = "strings")]
    pub kind: Vec<String>,
    #[serde(deserialize_with = "strings")]
    pub base_type: Vec<String>,
    #[serde(deserialize_with = "strings")]
    pub annotated_interface: Vec<String>,
    pub content: Option<String>,
}

fn strings<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Values {
        One(String),
        Many(Vec<String>),
    }
    Ok(match Values::deserialize(d)? {
        Values::One(v) => vec![v],
        Values::Many(v) => v,
    })
}

pub fn parse(text: &str) -> Result<Configuration> {
    let config: Configuration = serde_yaml::from_str(text)?;
    validate(&config)?;
    super::rules::validate_patterns(&config)?;
    Ok(config)
}

pub fn validate(config: &Configuration) -> Result<()> {
    ensure!(
        config.version == 1,
        "Unsupported layer configuration version {}",
        config.version
    );
    let mut layouts = std::collections::HashMap::new();
    for (language, rules) in &config.languages {
        ensure!(
            ["java", "csharp", "typescript"].contains(&language.as_str()),
            "Unsupported layer language: {language}"
        );
        validate_language(rules)?;
        for layer in &rules.layers {
            let layout = (&layer.tab, &layer.flow, layer.support, &layer.beside);
            if let Some(previous) = layouts.insert(&layer.name, layout) {
                ensure!(
                    previous == layout,
                    "Conflicting layout for layer {} across languages",
                    layer.name
                );
            }
        }
    }
    Ok(())
}

fn validate_language(rules: &LanguageRules) -> Result<()> {
    let mut seen = std::collections::HashSet::new();
    for layer in &rules.layers {
        ensure!(!layer.name.trim().is_empty(), "A layer has an empty name");
        ensure!(
            !["Tests", "Other"].contains(&layer.name.as_str()),
            "{} is a reserved layer name",
            layer.name
        );
        ensure!(seen.insert(&layer.name), "Duplicate layer: {}", layer.name);
        ensure!(
            !layer.matchers.is_empty(),
            "Layer {} has no matchers",
            layer.name
        );
        ensure!(
            !layer.support || layer.flow.is_none(),
            "Support layer {} cannot define a flow",
            layer.name
        );
    }
    for layer in &rules.layers {
        validate_beside(layer, rules)?;
    }
    Ok(())
}

fn validate_beside(layer: &LayerDefinition, rules: &LanguageRules) -> Result<()> {
    let Some(beside) = &layer.beside else {
        return Ok(());
    };
    ensure!(layer.support, "Only support layers can define beside");
    let target = rules.layers.iter().find(|l| &l.name == beside);
    ensure!(
        beside != &layer.name
            && (target.is_some() || ["Tests", "Other"].contains(&beside.as_str())),
        "Unknown beside lane: {beside}"
    );
    if let Some(target) = target {
        ensure!(
            layer.tab == target.tab,
            "Beside layers must be in the same tab"
        );
        ensure!(
            target.beside.is_none(),
            "Beside targets cannot themselves have a beside target"
        );
    }
    Ok(())
}
