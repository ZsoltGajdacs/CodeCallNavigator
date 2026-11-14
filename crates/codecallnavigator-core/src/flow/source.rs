//! Exact analyzed-source binding shared by adapters and read-only flow queries.
use super::{
    CallSite, Expression, FunctionFlowIR, SourceSpan, Statement, StatementKind, SwitchCase,
};
use anyhow::{Result, ensure};

pub fn validate_source(function: &FunctionFlowIR, content: &str) -> Result<()> {
    ensure!(
        function.span.start < function.span.end,
        "Function has no meaningful analyzed source range"
    );
    ensure!(
        content
            .get(function.span.start..function.span.end)
            .is_some(),
        "Function range does not match its analyzed UTF-8 source"
    );
    let ranges = SourceRanges {
        function: function.span,
        content,
    };
    let mut pending: Vec<_> = function.body.iter().map(Part::Statement).collect();
    while let Some(part) = pending.pop() {
        match part {
            Part::Statement(statement) => {
                ranges.check(statement.span)?;
                statement_parts(statement, &mut pending);
            }
            Part::Expression(expression) => {
                ranges.check(expression.span())?;
                expression_parts(expression, &mut pending);
            }
            Part::Case(case) => {
                ranges.check(case.span)?;
                pending.extend(case.test.iter().map(Part::Expression));
                pending.extend(case.body.iter().map(Part::Statement));
            }
            Part::Call(call) => ranges.check(call.span)?,
        }
    }
    Ok(())
}

struct SourceRanges<'a> {
    function: SourceSpan,
    content: &'a str,
}
impl SourceRanges<'_> {
    fn check(&self, span: SourceSpan) -> Result<()> {
        if span == SourceSpan::default() {
            return Ok(());
        } // Version-1 unlocated/synthetic sentinel.
        ensure!(
            span.start >= self.function.start && span.end <= self.function.end,
            "Flow range is outside its callable's analyzed source"
        );
        ensure!(
            self.content.get(span.start..span.end).is_some(),
            "Flow range does not match its analyzed UTF-8 source"
        );
        Ok(())
    }
}

enum Part<'a> {
    Statement(&'a Statement),
    Expression(&'a Expression),
    Case(&'a SwitchCase),
    Call(&'a CallSite),
}

fn statement_parts<'a>(statement: &'a Statement, pending: &mut Vec<Part<'a>>) {
    match &statement.kind {
        StatementKind::Action { calls, .. } => pending.extend(calls.iter().map(Part::Call)),
        StatementKind::Evaluate { value } | StatementKind::Throw { value } => {
            pending.push(Part::Expression(value))
        }
        StatementKind::Return { value } | StatementKind::Yield { value } => {
            pending.extend(value.iter().map(Part::Expression))
        }
        StatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            pending.push(Part::Expression(condition));
            pending.extend(then_branch.iter().chain(else_branch).map(Part::Statement));
        }
        StatementKind::Loop {
            condition,
            body,
            update,
            ..
        } => {
            pending.extend(condition.iter().map(Part::Expression));
            pending.extend(body.iter().chain(update).map(Part::Statement));
        }
        StatementKind::Switch { subject, cases, .. } => {
            pending.push(Part::Expression(subject));
            pending.extend(cases.iter().map(Part::Case));
        }
        StatementKind::Try {
            resources,
            body,
            catches,
            finally,
            cleanup,
        } => {
            pending.extend(
                resources
                    .iter()
                    .chain(body)
                    .chain(finally)
                    .chain(cleanup)
                    .map(Part::Statement),
            );
            pending.extend(
                catches
                    .iter()
                    .flat_map(|catch| &catch.body)
                    .map(Part::Statement),
            );
        }
        StatementKind::Block { body, .. } | StatementKind::Region { body, .. } => {
            pending.extend(body.iter().map(Part::Statement))
        }
        _ => {}
    }
}

fn expression_parts<'a>(expression: &'a Expression, pending: &mut Vec<Part<'a>>) {
    match expression {
        Expression::Sequence { items, .. } => pending.extend(items.iter().map(Part::Expression)),
        Expression::Value { calls, regions, .. } => {
            pending.extend(calls.iter().map(Part::Call));
            pending.extend(regions.iter().map(Part::Statement));
        }
        Expression::And { left, right, .. } | Expression::Or { left, right, .. } => {
            pending.push(Part::Expression(left));
            pending.push(Part::Expression(right));
        }
        Expression::Conditional {
            condition,
            when_true,
            when_false,
            ..
        } => {
            pending.push(Part::Expression(condition));
            pending.push(Part::Expression(when_true));
            pending.push(Part::Expression(when_false));
        }
        Expression::Switch { subject, cases, .. } => {
            pending.push(Part::Expression(subject));
            pending.extend(cases.iter().map(Part::Case));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn function(body: serde_json::Value) -> FunctionFlowIR {
        serde_json::from_value(json!({"span":{"start":2,"end":9},"body":body})).unwrap()
    }
    #[test]
    fn malformed_nested_ranges_are_rejected_but_unlocated_and_deferred_spans_remain_valid() {
        let content = "é😀abc";
        let invalid = function(
            json!([{"kind":"evaluate","span":{"start":2,"end":6},"value":{"kind":"value","text":"bad","span":{"start":3,"end":6}}}]),
        );
        assert!(validate_source(&invalid, content).is_err());
        let outside =
            function(json!([{"kind":"action","text":"outside","span":{"start":0,"end":2}}]));
        assert!(validate_source(&outside, content).is_err());
        let synthetic = function(
            json!([{"kind":"region","label":"deferred","deferred":true,"span":{"start":6,"end":7},"body":[{"kind":"action","text":"unlocated"},{"kind":"action","text":"earlier source","span":{"start":2,"end":6}}]}]),
        );
        validate_source(&synthetic, content).unwrap();
    }
    #[test]
    fn publication_source_validation_does_not_apply_the_query_display_cap() {
        let content = "x".repeat(17 * 1024 * 1024);
        let function = FunctionFlowIR {
            span: SourceSpan {
                start: 0,
                end: content.len(),
            },
            ..Default::default()
        };
        validate_source(&function, &content).unwrap();
    }
}
