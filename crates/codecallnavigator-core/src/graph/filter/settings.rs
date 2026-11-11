//! Session compatibility belongs at deserialization, not in graph filtering.
use super::Filter;
use crate::model::{RelationshipKind, TargetResolution};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const VERSION: u32 = 2;

#[derive(Deserialize, Serialize)]
pub struct Settings {
    #[serde(default)]
    version: Option<u32>,
    #[serde(default)]
    include_tests: bool,
    #[serde(default)]
    include_generated: bool,
    #[serde(default)]
    include_unavailable: bool,
    #[serde(default)]
    relationship_kinds: Option<BTreeSet<RelationshipKind>>,
    #[serde(default)]
    target_resolutions: Option<BTreeSet<TargetResolution>>,
}

impl TryFrom<Settings> for Filter {
    type Error = String;

    fn try_from(settings: Settings) -> Result<Self, Self::Error> {
        let mut filter = Self {
            include_tests: settings.include_tests,
            include_generated: settings.include_generated,
            include_unavailable: settings.include_unavailable,
            ..Default::default()
        };
        match settings.version {
            None => {} // Legacy relationship switches reset once, by explicit upgrade policy.
            Some(VERSION) => {
                filter.relationship_kinds = settings
                    .relationship_kinds
                    .ok_or("Missing relationship kinds")?;
                filter.target_resolutions = settings
                    .target_resolutions
                    .ok_or("Missing target resolutions")?;
            }
            Some(version) => return Err(format!("Unsupported graph filter version: {version}")),
        }
        Ok(filter)
    }
}

impl From<Filter> for Settings {
    fn from(filter: Filter) -> Self {
        Self {
            version: Some(VERSION),
            include_tests: filter.include_tests,
            include_generated: filter.include_generated,
            include_unavailable: filter.include_unavailable,
            relationship_kinds: Some(filter.relationship_kinds),
            target_resolutions: Some(filter.target_resolutions),
        }
    }
}
