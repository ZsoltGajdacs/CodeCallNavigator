//! Exact immutable source snapshots, without artifact or filesystem presentation coupling.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceId(pub usize);
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceOwner {
    pub build: String,
    pub project: String,
    pub file: String,
    pub language: String,
}
impl SourceOwner {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            [&self.build, &self.project, &self.file, &self.language]
                .iter()
                .all(|part| !part.trim().is_empty()),
            "Source owner is incomplete"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSnapshot {
    pub owner: SourceOwner,
    pub content: String,
    pub digest: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceTable(Vec<SourceSnapshot>);
impl SourceTable {
    /// Repeated owners compare borrowed bytes; text is cloned only on first insertion.
    pub fn intern(&mut self, owner: SourceOwner, content: &str) -> Result<SourceId> {
        owner.validate()?;
        if let Some(index) = self.0.iter().position(|source| source.owner == owner) {
            ensure!(
                self.0[index].content == content,
                "Conflicting analyzed source for {}",
                owner.file
            );
            return Ok(SourceId(index));
        }
        let id = SourceId(self.0.len());
        self.0.push(SourceSnapshot {
            owner,
            content: content.into(),
            digest: blake3::hash(content.as_bytes()).to_hex().to_string(),
        });
        Ok(id)
    }
    pub fn get(&self, id: SourceId) -> Option<&SourceSnapshot> {
        self.0.get(id.0)
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn byte_count(&self) -> Result<usize> {
        self.0.iter().try_fold(0usize, |total, source| {
            total
                .checked_add(source.content.len())
                .ok_or_else(|| anyhow::anyhow!("Source byte count overflow"))
        })
    }
    pub fn validate(&self) -> Result<()> {
        let mut owners = HashSet::new();
        for source in &self.0 {
            source.owner.validate()?;
            ensure!(owners.insert(&source.owner), "Duplicate source owner");
            ensure!(
                source.digest == blake3::hash(source.content.as_bytes()).to_hex().to_string(),
                "Source digest does not match exact analyzed bytes"
            );
        }
        self.byte_count()?;
        Ok(())
    }
}
