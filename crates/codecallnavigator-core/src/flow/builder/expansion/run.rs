//! Move one run state through staged builders; rollback contains only small metadata and borrows.
use super::super::Completion;
use super::*;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Status {
    Pending,
    Consumed,
    Blocked(OccurrenceId),
}
struct Checkpoint {
    statuses: Vec<Status>,
    sources: usize,
    occurrences: usize,
}
impl<'a> Selection<'a> {
    pub(super) fn request(&self, caller: OccurrenceId, address: &SiteAddress) -> Option<usize> {
        let path = &self.occurrences[caller.0].path;
        self.prepared
            .requests
            .iter()
            .position(|request| request.caller == *path && request.choice.site == *address)
    }
    pub(super) fn activate(
        mut self,
        builder: &mut Builder<'a>,
        activation: Activation<'_>,
    ) -> Result<Activated<'a>> {
        if let Some(stop) = self.budget_stop(&activation) {
            return self.stopped(builder, activation, stop);
        }
        let choice = self.prepared.requests[activation.request].choice.clone();
        let input = self.prepared.input(&choice.target.symbol);
        let stop = eligibility::activation(activation.site, &choice.target, activation.deferred)
            .or_else(|| self.cycle(activation.caller, &choice.target))
            .or_else(|| eligibility::body(input));
        if let Some(stop) = stop {
            return self.stopped(builder, activation, stop);
        }
        let input = input.expect("eligible callee input");
        let index = self.prepared.index(&choice.target.symbol)?;
        let checkpoint = Checkpoint {
            statuses: self.statuses.clone(),
            sources: self.sources.len(),
            occurrences: self.occurrences.len(),
        };
        let source = self.sources.intern(
            input,
            budget::Allowance::child(self.limits, activation.request),
        )?;
        let id = OccurrenceId(self.occurrences.len());
        self.occurrences
            .push(self.occurrence(&activation, Some((input, source))));
        let staged = child::stage(
            child::Input {
                function: input.function,
                index,
                source,
                id,
                request: activation.request,
            },
            self,
            builder.cancel,
        )
        .map_err(|error| budget::attribute(error, activation.request))?;
        if staged.exits.iter().any(|exit| {
            !matches!(
                exit.completion,
                Completion::Normal | Completion::Return | Completion::Throw
            )
        }) {
            let mut run = staged.into_run();
            run.rollback(checkpoint);
            return run.stopped(
                builder,
                activation,
                eligibility::Stop::new(
                    ExpansionStop::Ineligible,
                    "Callee has an unresolved abrupt transfer",
                ),
            );
        }
        builder.calls.handle(activation.call);
        child::graft(builder, staged, activation.call)
            .map_err(|error| budget::attribute(error, activation.request))
    }
    fn budget_stop(&mut self, activation: &Activation<'_>) -> Option<eligibility::Stop> {
        self.evaluated += 1;
        let limit = match &self.mode {
            attempt::Mode::RootOnly => {
                return Some(eligibility::Stop::new(
                    ExpansionStop::Unselected,
                    "Mandatory root preflight",
                ));
            }
            attempt::Mode::Expand(demotions) => demotions[activation.request],
        }
        .or_else(|| {
            if self.occurrences[activation.caller.0].path.0.len() >= self.limits.depth {
                Some(ExpansionLimit::Depth)
            } else if self.evaluated > self.limits.requests {
                Some(ExpansionLimit::Requests)
            } else {
                None
            }
        });
        limit.map(|limit| {
            eligibility::Stop::new(
                ExpansionStop::Limit { limit },
                "Selected expansion exceeds its allowance",
            )
        })
    }
    fn cycle(&self, caller: OccurrenceId, target: &CallTarget) -> Option<eligibility::Stop> {
        let path = &self.occurrences[caller.0].path;
        if target.symbol == self.root
            || path
                .0
                .iter()
                .any(|choice| choice.target.symbol == target.symbol)
        {
            Some(eligibility::Stop::new(
                ExpansionStop::Cycle,
                "Recursive target is not expanded",
            ))
        } else {
            None
        }
    }
    fn occurrence(
        &self,
        activation: &Activation<'_>,
        input: Option<(FunctionSource<'a>, SourceId)>,
    ) -> FlowOccurrence {
        let choice = self.prepared.requests[activation.request].choice.clone();
        let mut path = self.occurrences[activation.caller.0].path.clone();
        path.0.push(choice.clone());
        FlowOccurrence {
            id: OccurrenceId(self.occurrences.len()),
            symbol: choice.target.symbol,
            parent: Some(activation.caller),
            path,
            source: input.map(|(_, source)| source),
            owner: input.map(|(input, _)| owner(input.function)),
            span: input
                .map(|(input, _)| input.function.span)
                .unwrap_or_default(),
            outcome: OccurrenceOutcome::Expanded,
            boundaries: BoundaryRefs::default(),
        }
    }
    fn rollback(&mut self, checkpoint: Checkpoint) {
        self.statuses = checkpoint.statuses;
        self.sources.truncate(checkpoint.sources);
        self.occurrences.truncate(checkpoint.occurrences);
    }
    fn stopped(
        mut self,
        builder: &mut Builder<'_>,
        activation: Activation<'_>,
        stop: eligibility::Stop,
    ) -> Result<Activated<'a>> {
        let mut occurrence = self.occurrence(&activation, None);
        let id = occurrence.id;
        let count = self.block_descendants(&occurrence.path, id);
        let message = if count == 0 {
            stop.message.into()
        } else {
            format!("{}; {count} selected descendants blocked", stop.message)
        };
        let marker = activation.call;
        builder
            .recording
            .as_mut()
            .unwrap()
            .role(marker, OriginRole::Marker);
        if count > 0 {
            builder.diagnostic(&format!(
                "{count} selected descendants blocked by stopped occurrence {}",
                id.0
            ));
        }
        occurrence.outcome = OccurrenceOutcome::Stopped {
            reason: stop.reason,
            message,
        };
        occurrence.boundaries = BoundaryRefs {
            marker: Some(marker),
            ..Default::default()
        };
        self.occurrences.push(occurrence);
        Ok(Activated {
            run: self,
            value: SelectedValue::Stopped { step: marker },
        })
    }
    fn block_descendants(&mut self, path: &OccurrencePath, ancestor: OccurrenceId) -> usize {
        let mut count = 0;
        for (request, state) in self.prepared.requests.iter().zip(&mut self.statuses) {
            if *state == Status::Pending && request.caller.0.starts_with(&path.0) {
                *state = Status::Blocked(ancestor);
                count += 1;
            }
        }
        count
    }
    pub(super) fn blocked(&self) -> Vec<BlockedSelection> {
        self.prepared
            .requests
            .iter()
            .zip(&self.statuses)
            .filter_map(|(request, state)| match state {
                Status::Blocked(ancestor) => Some(BlockedSelection {
                    caller: request.caller.clone(),
                    choice: request.choice.clone(),
                    ancestor: *ancestor,
                }),
                _ => None,
            })
            .collect()
    }
}
