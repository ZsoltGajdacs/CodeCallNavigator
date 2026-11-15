//! Explicit selected occurrences assembled in builder order over borrowed immutable inputs.
mod attempt;
mod budget;
pub(super) use budget::Allowance;
mod child;
mod eligibility;
mod pending;
mod prepared;
mod retry;
mod run;
use super::{Builder, Exit, StepSpec, recording::Recording};
use crate::flow::*;
use anyhow::{Context, Result, ensure};
use pending::SourcePool;
use prepared::Prepared;
use run::Status;
use std::sync::atomic::{AtomicBool, Ordering};

pub(super) struct Selection<'a> {
    prepared: std::rc::Rc<Prepared<'a>>,
    statuses: Vec<Status>,
    root: String,
    sources: SourcePool<'a>,
    occurrences: Vec<FlowOccurrence>,
    limits: ExpansionLimits,
    mode: attempt::Mode,
    evaluated: usize,
}
struct Activation<'a> {
    call: usize,
    site: &'a CallSite,
    deferred: bool,
    caller: OccurrenceId,
    request: usize,
}
pub(super) enum SelectedValue {
    Expanded { exits: Vec<Exit> },
    Stopped { step: usize },
}
struct Activated<'a> {
    run: Selection<'a>,
    value: SelectedValue,
}
pub fn expand<'a>(
    inputs: &[FunctionSource<'a>],
    request: &ExpansionRequest,
    cancel: &'a AtomicBool,
) -> Result<ExpandedFlow> {
    ensure!(!cancel.load(Ordering::Relaxed), "Flow expansion cancelled");
    request.limits().validate()?;
    let prepared = Prepared::new(inputs, request)?;
    retry::expand(prepared, request, cancel)
}
fn owner(function: &FunctionFlowIR) -> SourceOwner {
    SourceOwner {
        build: function.build.clone(),
        project: function.project.clone(),
        file: function.file.clone(),
        language: function.language.clone(),
    }
}
impl<'a> Builder<'a> {
    pub(super) fn check_cancel(&self) -> Result<()> {
        ensure!(
            !self
                .cancel
                .is_some_and(|cancel| cancel.load(Ordering::Relaxed)),
            "Flow expansion cancelled"
        );
        Ok(())
    }
    pub(super) fn selected_value(
        &mut self,
        spec: &StepSpec<'_>,
        incoming: &[Exit],
    ) -> Result<Option<SelectedValue>> {
        let Some(run) = self.selection.as_ref() else {
            return Ok(None);
        };
        let recording = self.recording.as_ref().unwrap();
        let caller = recording.occurrence();
        if !run
            .prepared
            .requests
            .iter()
            .any(|request| request.caller == run.occurrences[caller.0].path)
        {
            return Ok(None);
        }
        let selected: Vec<_> = spec
            .calls
            .iter()
            .filter_map(|site| {
                let address = recording.address(site).ok()?;
                run.request(caller, &address).map(|request| (request, site))
            })
            .collect();
        if selected.is_empty() {
            return Ok(None);
        }
        ensure!(
            selected
                .iter()
                .all(|(request, _)| run.statuses[*request] == Status::Pending),
            "Duplicate selected call occurrence"
        );
        let deferred = recording.deferred > 0;
        let mut run = self.selection.take().unwrap();
        let call = self.step(spec.clone(), incoming)?;
        self.recording
            .as_mut()
            .unwrap()
            .role(call, OriginRole::Call);
        let mut value = SelectedValue::Stopped { step: call };
        for (request, site) in selected {
            run.statuses[request] = Status::Consumed;
            let activated = run.activate(
                self,
                Activation {
                    call,
                    site,
                    deferred,
                    caller,
                    request,
                },
            )?;
            run = activated.run;
            value = activated.value;
        }
        self.selection = Some(run);
        Ok(Some(value))
    }
}
fn remap(chart: &mut FlowChart, origins: &mut Vec<StepOrigin>, occurrences: &mut [FlowOccurrence]) {
    let map = crate::flow::prune::unreachable(chart);
    *origins = std::mem::take(origins)
        .into_iter()
        .enumerate()
        .filter_map(|(old, origin)| map.get(old).map(|_| origin))
        .collect();
    for occurrence in occurrences {
        remap_boundaries(&mut occurrence.boundaries, |id| map.get(id));
    }
}
fn remap_boundaries(refs: &mut BoundaryRefs, map: impl Fn(usize) -> Option<usize>) {
    refs.call = refs.call.and_then(&map);
    refs.entry = refs.entry.and_then(&map);
    refs.marker = refs.marker.and_then(&map);
    refs.returns = refs.returns.iter().filter_map(|id| map(*id)).collect();
    refs.throws = refs.throws.iter().filter_map(|id| map(*id)).collect();
}

impl Builder<'_> {
    pub(super) fn check_size(&self, limit: ExpansionLimit, next: usize) -> Result<()> {
        if let Some(allowance) = self.allowance {
            allowance.check(limit, next as u64)?;
        }
        Ok(())
    }
}
