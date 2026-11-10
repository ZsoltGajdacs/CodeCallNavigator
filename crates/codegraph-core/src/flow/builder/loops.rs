use super::{Builder, Completion, Exit, StepSpec};
use crate::flow::*;
use anyhow::Result;

impl Builder {
    pub(super) fn loop_statement(
        &mut self,
        statement: &Statement,
        incoming: Vec<Exit>,
    ) -> Result<Vec<Exit>> {
        let StatementKind::Loop {
            label,
            condition,
            body,
            update,
            test_first,
        } = &statement.kind
        else {
            unreachable!()
        };
        let span = statement.span;
        let entry = self.step(StepSpec::new("loop", "Loop", span), &incoming)?;
        let body_entry = self.step(StepSpec::new("loop_body", "Loop body", span), &[])?;
        let condition_entry =
            self.step(StepSpec::new("loop_condition", "Loop condition", span), &[])?;
        let update_entry = self.step(StepSpec::new("loop_update", "Next iteration", span), &[])?;
        self.link(
            &Exit::normal(entry),
            if *test_first {
                condition_entry
            } else {
                body_entry
            },
        )?;
        let (yes, mut exits) = self.loop_condition(condition.as_ref(), condition_entry)?;
        for exit in yes {
            self.link(&exit, body_entry)?;
        }
        let results = self.block(body, vec![Exit::normal(body_entry)])?;
        exits.extend(self.route_loop(results, label, update_entry)?);
        let updated = self.block(update, vec![Exit::normal(update_entry)])?;
        exits.extend(self.repeat_loop(updated, condition_entry)?);
        Ok(exits)
    }
    fn route_loop(
        &mut self,
        results: Vec<Exit>,
        label: &Option<String>,
        update_entry: usize,
    ) -> Result<Vec<Exit>> {
        let mut remaining = Vec::new();
        for exit in results {
            match &exit.completion {
                Completion::Normal => self.link(&exit, update_entry)?,
                Completion::Continue(target) if target.is_none() || target == label => {
                    self.link(&exit.resume(), update_entry)?
                }
                Completion::Break(target) if target.is_none() || target == label => {
                    remaining.push(exit.resume())
                }
                _ => remaining.push(exit),
            }
        }
        Ok(remaining)
    }
    fn repeat_loop(&mut self, updated: Vec<Exit>, condition_entry: usize) -> Result<Vec<Exit>> {
        let mut exits = Vec::new();
        for exit in updated {
            if exit.completion == Completion::Normal {
                self.link(&exit.branch("repeat", "back"), condition_entry)?;
            } else {
                exits.push(exit);
            }
        }
        Ok(exits)
    }
    fn loop_condition(
        &mut self,
        condition: Option<&Expression>,
        entry: usize,
    ) -> Result<(Vec<Exit>, Vec<Exit>)> {
        match condition {
            Some(condition) => {
                let test = self.condition(condition, vec![Exit::normal(entry)])?;
                let mut exits = test.abrupt;
                exits.extend(test.no);
                Ok((test.yes, exits))
            }
            None => Ok((vec![Exit::normal(entry)], Vec::new())),
        }
    }
}
