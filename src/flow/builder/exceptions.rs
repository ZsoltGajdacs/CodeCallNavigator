use super::{Builder, Completion, Exit, StepSpec};
use crate::flow::*;
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

struct ResourceScope<'a> {
    resources: &'a [Statement],
    cleanup: &'a [Statement],
    body: &'a [Statement],
}
impl Builder {
    pub(super) fn try_statement(
        &mut self,
        statement: &Statement,
        incoming: Vec<Exit>,
    ) -> Result<Vec<Exit>> {
        let StatementKind::Try {
            resources,
            body,
            catches,
            finally,
            cleanup,
        } = &statement.kind
        else {
            unreachable!()
        };
        let exits = self.protected(
            ResourceScope {
                resources,
                cleanup,
                body,
            },
            incoming,
        )?;
        let exits = self.catches(exits, catches, statement.span)?;
        self.cleanup(exits, finally)
    }
    fn protected(&mut self, scope: ResourceScope<'_>, incoming: Vec<Exit>) -> Result<Vec<Exit>> {
        let ResourceScope {
            resources,
            cleanup,
            body,
        } = scope;
        let Some((resource, remaining)) = resources.split_first() else {
            let first = self.chart.steps.len();
            let mut exits = self.block(body, incoming)?;
            self.possible_call_exceptions(first, &mut exits);
            return self.cleanup(exits, cleanup);
        };
        ensure!(
            resources.len() == cleanup.len(),
            "Resource acquisitions and reverse-order cleanup metadata must pair exactly"
        );
        let (close, remaining_cleanup) = cleanup.split_last().expect("paired cleanup");
        let first = self.chart.steps.len();
        let mut acquired = self.block(std::slice::from_ref(resource), incoming)?;
        self.possible_call_exceptions(first, &mut acquired);
        let (normal, mut failed): (Vec<_>, Vec<_>) = acquired
            .into_iter()
            .partition(|exit| exit.completion == Completion::Normal);
        let exits = self.protected(
            ResourceScope {
                resources: remaining,
                cleanup: remaining_cleanup,
                body,
            },
            normal,
        )?;
        failed.extend(self.cleanup(exits, std::slice::from_ref(close))?);
        Ok(failed)
    }
    fn possible_call_exceptions(&mut self, first: usize, exits: &mut Vec<Exit>) {
        let calls: Vec<_> = self.chart.steps[first..]
            .iter()
            .filter(|step| {
                !step.calls.is_empty()
                    && !self.deferred.contains(&step.id)
                    && !self.handled_calls.contains(&step.id)
            })
            .map(|step| step.id)
            .collect();
        if calls.is_empty() {
            return;
        }
        self.handled_calls.extend(calls.iter().copied());
        self.diagnostic("Calls inside protected regions may throw; exceptional paths are conservative, not runtime predictions");
        exits.extend(calls.into_iter().map(|id| {
            Exit::with_completion(id, Completion::Throw).branch("may throw", "exception")
        }));
    }
    fn catches(
        &mut self,
        exits: Vec<Exit>,
        catches: &[CatchClause],
        span: SourceSpan,
    ) -> Result<Vec<Exit>> {
        if catches.is_empty() {
            return Ok(exits);
        }
        let (thrown, mut remaining): (Vec<_>, Vec<_>) = exits
            .into_iter()
            .partition(|exit| exit.completion == Completion::Throw);
        if thrown.is_empty() {
            return Ok(remaining);
        }
        if !catches.first().is_some_and(|catch| catch.catch_all) {
            self.diagnostic("Catch type dispatch is unresolved; candidate handlers and uncaught exceptions are shown separately");
        }
        for catch in catches {
            let incoming: Vec<_> = thrown
                .iter()
                .cloned()
                .map(|exit| exit.resume().branch("possible catch", "exception"))
                .collect();
            let entry = self.step(
                StepSpec::new("catch", format!("catch {}", catch.parameter), span),
                &incoming,
            )?;
            let first = self.chart.steps.len();
            let mut caught = self.block(&catch.body, vec![Exit::normal(entry)])?;
            self.possible_call_exceptions(first, &mut caught);
            remaining.extend(caught);
            if catch.catch_all {
                return Ok(remaining);
            }
        }
        remaining.extend(thrown);
        Ok(remaining)
    }
    fn cleanup(&mut self, exits: Vec<Exit>, statements: &[Statement]) -> Result<Vec<Exit>> {
        if statements.is_empty() {
            return Ok(exits);
        }
        let mut groups = BTreeMap::<Completion, Vec<Exit>>::new();
        for exit in exits {
            groups
                .entry(exit.completion.clone())
                .or_default()
                .push(exit.resume());
        }
        let mut results = Vec::new();
        for (pending, incoming) in groups {
            let first = self.chart.steps.len();
            let mut exits = self.block(statements, incoming)?;
            self.possible_call_exceptions(first, &mut exits);
            for mut exit in exits {
                if exit.completion == Completion::Normal {
                    exit.completion = pending.clone();
                }
                results.push(exit);
            }
        }
        Ok(results)
    }
}
