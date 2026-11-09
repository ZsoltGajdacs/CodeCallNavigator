use super::{Builder, Completion, Exit, StepSpec};
use crate::flow::*;
use anyhow::{Result, ensure};

pub(super) struct SwitchInput<'a> {
    pub subject: &'a Expression,
    pub cases: &'a [SwitchCase],
    pub label: Option<&'a str>,
    pub span: SourceSpan,
    pub expression: bool,
}
struct Dispatch {
    cases: Vec<Vec<Exit>>,
    unmatched: Vec<Exit>,
    abrupt: Vec<Exit>,
}
impl Builder {
    pub(super) fn branch(
        &mut self,
        statement: &Statement,
        incoming: Vec<Exit>,
    ) -> Result<Vec<Exit>> {
        let StatementKind::If {
            condition,
            then_branch,
            else_branch,
        } = &statement.kind
        else {
            unreachable!()
        };
        let test = self.condition(condition, incoming)?;
        let mut exits = test.abrupt;
        exits.extend(self.block(then_branch, test.yes)?);
        exits.extend(self.block(else_branch, test.no)?);
        Ok(exits)
    }
    pub(super) fn switch(
        &mut self,
        statement: &Statement,
        incoming: Vec<Exit>,
    ) -> Result<Vec<Exit>> {
        let StatementKind::Switch {
            subject,
            cases,
            label,
        } = &statement.kind
        else {
            unreachable!()
        };
        self.switch_parts(
            SwitchInput {
                subject,
                cases,
                label: label.as_deref(),
                span: statement.span,
                expression: false,
            },
            incoming,
        )
    }
    pub(super) fn switch_parts(
        &mut self,
        input: SwitchInput<'_>,
        incoming: Vec<Exit>,
    ) -> Result<Vec<Exit>> {
        let evaluated = self.expression(input.subject, incoming)?;
        let (normal, mut exits): (Vec<_>, Vec<_>) = evaluated
            .into_iter()
            .partition(|exit| exit.completion == Completion::Normal);
        let selector = self.step(
            StepSpec::new(
                "switch",
                format!("switch ({})", input.subject.text()),
                input.span,
            ),
            &normal,
        )?;
        let dispatch = self.switch_dispatch(&input, vec![Exit::normal(selector)])?;
        exits.extend(dispatch.abrupt);
        exits.extend(dispatch.unmatched);
        let mut fallthrough = Vec::new();
        for (index, case) in input.cases.iter().enumerate() {
            fallthrough.extend(dispatch.cases[index].iter().cloned());
            if fallthrough.is_empty() {
                continue;
            }
            let span = case_span(case, input.span);
            let entry = self.step(StepSpec::new("case", case_label(case), span), &fallthrough)?;
            let results = self.block(&case.body, vec![Exit::normal(entry)])?;
            (fallthrough, exits) = case_exits(results, exits, case, &input);
        }
        exits.extend(fallthrough);
        Ok(exits)
    }
    fn switch_dispatch(
        &mut self,
        input: &SwitchInput<'_>,
        mut remaining: Vec<Exit>,
    ) -> Result<Dispatch> {
        let mut dispatch = Dispatch {
            cases: vec![Vec::new(); input.cases.len()],
            unmatched: Vec::new(),
            abrupt: Vec::new(),
        };
        let defaults: Vec<_> = input
            .cases
            .iter()
            .enumerate()
            .filter_map(|(index, case)| case.is_default.then_some(index))
            .collect();
        ensure!(
            defaults.len() <= 1,
            "Multiple default arms in switch flow metadata"
        );
        for (index, case) in input
            .cases
            .iter()
            .enumerate()
            .filter(|(_, case)| !case.is_default)
        {
            let evaluated = match &case.test {
                Some(test) => self.expression(test, remaining)?,
                None => remaining,
            };
            let (normal, abrupt): (Vec<_>, Vec<_>) = evaluated
                .into_iter()
                .partition(|exit| exit.completion == Completion::Normal);
            dispatch.abrupt.extend(abrupt);
            if normal.is_empty() {
                remaining = Vec::new();
                break;
            }
            let comparison = self.step(
                StepSpec::new(
                    "decision",
                    format!("Matches {}", case_label(case)),
                    case_span(case, input.span),
                ),
                &normal,
            )?;
            dispatch.cases[index].push(Exit::normal(comparison).branch("match", "yes"));
            remaining = vec![Exit::normal(comparison).branch("no match", "no")];
        }
        assign_unmatched(
            &mut dispatch,
            remaining,
            defaults.first().copied(),
            input.expression,
        );
        Ok(dispatch)
    }
}
fn assign_unmatched(
    dispatch: &mut Dispatch,
    mut remaining: Vec<Exit>,
    default: Option<usize>,
    expression: bool,
) {
    if let Some(default) = default {
        dispatch.cases[default].extend(remaining);
        return;
    }
    if expression {
        for exit in &mut remaining {
            exit.completion = Completion::Throw;
            exit.label = "no matching arm".into();
            exit.kind = "exception".into();
        }
    }
    dispatch.unmatched = remaining;
}
fn case_label(case: &SwitchCase) -> String {
    if case.is_default {
        "default".into()
    } else {
        format!("case {}", case.labels.join(", "))
    }
}
fn case_span(case: &SwitchCase, fallback: SourceSpan) -> SourceSpan {
    if case.span.start < case.span.end {
        case.span
    } else {
        fallback
    }
}
fn case_exits(
    results: Vec<Exit>,
    mut exits: Vec<Exit>,
    case: &SwitchCase,
    input: &SwitchInput<'_>,
) -> (Vec<Exit>, Vec<Exit>) {
    let mut next = Vec::new();
    for mut exit in results {
        match &exit.completion {
            Completion::Normal if case.falls_through => next.push(exit),
            Completion::Yield if input.expression => {
                exit.completion = Completion::Normal;
                exits.push(exit);
            }
            Completion::Break(target) if target.is_none() || target.as_deref() == input.label => {
                exit.completion = Completion::Normal;
                exits.push(exit);
            }
            _ => exits.push(exit),
        }
    }
    (next, exits)
}
