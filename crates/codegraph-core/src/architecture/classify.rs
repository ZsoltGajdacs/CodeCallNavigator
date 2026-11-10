//! Layer assignments are a disposable projection over immutable compiler facts.
use super::{
    Assignment, Configuration, FileFacts, LayerAssignments, LayerDefinition, facts::FACTS_VERSION,
    file_key, rules::CompiledMatcher,
};
use crate::model::GraphData;
use anyhow::{Result, ensure};
use std::collections::HashMap;

struct CompiledLanguage {
    tests: Vec<CompiledMatcher>,
    other: Vec<CompiledMatcher>,
    layers: Vec<(String, Vec<CompiledMatcher>)>,
}
impl CompiledLanguage {
    fn new(rules: &super::LanguageRules) -> Result<Self> {
        let compile = |rules: &[super::Matcher]| {
            rules
                .iter()
                .map(CompiledMatcher::new)
                .collect::<Result<Vec<_>>>()
        };
        Ok(Self {
            tests: compile(&rules.tests)?,
            other: compile(&rules.other)?,
            layers: rules
                .layers
                .iter()
                .map(|l| Ok((l.name.clone(), compile(&l.matchers)?)))
                .collect::<Result<_>>()?,
        })
    }
    fn assign(&self, facts: &FileFacts, root: &std::path::Path) -> Assignment {
        if facts.is_test || self.tests.iter().any(|m| m.matches(facts, root)) {
            return Assignment {
                layer: "Tests".into(),
                rule: "Test source or tests rule".into(),
            };
        }
        if self.other.iter().any(|m| m.matches(facts, root)) {
            return Assignment {
                layer: "Other".into(),
                rule: "Other rule".into(),
            };
        }
        for (layer, matchers) in &self.layers {
            if let Some(index) = matchers.iter().position(|m| m.matches(facts, root)) {
                return Assignment {
                    layer: layer.clone(),
                    rule: format!("{layer}, matcher {}", index + 1),
                };
            }
        }
        Assignment {
            layer: "Other".into(),
            rule: "No matching layer rule".into(),
        }
    }
}

pub fn classify(
    graph: &GraphData,
    config: &Configuration,
    origin: String,
) -> Result<LayerAssignments> {
    super::config::validate(config)?;
    let facts = graph
        .architecture
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Refresh the graph to collect architectural layer facts"))?;
    ensure!(
        facts.version == FACTS_VERSION,
        "Refresh the graph to update architectural layer facts"
    );
    let compiled = config
        .languages
        .iter()
        .map(|(k, v)| Ok((k.clone(), CompiledLanguage::new(v)?)))
        .collect::<Result<HashMap<_, _>>>()?;
    let mut files = HashMap::new();
    for file in &facts.files {
        let assignment = compiled
            .get(&file.language)
            .map(|r| r.assign(file, &config.root))
            .unwrap_or_else(|| Assignment {
                layer: if file.is_test { "Tests" } else { "Other" }.into(),
                rule: "No language rules".into(),
            });
        files.insert(file_key(&file.build, &file.project, &file.file), assignment);
    }
    Ok(LayerAssignments {
        files,
        layers: layer_definitions(config),
        origin,
    })
}

fn layer_definitions(config: &Configuration) -> Vec<LayerDefinition> {
    let mut layers: Vec<LayerDefinition> = Vec::new();
    for layer in config.languages.values().flat_map(|r| &r.layers) {
        if !layers.iter().any(|l| l.name == layer.name) {
            layers.push(layer.clone());
        }
    }
    layers.extend(["Tests", "Other"].map(|name| LayerDefinition {
        name: name.into(),
        ..Default::default()
    }));
    layers
}
