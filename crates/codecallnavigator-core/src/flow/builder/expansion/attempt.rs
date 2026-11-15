//! One actual assembly path, reused for mandatory-root proof and bounded retries.
use super::*;
use budget::Allowance;
use std::rc::Rc;
#[derive(Clone)]
pub(super) enum Mode {
    RootOnly,
    Expand(Vec<Option<ExpansionLimit>>),
}
pub(super) struct Attempt<'a> {
    chart: FlowChart,
    occurrences: Vec<FlowOccurrence>,
    sources: SourcePool<'a>,
    origins: Vec<StepOrigin>,
    blocked: Vec<BlockedSelection>,
}
pub(super) fn assemble<'a>(
    prepared: Rc<Prepared<'a>>,
    request: &ExpansionRequest,
    mode: Mode,
    cancel: &'a AtomicBool,
) -> Result<Attempt<'a>> {
    ensure!(!cancel.load(Ordering::Relaxed), "Flow expansion cancelled");
    let root = prepared
        .input(request.root())
        .context("Expansion root is unavailable")?;
    let mut sources = SourcePool::default();
    let source = sources.intern(root, Allowance::root(request.limits()))?;
    let index = prepared.index(request.root())?;
    let count = prepared.requests.len();
    let occurrence = FlowOccurrence {
        id: OccurrenceId(0),
        symbol: root.function.symbol.clone(),
        parent: None,
        path: OccurrencePath::default(),
        source: Some(source),
        owner: Some(owner(root.function)),
        span: root.function.span,
        outcome: OccurrenceOutcome::Root,
        boundaries: BoundaryRefs::default(),
    };
    let mut builder = Builder::new(root.function);
    builder.recording = Some(Recording::new(index, OccurrenceId(0), source));
    builder.cancel = Some(cancel);
    builder.allowance = Some(Allowance::root(request.limits()));
    builder.selection = Some(Selection {
        prepared,
        statuses: vec![Status::Pending; count],
        root: request.root().into(),
        sources,
        occurrences: vec![occurrence],
        limits: request.limits(),
        mode,
        evaluated: 0,
    });
    let assembled = builder.function(
        StepSpec::new("start", "Function entry", root.function.span),
        &root.function.body,
    )?;
    builder.chart.entry = assembled.entry;
    builder.finish(assembled.exits, root.function.span)?;
    let mut run = builder.selection.take().unwrap();
    ensure!(
        run.statuses.iter().all(|state| *state != Status::Pending),
        "Selected call occurrence is unavailable or stale"
    );
    run.occurrences[0].boundaries.entry = Some(assembled.entry);
    let blocked = run.blocked();
    let mut origins = builder.recording.take().unwrap().origins;
    remap(&mut builder.chart, &mut origins, &mut run.occurrences);
    Ok(Attempt {
        chart: builder.chart,
        occurrences: run.occurrences,
        sources: run.sources,
        origins,
        blocked,
    })
}
impl Attempt<'_> {
    pub fn materialize(self, root: &str, cancel: &AtomicBool) -> Result<ExpandedFlow> {
        ensure!(!cancel.load(Ordering::Relaxed), "Flow expansion cancelled");
        let result = ExpandedFlow {
            version: 1,
            root_symbol: root.into(),
            chart: self.chart,
            occurrences: self.occurrences,
            sources: self.sources.materialize()?,
            step_origins: self.origins,
            blocked_selections: self.blocked,
        };
        result.validate()?;
        ensure!(!cancel.load(Ordering::Relaxed), "Flow expansion cancelled");
        Ok(result)
    }
}
