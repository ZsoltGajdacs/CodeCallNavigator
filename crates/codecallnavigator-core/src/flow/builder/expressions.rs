use super::{Builder, Completion, Exit, StepSpec, branches::SwitchInput};
use crate::flow::*;
use anyhow::Result;

pub(super) struct Condition {
    pub yes: Vec<Exit>,
    pub no: Vec<Exit>,
    pub abrupt: Vec<Exit>,
}
impl Builder {
    pub(super) fn expression(
        &mut self,
        value: &Expression,
        incoming: Vec<Exit>,
    ) -> Result<Vec<Exit>> {
        if incoming.is_empty() {
            return Ok(Vec::new());
        }
        match value {
            Expression::Sequence { items, .. } => self.sequence(items, incoming),
            Expression::Value {
                text,
                span,
                calls,
                regions,
            } => {
                let exits = self.action(
                    StepSpec::new("expression", text, *span).calls(calls.clone()),
                    incoming,
                )?;
                for region in regions {
                    self.statement(region, exits.clone())?;
                }
                Ok(exits)
            }
            Expression::Conditional {
                condition,
                when_true,
                when_false,
                ..
            } => {
                let test = self.condition(condition, incoming)?;
                let mut exits = test.abrupt;
                exits.extend(self.expression(when_true, test.yes)?);
                exits.extend(self.expression(when_false, test.no)?);
                Ok(exits)
            }
            Expression::Switch {
                subject,
                cases,
                span,
            } => self.switch_parts(
                SwitchInput {
                    subject,
                    cases,
                    label: None,
                    span: *span,
                    expression: true,
                },
                incoming,
            ),
            _ => {
                let test = self.condition(value, incoming)?;
                let mut exits = test.abrupt;
                exits.extend(test.yes);
                exits.extend(test.no);
                Ok(exits)
            }
        }
    }
    pub(super) fn condition(
        &mut self,
        value: &Expression,
        incoming: Vec<Exit>,
    ) -> Result<Condition> {
        if incoming.is_empty() {
            return Ok(Condition {
                yes: Vec::new(),
                no: Vec::new(),
                abrupt: Vec::new(),
            });
        }
        match value {
            Expression::Sequence { items, .. } if !items.is_empty() => {
                self.sequence_condition(items, incoming)
            }
            Expression::And { left, right, .. } => {
                let left = self.condition(left, incoming)?;
                let right = self.condition(right, left.yes)?;
                Ok(Condition {
                    yes: right.yes,
                    no: joined(left.no, right.no),
                    abrupt: joined(left.abrupt, right.abrupt),
                })
            }
            Expression::Or { left, right, .. } => {
                let left = self.condition(left, incoming)?;
                let right = self.condition(right, left.no)?;
                Ok(Condition {
                    yes: joined(left.yes, right.yes),
                    no: right.no,
                    abrupt: joined(left.abrupt, right.abrupt),
                })
            }
            Expression::Conditional { .. } => self.conditional_condition(value, incoming),
            Expression::Value { .. } => self.value_condition(value, incoming),
            _ => {
                let exits = self.expression(value, incoming)?;
                let (normal, abrupt): (Vec<_>, Vec<_>) = exits
                    .into_iter()
                    .partition(|exit| exit.completion == Completion::Normal);
                let id = self.step(
                    StepSpec::new("decision", "Expression is true", value.span()),
                    &normal,
                )?;
                Ok(Condition {
                    yes: vec![Exit::normal(id).branch("true", "yes")],
                    no: vec![Exit::normal(id).branch("false", "no")],
                    abrupt,
                })
            }
        }
    }
    fn conditional_condition(
        &mut self,
        value: &Expression,
        incoming: Vec<Exit>,
    ) -> Result<Condition> {
        let Expression::Conditional {
            condition,
            when_true,
            when_false,
            ..
        } = value
        else {
            unreachable!()
        };
        let test = self.condition(condition, incoming)?;
        let yes = self.condition(when_true, test.yes)?;
        let no = self.condition(when_false, test.no)?;
        Ok(Condition {
            yes: joined(yes.yes, no.yes),
            no: joined(yes.no, no.no),
            abrupt: joined(test.abrupt, joined(yes.abrupt, no.abrupt)),
        })
    }
    fn sequence(&mut self, items: &[Expression], mut exits: Vec<Exit>) -> Result<Vec<Exit>> {
        for item in items {
            let (normal, mut abrupt): (Vec<_>, Vec<_>) = exits
                .into_iter()
                .partition(|exit| exit.completion == Completion::Normal);
            if normal.is_empty() {
                return Ok(abrupt);
            }
            abrupt.extend(self.expression(item, normal)?);
            exits = abrupt;
        }
        Ok(exits)
    }
    fn sequence_condition(
        &mut self,
        items: &[Expression],
        incoming: Vec<Exit>,
    ) -> Result<Condition> {
        let evaluated = self.sequence(&items[..items.len() - 1], incoming)?;
        let (normal, abrupt): (Vec<_>, Vec<_>) = evaluated
            .into_iter()
            .partition(|exit| exit.completion == Completion::Normal);
        let mut condition = self.condition(items.last().expect("nonempty sequence"), normal)?;
        condition.abrupt.extend(abrupt);
        Ok(condition)
    }
    fn value_condition(&mut self, value: &Expression, incoming: Vec<Exit>) -> Result<Condition> {
        let Expression::Value {
            text,
            span,
            calls,
            regions,
        } = value
        else {
            unreachable!()
        };

        let id = self.step(
            StepSpec::new("decision", text, *span).calls(calls.clone()),
            &incoming,
        )?;
        for region in regions {
            self.statement(region, vec![Exit::normal(id)])?;
        }
        Ok(Condition {
            yes: (text.trim() != "false")
                .then(|| Exit::normal(id).branch("true", "yes"))
                .into_iter()
                .collect(),
            no: (text.trim() != "true")
                .then(|| Exit::normal(id).branch("false", "no"))
                .into_iter()
                .collect(),
            abrupt: Vec::new(),
        })
    }
    pub(super) fn region(
        &mut self,
        statement: &Statement,
        incoming: Vec<Exit>,
    ) -> Result<Vec<Exit>> {
        let StatementKind::Region {
            label,
            deferred,
            body,
        } = &statement.kind
        else {
            unreachable!()
        };
        let span = statement.span;
        if !*deferred {
            return self.block(body, incoming);
        }
        let first = self.chart.steps.len();
        let entry = self.step(StepSpec::new("deferred", label, span), &[])?;
        let exits = self.block(body, vec![Exit::normal(entry)])?;
        self.finish(exits, span)?;
        let steps: Vec<_> = (first..self.chart.steps.len()).collect();
        self.deferred.extend(steps.iter().copied());
        self.chart.regions.push(FlowRegion {
            label: label.into(),
            deferred: *deferred,
            entry,
            steps,
        });
        Ok(incoming)
    }
}
fn joined(mut left: Vec<Exit>, right: Vec<Exit>) -> Vec<Exit> {
    left.extend(right);
    left
}
