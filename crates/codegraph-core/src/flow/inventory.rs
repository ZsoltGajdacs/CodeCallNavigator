//! Inventory composition and binding cleanup never modify call graph relationships.
use super::*;
use anyhow::{Result, ensure};
use std::collections::{HashMap, HashSet};

pub fn merge_inventory(
    target: &mut Option<FlowInventory>,
    part: Option<FlowInventory>,
) -> Result<()> {
    let Some(part) = part else {
        return Ok(());
    };
    ensure!(
        part.version == INVENTORY_VERSION,
        "Unsupported function flow inventory version"
    );
    let Some(target) = target else {
        *target = Some(part);
        return Ok(());
    };
    ensure!(
        target.version == part.version,
        "Conflicting function flow inventory versions"
    );
    let mut known: HashMap<_, _> = target
        .functions
        .iter()
        .enumerate()
        .map(|(index, function)| (function.symbol.clone(), index))
        .collect();
    for function in part.functions {
        if let Some(&index) = known.get(&function.symbol) {
            ensure!(
                target.functions[index] == function,
                "Conflicting function flow identity: {}",
                function.symbol
            );
        } else {
            known.insert(function.symbol.clone(), target.functions.len());
            target.functions.push(function);
        }
    }
    Ok(())
}
impl FlowInventory {
    pub fn retain_symbols(&mut self, symbols: &HashSet<&str>) {
        self.functions
            .retain(|function| symbols.contains(function.symbol.as_str()));
        for function in &mut self.functions {
            visit_calls_mut(function, &mut |call| {
                let previous = call.targets.len();
                call.targets
                    .retain(|target| symbols.contains(target.symbol.as_str()));
                if previous != call.targets.len() {
                    call.diagnostic =
                        Some("Some call targets are excluded from the indexed graph".into());
                }
            });
        }
    }
}

pub fn visit_calls_mut(function: &mut FunctionFlowIR, visitor: &mut impl FnMut(&mut CallSite)) {
    statements(&mut function.body, visitor);
}
fn statements(body: &mut [Statement], visitor: &mut impl FnMut(&mut CallSite)) {
    for statement in body {
        statement_calls(statement, visitor);
    }
}
fn statement_calls(statement: &mut Statement, visitor: &mut impl FnMut(&mut CallSite)) {
    match &mut statement.kind {
        StatementKind::Action { calls, .. } => calls.iter_mut().for_each(visitor),
        StatementKind::Evaluate { value } | StatementKind::Throw { value } => {
            expression(value, visitor)
        }
        StatementKind::Return { value } | StatementKind::Yield { value } => {
            if let Some(value) = value {
                expression(value, visitor);
            }
        }
        StatementKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expression(condition, visitor);
            statements(then_branch, visitor);
            statements(else_branch, visitor);
        }
        StatementKind::Loop {
            condition,
            body,
            update,
            ..
        } => {
            if let Some(condition) = condition {
                expression(condition, visitor);
            }
            statements(body, visitor);
            statements(update, visitor);
        }
        StatementKind::Switch { subject, cases, .. } => {
            expression(subject, visitor);
            case_calls(cases, visitor);
        }
        StatementKind::Try {
            resources,
            body,
            catches,
            finally,
            cleanup,
        } => {
            statements(resources, visitor);
            statements(body, visitor);
            for catch in catches {
                statements(&mut catch.body, visitor);
            }
            statements(finally, visitor);
            statements(cleanup, visitor);
        }
        StatementKind::Block { body, .. } | StatementKind::Region { body, .. } => {
            statements(body, visitor)
        }
        _ => {}
    }
}
fn case_calls(cases: &mut [SwitchCase], visitor: &mut impl FnMut(&mut CallSite)) {
    for case in cases {
        if let Some(test) = &mut case.test {
            expression(test, visitor);
        }
        statements(&mut case.body, visitor);
    }
}
fn expression(value: &mut Expression, visitor: &mut impl FnMut(&mut CallSite)) {
    match value {
        Expression::Sequence { items, .. } => {
            for item in items {
                expression(item, visitor);
            }
        }
        Expression::Value { calls, regions, .. } => {
            calls.iter_mut().for_each(&mut *visitor);
            statements(regions, visitor);
        }
        Expression::And { left, right, .. } | Expression::Or { left, right, .. } => {
            expression(left, visitor);
            expression(right, visitor);
        }
        Expression::Conditional {
            condition,
            when_true,
            when_false,
            ..
        } => {
            expression(condition, visitor);
            expression(when_true, visitor);
            expression(when_false, visitor);
        }
        Expression::Switch { subject, cases, .. } => {
            expression(subject, visitor);
            case_calls(cases, visitor);
        }
    }
}
