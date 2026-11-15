//! Stage a callee independently, then relocate it without registering its calls in the parent.
use super::super::{Builder, Completion, Exit, StepSpec, recording::Recording};
use crate::flow::*;
use anyhow::Result;
use std::sync::atomic::AtomicBool;
pub(super) struct Staged<'a> {
    builder: Builder<'a>,
    entry: usize,
    pub exits: Vec<Exit>,
    id: OccurrenceId,
    span: SourceSpan,
    request: usize,
}
pub(super) struct Input<'a> {
    pub function: &'a FunctionFlowIR,
    pub index: std::rc::Rc<super::super::recording::OriginalIndex>,
    pub source: SourceId,
    pub id: OccurrenceId,
    pub request: usize,
}
struct Connections {
    exits: Vec<Exit>,
    returns: Vec<usize>,
    throws: Vec<usize>,
}
pub(super) fn stage<'a>(
    input: Input<'a>,
    run: super::Selection<'a>,
    cancel: Option<&'a AtomicBool>,
) -> Result<Staged<'a>> {
    let mut builder = Builder::new(input.function);
    builder.calls = super::super::calls::CallFrame::callee();
    builder.recording = Some(Recording::new(input.index, input.id, input.source));
    builder.cancel = cancel;
    builder.allowance = Some(super::budget::Allowance::child(run.limits, input.request));
    builder.selection = Some(run);
    let assembled = builder.function(
        StepSpec::new("start", "Callee entry", input.function.span),
        &input.function.body,
    )?;
    let mut exits = assembled.exits;
    builder.function_call_exceptions(&mut exits);
    Ok(Staged {
        builder,
        entry: assembled.entry,
        exits,
        id: input.id,
        span: input.function.span,
        request: input.request,
    })
}
pub(super) fn graft<'a>(
    parent: &mut Builder<'a>,
    mut staged: Staged<'a>,
    call: usize,
) -> Result<super::Activated<'a>> {
    let mut connections = staged.connections()?;
    let mut run = staged.builder.selection.take().unwrap();
    let offset = append(parent, staged.builder, staged.request)?;
    let entry = offset + staged.entry;
    connections.relocate(offset);
    for occurrence in &mut run.occurrences[staged.id.0..] {
        super::remap_boundaries(&mut occurrence.boundaries, |id| Some(id + offset));
    }
    parent.link(&Exit::normal(call), entry)?;
    if parent.recording.as_ref().unwrap().occurrence() == OccurrenceId(0) {
        parent.allowance.as_mut().unwrap().candidate = Some(staged.request);
    }

    connections.exits.push(
        Exit::with_completion(call, Completion::Throw)
            .branch("call may fail before entry", "exception"),
    );
    run.occurrences[staged.id.0].boundaries = BoundaryRefs {
        call: Some(call),
        entry: Some(entry),
        returns: connections.returns,
        throws: connections.throws,
        marker: None,
    };
    Ok(super::Activated {
        run,
        value: super::SelectedValue::Expanded {
            exits: connections.exits,
        },
    })
}
impl<'a> Staged<'a> {
    pub fn into_run(mut self) -> super::Selection<'a> {
        self.builder.selection.take().unwrap()
    }
}
impl Staged<'_> {
    fn connections(&mut self) -> Result<Connections> {
        let (normal, thrown): (Vec<_>, Vec<_>) = std::mem::take(&mut self.exits)
            .into_iter()
            .partition(|exit| exit.completion != Completion::Throw);
        let mut connections = Connections {
            exits: vec![],
            returns: vec![],
            throws: vec![],
        };
        if !normal.is_empty() {
            let id = self.boundary(&normal, OriginRole::Return)?;
            connections.returns.push(id);
            connections.exits.push(Exit::normal(id));
        }
        if !thrown.is_empty() {
            let id = self.boundary(&thrown, OriginRole::Throw)?;
            connections.throws.push(id);
            connections
                .exits
                .push(Exit::with_completion(id, Completion::Throw));
        }
        Ok(connections)
    }
    fn boundary(&mut self, exits: &[Exit], role: OriginRole) -> Result<usize> {
        let (kind, label) = match role {
            OriginRole::Return => ("return_boundary", "Return from callee"),
            OriginRole::Throw => ("throw_boundary", "Exception from callee"),
            _ => unreachable!("callee exit boundary"),
        };
        let id = self
            .builder
            .step(StepSpec::new(kind, label, self.span), exits)?;
        self.builder.recording.as_mut().unwrap().role(id, role);
        Ok(id)
    }
}
impl Connections {
    fn relocate(&mut self, offset: usize) {
        for exit in &mut self.exits {
            exit.from += offset;
        }
        for id in self.returns.iter_mut().chain(&mut self.throws) {
            *id += offset;
        }
    }
}
fn append(parent: &mut Builder<'_>, mut child: Builder<'_>, request: usize) -> Result<usize> {
    parent.check_cancel()?;
    let offset = parent.chart.steps.len();
    let allowance = super::budget::Allowance::child(parent.allowance.unwrap().limits, request);
    allowance.check(
        ExpansionLimit::Steps,
        (offset + child.chart.steps.len()) as u64,
    )?;
    allowance.check(
        ExpansionLimit::Links,
        (parent.chart.links.len() + child.chart.links.len() + 1) as u64,
    )?;
    relocate(&mut child.chart, offset);
    parent.chart.steps.extend(child.chart.steps);
    parent.chart.links.extend(child.chart.links);
    parent.chart.regions.extend(child.chart.regions);
    for diagnostic in child.chart.diagnostics {
        parent.diagnostic(&diagnostic);
    }
    parent
        .recording
        .as_mut()
        .unwrap()
        .origins
        .extend(child.recording.take().unwrap().origins);
    Ok(offset)
}
fn relocate(chart: &mut FlowChart, offset: usize) {
    for step in &mut chart.steps {
        step.id += offset;
    }
    for link in &mut chart.links {
        link.from += offset;
        link.to += offset;
    }
    for region in &mut chart.regions {
        region.entry += offset;
        for step in &mut region.steps {
            *step += offset;
        }
    }
}
