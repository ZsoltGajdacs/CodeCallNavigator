//! Compile input patterns once; classification is deterministic and side-effect free.
use super::{Configuration, FileFacts, Matcher};
use anyhow::Result;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use regex::Regex;
use std::path::Path;

pub struct CompiledMatcher {
    rule: Matcher,
    paths: Option<GlobSet>,
    namespaces: Option<GlobSet>,
    content: Option<Regex>,
}

fn globs(patterns: &[String], separators: bool) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(
            GlobBuilder::new(pattern)
                .literal_separator(separators)
                .build()?,
        );
    }
    Ok(Some(builder.build()?))
}

impl CompiledMatcher {
    pub fn new(rule: &Matcher) -> Result<Self> {
        Ok(Self {
            rule: rule.clone(),
            paths: globs(&rule.path, true)?,
            namespaces: globs(&rule.namespace, false)?,
            content: rule.content.as_deref().map(Regex::new).transpose()?,
        })
    }
    pub fn matches(&self, facts: &FileFacts, root: &Path) -> bool {
        self.location_matches(facts, root)
            && self.declaration_matches(facts)
            && self
                .content
                .as_ref()
                .is_none_or(|p| p.is_match(&facts.content))
    }

    fn location_matches(&self, facts: &FileFacts, root: &Path) -> bool {
        let file = Path::new(&facts.file);
        let root = match self.rule.path_base {
            super::config::PathBase::Repository => root,
            super::config::PathBase::Build => {
                let build = Path::new(&facts.build);
                if facts.language == "java" {
                    build
                } else {
                    build.parent().unwrap_or(build)
                }
            }
        };
        let relative = file.strip_prefix(root).unwrap_or(file);
        let namespaces = self.namespaces.as_ref().is_none_or(|patterns| {
            facts
                .namespaces
                .iter()
                .any(|n| patterns.is_match(n) || patterns.is_match(format!("{n}.")))
        });
        self.paths.as_ref().is_none_or(|p| p.is_match(relative)) && namespaces
    }

    fn declaration_matches(&self, facts: &FileFacts) -> bool {
        let names = std::iter::once(
            Path::new(&facts.file)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        )
        .chain(facts.names.clone());
        let suffix = self.rule.suffix.is_empty()
            || names
                .into_iter()
                .any(|n| self.rule.suffix.iter().any(|s| n.ends_with(s)));
        let named_facts = [
            (&self.rule.annotation, &facts.annotations),
            (&self.rule.method_annotation, &facts.method_annotations),
            (&self.rule.kind, &facts.kinds),
            (&self.rule.base_type, &facts.base_types),
            (&self.rule.annotated_interface, &facts.annotated_interfaces),
        ];
        suffix
            && self.rule.interface.is_none_or(|v| v == facts.is_interface)
            && named_facts
                .into_iter()
                .all(|(patterns, values)| any_names(patterns, values, &facts.language))
    }
}

fn name(value: &str, language: &str) -> String {
    let value = value.trim_start_matches("global::");
    if language == "csharp" {
        value.strip_suffix("Attribute").unwrap_or(value).to_owned()
    } else {
        value.to_owned()
    }
}
fn any_names(patterns: &[String], values: &[String], language: &str) -> bool {
    patterns.is_empty()
        || patterns.iter().any(|p| {
            values.iter().any(|v| {
                let expected = name(p, language);
                let actual = name(v, language);
                actual == expected
                    || (!expected.contains('.')
                        && actual.rsplit(['.', '$']).next() == Some(expected.as_str()))
            })
        })
}

pub fn validate_patterns(config: &Configuration) -> Result<()> {
    for rules in config.languages.values() {
        for matcher in rules
            .tests
            .iter()
            .chain(&rules.other)
            .chain(rules.layers.iter().flat_map(|l| &l.matchers))
        {
            CompiledMatcher::new(matcher)?;
        }
    }
    Ok(())
}
