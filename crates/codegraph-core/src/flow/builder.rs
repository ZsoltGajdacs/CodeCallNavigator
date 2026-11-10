//! Completion-aware flow assembly. Abrupt exits survive cleanup until overridden.
mod branches;
mod exceptions;
mod expressions;
mod loops;
use super::*;
use anyhow::{Result, ensure};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Completion {
    Normal,
    Return,
    Throw,
    Break(Option<String>),
    Continue(Option<String>),
    Yield,
}
#[derive(Clone)]
struct Exit {
    from: usize,
    completion: Completion,
    label: String,
    kind: String,
}
impl Exit {
    fn normal(from: usize) -> Self {
        Self {
            from,
            completion: Completion::Normal,
            label: String::new(),
            kind: "normal".into(),
        }
    }
    fn with_completion(from: usize, completion: Completion) -> Self {
        Self {
            completion,
            ..Self::normal(from)
        }
    }
    fn resume(mut self) -> Self {
        self.completion = Completion::Normal;
        self
    }
    fn branch(mut self, label: &str, kind: &str) -> Self {
        self.label = label.into();
        self.kind = kind.into();
        self
    }
}
struct StepSpec {
    kind: String,
    label: String,
    span: SourceSpan,
    calls: Vec<CallSite>,
}
impl StepSpec {
    fn new(kind: impl Into<String>, label: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            kind: kind.into(),
            label: label.into(),
            span,
            calls: Vec::new(),
        }
    }
    fn calls(mut self, calls: Vec<CallSite>) -> Self {
        self.calls = calls;
        self
    }
}
struct Builder {
    chart: FlowChart,
    deferred: HashSet<usize>,
    handled_calls: HashSet<usize>,
}

pub fn build(function: &FunctionFlowIR) -> Result<FlowChart> {
    ensure!(
        function.span.start <= function.span.end,
        "Invalid function source span"
    );
    let mut builder = Builder {
        chart: FlowChart {
            symbol: function.symbol.clone(),
            file: function.file.clone(),
            span: function.span,
            diagnostics: function.diagnostics.clone(),
            partial: !function.diagnostics.is_empty(),
            ..Default::default()
        },
        deferred: HashSet::new(),
        handled_calls: HashSet::new(),
    };
    let entry = builder.step(StepSpec::new("start", "Function entry", function.span), &[])?;
    builder.chart.entry = entry;
    let exits = builder.block(&function.body, vec![Exit::normal(entry)])?;
    builder.finish(exits, function.span)?;
    super::prune::unreachable(&mut builder.chart);
    Ok(builder.chart)
}

