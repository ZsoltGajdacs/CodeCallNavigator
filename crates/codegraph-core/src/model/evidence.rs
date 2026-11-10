//! Source-backed explanations describe static relationships, never runtime frequency.
use crate::flow::SourceSpan;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSite {
    pub file: String,
    pub line: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipEvidence {
    pub rule: String,
    pub version: u32,
    /// Source construct establishing the rule, rather than an observed execution.
    pub invocation: SourceSite,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration: Option<SourceSite>,
    #[serde(default)]
    pub supporting_symbols: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// Opaque rule-specific restrictions, not observed runtime state or references.
    /// Symbol and path references belong in supporting_symbols and SourceSite.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub qualifiers: BTreeMap<String, String>,
    pub semantics: String,
}

impl RelationshipEvidence {
    pub(super) fn validate(&self, identities: &HashSet<&str>) -> Result<()> {
        ensure!(
            !self.rule.trim().is_empty(),
            "Relationship evidence needs a rule"
        );
        ensure!(
            self.version == 1,
            "Unsupported relationship evidence version: {}",
            self.version
        );
        ensure!(
            matches!(
                self.semantics.as_str(),
                "reference"
                    | "possible-invocation"
                    | "deferred"
                    | "async-boundary"
                    | "navigation"
                    | "http"
            ),
            "Unsupported relationship evidence semantics: {}",
            self.semantics
        );
        self.invocation.validate()?;
        if let Some(site) = &self.registration {
            site.validate()?;
        }
        ensure!(
            self.qualifiers.keys().all(|key| !key.trim().is_empty()),
            "Relationship qualifier needs a name"
        );
        for symbol in &self.supporting_symbols {
            ensure!(
                identities.contains(symbol.as_str()),
                "Relationship evidence references missing symbol: {symbol}"
            );
        }
        Ok(())
    }
}

impl SourceSite {
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.file.trim().is_empty(),
            "Relationship evidence needs a source file"
        );
        ensure!(self.line > 0, "Relationship evidence line must be positive");
        ensure!(
            self.column.is_none_or(|column| column > 0),
            "Relationship evidence column must be positive"
        );
        ensure!(
            self.span.is_none_or(|span| span.start < span.end),
            "Relationship evidence span must be nonempty and ordered"
        );
        Ok(())
    }
}
