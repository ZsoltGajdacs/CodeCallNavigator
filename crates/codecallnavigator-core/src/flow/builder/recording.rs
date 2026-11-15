//! Expansion-only original-reference indexing. Pointer keys are never dereferenced,
//! serialized or ordered; ordinary builds do not allocate this index.
use crate::flow::{
    parts::{Part, Parts},
    *,
};
use anyhow::{Context, Result};
use std::collections::HashMap;

pub(super) struct OriginalIndex {
    calls: HashMap<*const CallSite, SiteAddress>,
    statements: HashMap<*const Statement, IrPath>,
}
pub(super) struct Recording {
    index: std::rc::Rc<OriginalIndex>,
    pub origins: Vec<StepOrigin>,
    pub cleanup: Vec<CleanupCopy>,
    pub deferred: usize,
    occurrence: OccurrenceId,
    source: SourceId,
}
impl OriginalIndex {
    pub fn new(function: &FunctionFlowIR) -> Self {
        let mut calls = HashMap::new();
        let mut statements = HashMap::new();
        let mut parts = Parts::located(function);
        while let Some(located) = parts.next_located() {
            match located.part {
                Part::Statement(statement) => {
                    statements.insert(statement as *const _, parts.path(located.path));
                }
                Part::Call { site, .. } => {
                    let mut path = parts.path(located.path);
                    let Some(IrPathComponent::Index { index: call_slot }) = path.0.pop() else {
                        unreachable!("typed call index")
                    };
                    path.0.pop();
                    calls.insert(
                        site as *const _,
                        SiteAddress {
                            path,
                            call_slot,
                            cleanup: vec![],
                        },
                    );
                }
                _ => (),
            }
        }
        Self { calls, statements }
    }
}
impl Recording {
    pub fn new(
        index: std::rc::Rc<OriginalIndex>,
        occurrence: OccurrenceId,
        source: SourceId,
    ) -> Self {
        Self {
            index,
            origins: vec![],
            cleanup: vec![],
            deferred: 0,
            occurrence,
            source,
        }
    }
    pub fn occurrence(&self) -> OccurrenceId {
        self.occurrence
    }
    pub fn address(&self, site: &CallSite) -> Result<SiteAddress> {
        let mut address = self
            .index
            .calls
            .get(&(site as *const _))
            .context("Call is not an original indexed IR site")?
            .clone();
        address.cleanup = self.cleanup.clone();
        Ok(address)
    }
    fn statement(&self, statement: &Statement) -> Result<IrPath> {
        self.index
            .statements
            .get(&(statement as *const _))
            .cloned()
            .context("Statement is not an original indexed IR node")
    }
    pub fn scope(&self, statement: &Statement, field: IrField) -> Result<CleanupSite> {
        let path = self.statement(statement)?;
        Ok(match field {
            IrField::Finally => CleanupSite::Finally {
                block: path.field(field),
            },
            IrField::Cleanup => CleanupSite::Cleanup {
                block: path.field(field),
            },
            IrField::Resources => CleanupSite::Resource { resource: path },
            _ => unreachable!("typed cleanup scope"),
        })
    }
    pub fn step(&mut self, calls: &[CallSite]) -> Result<()> {
        let calls = calls
            .iter()
            .map(|site| self.address(site))
            .collect::<Result<_>>()?;
        self.origins.push(StepOrigin {
            occurrence: self.occurrence,
            source: Some(self.source),
            role: OriginRole::Original,
            calls,
        });
        Ok(())
    }
    pub fn role(&mut self, id: usize, role: OriginRole) {
        self.origins[id].role = role;
    }
}
