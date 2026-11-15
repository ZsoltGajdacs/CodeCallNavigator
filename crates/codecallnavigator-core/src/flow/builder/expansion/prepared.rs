//! Exact input identities and reusable original-reference indexes, independent of request order.
use super::super::recording::OriginalIndex;
use crate::flow::*;
use anyhow::{Context, Result, ensure};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
struct Indexed<'a> {
    index: Rc<OriginalIndex>,
    sites: Vec<LocatedCallSite<'a>>,
}
pub(super) struct Prepared<'a> {
    inputs: HashMap<&'a str, FunctionSource<'a>>,
    indexes: RefCell<HashMap<&'a str, Indexed<'a>>>,
    pub requests: Vec<RequestedCall>,
}
impl<'a> Prepared<'a> {
    pub fn new(inputs: &[FunctionSource<'a>], request: &ExpansionRequest) -> Result<Self> {
        request.validate()?;
        let mut identities = HashMap::new();
        let mut owners = HashMap::new();
        for input in inputs {
            let function = input.function;
            ensure!(
                !function.symbol.trim().is_empty(),
                "Immutable function input has no symbol"
            );
            super::owner(function).validate()?;
            ensure!(
                identities
                    .insert(function.symbol.as_str(), *input)
                    .is_none(),
                "Duplicate immutable function input: {}",
                function.symbol
            );
            let key = (
                function.build.as_str(),
                function.project.as_str(),
                function.file.as_str(),
                function.language.as_str(),
            );
            if let Some(content) = owners.insert(key, input.content) {
                ensure!(
                    content == input.content,
                    "Conflicting analyzed source owner"
                );
            }
        }
        let mut keyed = request
            .selections()
            .iter()
            .cloned()
            .map(|entry| {
                Ok((
                    serde_json::to_string(&(&entry.caller, &entry.choice.site))?,
                    entry,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        let requests = keyed.into_iter().map(|(_, entry)| entry).collect();
        let prepared = Self {
            inputs: identities,
            indexes: RefCell::new(HashMap::new()),
            requests,
        };
        prepared.validate_requests(request.root())?;
        Ok(prepared)
    }
    pub fn input(&self, symbol: &str) -> Option<FunctionSource<'a>> {
        self.inputs.get(symbol).copied()
    }
    pub fn index(&self, symbol: &str) -> Result<Rc<OriginalIndex>> {
        if let Some(index) = self.indexes.borrow().get(symbol) {
            return Ok(index.index.clone());
        }
        let input = self
            .input(symbol)
            .context("Expansion function is unavailable")?;
        validate_source(input.function, input.content)?;
        let index = Rc::new(OriginalIndex::new(input.function));
        self.indexes.borrow_mut().insert(
            input.function.symbol.as_str(),
            Indexed {
                index: index.clone(),
                sites: call_sites(input.function),
            },
        );
        Ok(index)
    }
    fn validate_requests(&self, root: &str) -> Result<()> {
        self.validate_ancestry()?;
        self.validate_sites(root)
    }
    fn validate_ancestry(&self) -> Result<()> {
        let mut keys = HashSet::new();
        for entry in &self.requests {
            ensure!(
                keys.insert((&entry.caller, &entry.choice.site)),
                "Duplicate or conflicting selected call site"
            );
            for depth in 0..entry.caller.0.len() {
                let ancestor = &entry.caller.0[depth];
                ensure!(
                    self.requests
                        .iter()
                        .any(|candidate| candidate.caller.0 == entry.caller.0[..depth]
                            && candidate.choice == *ancestor),
                    "Missing explicit selected ancestor"
                );
            }
        }
        Ok(())
    }
    fn validate_sites(&self, root: &str) -> Result<()> {
        for index in 0..self.requests.len() {
            let symbol = self.requests[index]
                .caller
                .0
                .last()
                .map(|choice| choice.target.symbol.as_str())
                .unwrap_or(root)
                .to_owned();
            if self.input(&symbol).is_none() {
                continue;
            }
            self.index(&symbol)?;
            let choice = &self.requests[index].choice;
            let indexes = self.indexes.borrow();
            let site = indexes[&*symbol]
                .sites
                .iter()
                .find(|site| {
                    site.address.path == choice.site.path
                        && site.address.call_slot == choice.site.call_slot
                })
                .context("Selected IR call site is unavailable or stale")?;
            ensure!(
                site.site.targets.contains(&choice.target),
                "Selected target is unavailable or stale"
            );
        }
        Ok(())
    }
}
