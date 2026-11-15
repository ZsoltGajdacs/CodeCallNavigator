//! Completion-aware flow assembly. Abrupt exits survive cleanup until overridden.
mod branches;
mod calls;
mod completion;
mod exceptions;
mod expansion;
mod expressions;
mod function;
mod recording;
pub use expansion::expand;
mod loops;
use super::*;
use anyhow::{Result, ensure};
use calls::CallFrame;
use completion::{Completion, Exit};

#[derive(Clone)]
struct StepSpec<'a> {
    kind: String,
    label: String,
    span: SourceSpan,
    calls: &'a [CallSite],
}
impl<'a> StepSpec<'a> {
    fn new(kind: impl Into<String>, label: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            kind: kind.into(),
            label: label.into(),
            span,
            calls: &[],
        }
    }
    fn calls(mut self, calls: &'a [CallSite]) -> Self {
        self.calls = calls;
        self
    }
}
struct Builder<'a> {
    chart: FlowChart,
    calls: CallFrame,
    recording: Option<recording::Recording>,
    selection: Option<expansion::Selection<'a>>,
    cancel: Option<&'a std::sync::atomic::AtomicBool>,
    allowance: Option<expansion::Allowance>,
}

pub fn build(function: &FunctionFlowIR) -> Result<FlowChart> {
    ensure!(
        function.span.start <= function.span.end,
        "Invalid function source span"
    );
    super::validate_metadata(function)?;
    let mut builder = Builder::new(function);
    let assembled = builder.function(
        StepSpec::new("start", "Function entry", function.span),
        &function.body,
    )?;
    builder.chart.entry = assembled.entry;
    builder.finish(assembled.exits, function.span)?;
    super::prune::unreachable(&mut builder.chart);
    Ok(builder.chart)
}

impl Builder<'_> {
    fn new(function: &FunctionFlowIR) -> Self {
        Self {
            chart: FlowChart {
                symbol: function.symbol.clone(),
                file: function.file.clone(),
                span: function.span,
                diagnostics: function.diagnostics.clone(),
                partial: !function.diagnostics.is_empty(),
                ..Default::default()
            },
            calls: CallFrame::default(),
            recording: None,
            selection: None,
            cancel: None,
            allowance: None,
        }
    }
    fn step(&mut self, spec: StepSpec<'_>, incoming: &[Exit]) -> Result<usize> {
        self.check_cancel()?;
        self.check_size(ExpansionLimit::Steps, self.chart.steps.len() + 1)?;
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
        if let Some(recording) = &mut self.recording {
            recording.step(calls)?;
        }
        let id = self.chart.steps.len();
        let step = FlowStep {
            id,
            kind,
            label: compact(&label),
            span,
            calls: calls.to_vec(),
            ..Default::default()
        };
        self.calls.register(&step);
        self.chart.steps.push(step);
        for exit in incoming {
            self.link(exit, id)?;
        }
        Ok(id)
    }
    fn link(&mut self, exit: &Exit, to: usize) -> Result<()> {
        self.check_size(ExpansionLimit::Links, self.chart.links.len() + 1)?;
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
            StatementKind::Action { text, calls } => {
                self.action(StepSpec::new("action", text, span).calls(calls), incoming)
            }
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
    fn action(&mut self, spec: StepSpec<'_>, incoming: Vec<Exit>) -> Result<Vec<Exit>> {
        if let Some(value) = self.selected_value(&spec, &incoming)? {
            return Ok(match value {
                expansion::SelectedValue::Expanded { exits } => exits,
                expansion::SelectedValue::Stopped { step } => vec![Exit::normal(step)],
            });
        }
        let id = self.step(spec, &incoming)?;
        Ok(vec![Exit::normal(id)])
    }
    fn transfer(
        &mut self,
        spec: StepSpec<'_>,
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
