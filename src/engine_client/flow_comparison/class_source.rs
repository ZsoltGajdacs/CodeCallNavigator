use super::{BoundComparison, Header, client::Result};
use anyhow::ensure;
use codecallnavigator::{
    artifact::comparison_changes::{ClassSource, ClassSourceTarget},
    query::{ComparisonSide, Query},
};
use serde::Deserialize;
use std::sync::atomic::AtomicBool;

#[derive(Deserialize)]
struct Preview {
    #[serde(flatten)]
    header: Header,
    id: String,
    node: String,
    side: ComparisonSide,
    source: ClassSource,
}

impl BoundComparison {
    pub fn class_source(
        &self,
        target: ClassSourceTarget<'_>,
        cancel: &AtomicBool,
    ) -> Result<ClassSource> {
        if self
            .class_source_depth(cancel)?
            .is_none_or(|max| target.depth > max)
        {
            return Ok(ClassSource::Unavailable {
                reason: "The configured CLI does not provide saved class previews".into(),
            });
        }
        let result: Preview = self.bound_request(
            |refs, comparison_id| Query::ClassComparisonSource {
                base: refs.base.clone(),
                head: refs.head.clone(),
                base_snapshot: refs.base_snapshot.clone(),
                head_snapshot: refs.head_snapshot.clone(),
                expected_comparison: comparison_id.into(),
                id: target.root.into(),
                node: target.node.into(),
                side: target.side,
                depth: target.depth,
            },
            cancel,
            |result: &Preview| &result.header,
        )?;
        Ok(result.validated(target)?)
    }
}

impl Preview {
    fn validated(self, target: ClassSourceTarget<'_>) -> anyhow::Result<ClassSource> {
        ensure!(
            (self.id.as_str(), self.node.as_str(), self.side)
                == (target.root, target.node, target.side),
            "CLI returned saved source for another class or revision"
        );
        Ok(self.source)
    }
}