impl Builder {
    fn step(&mut self, spec: StepSpec, incoming: &[Exit]) -> Result<usize> {
        let StepSpec {
            kind,
            label,
            span,
            calls,
        } = spec;
        ensure!(
            self.chart.steps.len() < MAX_STEPS,
            "Function flow exceeds {MAX_STEPS} steps; choose a smaller function"
        );
        ensure!(span.start <= span.end, "Invalid flow source span");
        let id = self.chart.steps.len();
        self.chart.steps.push(FlowStep {
            id,
            kind,
            label: compact(&label),
            span,
            calls,
            ..Default::default()
        });
        for exit in incoming {
            self.link(exit, id)?;
        }
        Ok(id)
    }
    fn link(&mut self, exit: &Exit, to: usize) -> Result<()> {
        ensure!(
            self.chart.links.len() < MAX_LINKS,
            "Function flow exceeds {MAX_LINKS} links; choose a smaller function"
        );
        self.chart.links.push(FlowLink {
            from: exit.from,
            to,
            label: exit.label.clone(),
            kind: exit.kind.clone(),
            ..Default::default()
        });
        Ok(())
    }
    fn block(&mut self, statements: &[Statement], mut exits: Vec<Exit>) -> Result<Vec<Exit>> {
        for statement in statements {
            let (normal, mut abrupt): (Vec<_>, Vec<_>) = exits
                .into_iter()
                .partition(|exit| exit.completion == Completion::Normal);
            if normal.is_empty() {
                return Ok(abrupt);
            }
            abrupt.extend(self.statement(statement, normal)?);
            exits = abrupt;
        }
        Ok(exits)
    }
    fn statement(&mut self, statement: &Statement, incoming: Vec<Exit>) -> Result<Vec<Exit>> {
        let span = statement.span;
        match &statement.kind {
            StatementKind::Action { text, calls } => self.action(
                StepSpec::new("action", text, span).calls(calls.clone()),
                incoming,
            ),
            StatementKind::Evaluate { value } => self.expression(value, incoming),
            StatementKind::If { .. } => self.branch(statement, incoming),
            StatementKind::Loop { .. } => self.loop_statement(statement, incoming),
            StatementKind::Switch { .. } => self.switch(statement, incoming),
            StatementKind::Try { .. } => self.try_statement(statement, incoming),
            StatementKind::Return { .. }
            | StatementKind::Throw { .. }
            | StatementKind::Yield { .. } => self.complete(statement, incoming),
            StatementKind::Break { label } => self.transfer(
                StepSpec::new("break", jump_text("break", label), span),
                Completion::Break(label.clone()),
                incoming,
            ),
            StatementKind::Continue { label } => self.transfer(
                StepSpec::new("continue", jump_text("continue", label), span),
                Completion::Continue(label.clone()),
                incoming,
            ),
            StatementKind::Block { label, body } => {
                let mut exits = self.block(body, incoming)?;
                if label.is_some() {
                    Self::consume_breaks(&mut exits, label);
                }
                Ok(exits)
            }
            StatementKind::Region { .. } => self.region(statement, incoming),
            StatementKind::Unsupported { text, reason } => {
                self.diagnostic(reason);
                let exits = self.action(StepSpec::new("unsupported", text, span), incoming)?;
                Ok(exits
                    .into_iter()
                    .map(|exit| exit.branch("flow not expanded", "unknown"))
                    .collect())
            }
        }
    }
    fn action(&mut self, spec: StepSpec, incoming: Vec<Exit>) -> Result<Vec<Exit>> {
        let id = self.step(spec, &incoming)?;
        Ok(vec![Exit::normal(id)])
    }
    fn transfer(
        &mut self,
        spec: StepSpec,
        completion: Completion,
        incoming: Vec<Exit>,
    ) -> Result<Vec<Exit>> {
        let id = self.step(spec, &incoming)?;
        Ok(vec![Exit::with_completion(id, completion)])
    }
    fn complete(&mut self, statement: &Statement, incoming: Vec<Exit>) -> Result<Vec<Exit>> {
        let (kind, value, completion) = match &statement.kind {
            StatementKind::Return { value } => ("return", value.as_ref(), Completion::Return),
            StatementKind::Throw { value } => ("throw", Some(value), Completion::Throw),
            StatementKind::Yield { value } => ("yield", value.as_ref(), Completion::Yield),
            _ => unreachable!(),
        };
        let incoming = match value {
            Some(value) => self.expression(value, incoming)?,
            None => incoming,
        };
        let (normal, mut abrupt): (Vec<_>, Vec<_>) = incoming
            .into_iter()
            .partition(|exit| exit.completion == Completion::Normal);
        if !normal.is_empty() {
            let id = self.step(StepSpec::new(kind, kind, statement.span), &normal)?;
            abrupt.push(Exit::with_completion(id, completion));
        }
        Ok(abrupt)
    }
    fn finish(&mut self, exits: Vec<Exit>, span: SourceSpan) -> Result<()> {
        let mut groups = std::collections::BTreeMap::<Completion, Vec<Exit>>::new();
        for exit in exits {
            groups
                .entry(exit.completion.clone())
                .or_default()
                .push(exit);
        }
        for (completion, exits) in groups {
            let label = match completion {
                Completion::Normal => "Function exit",
                Completion::Return => "Return to caller",
                Completion::Throw => "Exception escapes",
                _ => {
                    self.diagnostic("Unresolved jump target is shown as an incomplete exit");
                    "Unresolved jump"
                }
            };
            self.step(StepSpec::new("exit", label, span), &exits)?;
        }
        Ok(())
    }
    fn diagnostic(&mut self, message: &str) {
        self.chart.partial = true;
        if !self.chart.diagnostics.iter().any(|value| value == message) {
            self.chart.diagnostics.push(message.into());
        }
    }
    fn consume_breaks(exits: &mut [Exit], label: &Option<String>) {
        for exit in exits {
            if matches!(&exit.completion,Completion::Break(target) if target == label) {
                exit.completion = Completion::Normal;
            }
        }
    }
}
fn compact(text: &str) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut result = text.chars().take(160).collect::<String>();
    if text.chars().count() > 160 {
        result.push('…');
    }
    result
}

fn jump_text(kind: &str, label: &Option<String>) -> String {
    label
        .as_ref()
        .map(|label| format!("{kind} {label}"))
        .unwrap_or_else(|| kind.into())
}
