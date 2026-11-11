//! Independent code inclusion and relationship selections.
mod settings;
use crate::model::{Edge, Node, RelationshipClassification, RelationshipKind, TargetResolution};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "settings::Settings", into = "settings::Settings")]
pub struct Filter {
    pub include_tests: bool,
    pub include_generated: bool,
    pub include_unavailable: bool,
    pub relationship_kinds: BTreeSet<RelationshipKind>,
    pub target_resolutions: BTreeSet<TargetResolution>,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            include_tests: false,
            include_generated: false,
            include_unavailable: false,
            relationship_kinds: RelationshipKind::ALL.into(),
            target_resolutions: TargetResolution::ALL.into(),
        }
    }
}

impl Filter {
    pub fn accepts_node(&self, node: &Node) -> bool {
        (self.include_tests || !node.is_test)
            && (self.include_generated || !node.is_generated)
            && (self.include_unavailable || !node.body_unavailable)
    }

    pub fn accepts_classification(&self, classification: RelationshipClassification) -> bool {
        self.relationship_kinds.contains(&classification.kind)
            && self.target_resolutions.contains(&classification.resolution)
    }

    /// For unindexed analyzer results. Graph operations use generation-cached classifications.
    pub fn accepts_edge(&self, edge: &Edge) -> bool {
        self.accepts_classification(edge.classification())
    }
}
