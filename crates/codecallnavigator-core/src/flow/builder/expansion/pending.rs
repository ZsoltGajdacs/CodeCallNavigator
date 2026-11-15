//! Borrowed source staging; only a successful complete result materializes text.
use super::{budget::Allowance, owner};
use crate::flow::*;
use anyhow::{Result, ensure};
struct Pending<'a> {
    owner: SourceOwner,
    content: &'a str,
}
#[derive(Default)]
pub(super) struct SourcePool<'a> {
    items: Vec<Pending<'a>>,
}
impl<'a> SourcePool<'a> {
    pub fn intern(&mut self, input: FunctionSource<'a>, allowance: Allowance) -> Result<SourceId> {
        let owner = owner(input.function);
        if let Some(index) = self.items.iter().position(|source| source.owner == owner) {
            ensure!(
                self.items[index].content == input.content,
                "Conflicting analyzed source for {}",
                owner.file
            );
            return Ok(SourceId(index));
        }
        let bytes = self
            .items
            .iter()
            .map(|source| source.content.len())
            .sum::<usize>()
            .saturating_add(input.content.len());
        allowance.check(ExpansionLimit::SourceBytes, bytes as u64)?;
        let id = SourceId(self.items.len());
        self.items.push(Pending {
            owner,
            content: input.content,
        });
        Ok(id)
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn truncate(&mut self, len: usize) {
        self.items.truncate(len);
    }
    pub fn materialize(self) -> Result<SourceTable> {
        let mut table = SourceTable::default();
        for source in self.items {
            table.intern(source.owner, source.content)?;
        }
        Ok(table)
    }
}
