//! A function/region boundary retains abrupt exits until its owning caller finishes them.
use super::{Builder, Exit, StepSpec};
use crate::flow::Statement;
use anyhow::Result;

pub(super) struct FunctionAssembly {
    pub entry: usize,
    pub exits: Vec<Exit>,
}
impl Builder<'_> {
    pub(super) fn function(
        &mut self,
        entry: StepSpec<'_>,
        body: &[Statement],
    ) -> Result<FunctionAssembly> {
        let entry = self.step(entry, &[])?;
        let exits = self.block(body, vec![Exit::normal(entry)])?;
        Ok(FunctionAssembly { entry, exits })
    }
}
