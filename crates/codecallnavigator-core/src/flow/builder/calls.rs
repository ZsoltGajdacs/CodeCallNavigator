//! Calls registered in assembly order, excluding deferred and already handled steps.
use crate::flow::FlowStep;
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct CallFrame {
    steps: Vec<usize>,
    deferred: HashSet<usize>,
    handled: HashSet<usize>,
    // Only staged callees need this additional negative activation proof.
    references: Option<HashSet<usize>>,
}
impl CallFrame {
    pub fn callee() -> Self {
        Self {
            references: Some(HashSet::new()),
            ..Default::default()
        }
    }
    pub fn register(&mut self, step: &FlowStep) {
        if !step.calls.is_empty() {
            self.steps.push(step.id);
            if let Some(references) = &mut self.references
                && reference_only(step)
            {
                references.insert(step.id);
            }
        }
    }
    pub fn handle(&mut self, step: usize) {
        self.handled.insert(step);
    }
    pub fn defer(&mut self, steps: &[usize]) {
        self.deferred.extend(steps.iter().copied());
    }
    pub fn take_exceptions(&mut self, first: usize) -> Vec<usize> {
        let calls = self.pending(first).collect();
        self.consume(calls)
    }
    pub fn take_function_exceptions(&mut self) -> Vec<usize> {
        let calls = self
            .pending(0)
            .filter(|id| {
                self.references
                    .as_ref()
                    .is_none_or(|references| !references.contains(id))
            })
            .collect();
        self.consume(calls)
    }
    fn pending(&self, first: usize) -> impl Iterator<Item = usize> + '_ {
        self.steps.iter().copied().filter(move |id| {
            *id >= first && !self.deferred.contains(id) && !self.handled.contains(id)
        })
    }
    fn consume(&mut self, calls: Vec<usize>) -> Vec<usize> {
        self.handled.extend(calls.iter().copied());
        calls
    }
}
fn reference_only(step: &FlowStep) -> bool {
    step.calls.iter().all(|site| {
        !site.targets.is_empty()
            && site
                .targets
                .iter()
                .all(|target| target.certainty == "reference")
    })
}
