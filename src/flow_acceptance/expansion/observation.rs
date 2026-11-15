//! Assertions over immutable sessions and the source rows actually drawn by the native host.
use super::{config::Configuration, report::Report};
use crate::{
    app::State, camera::Camera, flow_acceptance::observation::CallSelection,
    flow_workspace::Session,
};
use anyhow::{Context, Result, ensure};
use codecallnavigator::flow::{self, ExpansionStop, OccurrenceOutcome, SiteAddress};
use rune::Runtime;

pub(super) type Check =
    fn(&mut Evidence, &Runtime<State>, &Configuration, &mut Report) -> Result<bool>;
pub(super) type Select = fn(&Evidence, &Session, &Configuration) -> Result<CallSelection>;
#[derive(Default)]
pub(super) struct Evidence {
    pub ordinary: Option<Saved>,
    pub follow: Option<Saved>,
    pub pan: Option<Saved>,
    pub sites: Vec<SiteAddress>,
    pub navigation: Option<serde_json::Value>,
}
pub(super) struct Saved {
    chart: serde_json::Value,
    expansion: Option<serde_json::Value>,
    camera: Camera,
    scroll: [f32; 2],
    selected: Option<usize>,
}
impl Saved {
    fn capture(session: &Session) -> Result<Self> {
        Ok(Self {
            chart: serde_json::to_value(
                &session
                    .ready
                    .as_ref()
                    .context("Session is not ready")?
                    .chart,
            )?,
            expansion: session
                .expanded_flow()
                .map(serde_json::to_value)
                .transpose()?,
            camera: session.camera.clone(),
            scroll: session.scroll.get(),
            selected: session.selected,
        })
    }
    fn restored(&self, session: &Session) -> Result<()> {
        let actual = Self::capture(session)?;
        ensure!(
            self.chart == actual.chart && self.expansion == actual.expansion,
            "Restored chart or occurrence choices differ"
        );
        ensure!(
            self.selected == actual.selected
                && close(self.scroll, actual.scroll)
                && same_camera(&self.camera, &actual.camera),
            "Restored selection, camera or source scroll differ"
        );
        Ok(())
    }
}
pub(super) fn session(runtime: &Runtime<State>) -> Result<&Session> {
    runtime
        .state
        .flow_workspace
        .current
        .as_ref()
        .context("Flow session disappeared")
}
fn expanded(runtime: &Runtime<State>) -> Result<Option<&flow::ExpandedFlow>> {
    Ok(session(runtime)?.expanded_flow())
}
fn count(flow: &flow::ExpandedFlow) -> usize {
    flow.occurrences
        .iter()
        .filter(|occurrence| occurrence.outcome == OccurrenceOutcome::Expanded)
        .count()
}
pub(super) fn boot(
    e: &mut Evidence,
    rt: &Runtime<State>,
    c: &Configuration,
    _: &mut Report,
) -> Result<bool> {
    let Some(context) = &rt.state.query_context else {
        return Ok(false);
    };
    ensure!(
        !rt.state.uses_fixture_queries() && context.descriptor.project == c.project,
        "Probe did not boot the real configured CLI project"
    );
    if !rt.state.view_is_current() {
        return Ok(false);
    }
    ensure!(
        rt.state.nav.selected.as_deref() == Some(c.root_symbol.as_str()),
        "Normal boot did not restore the selected root"
    );
    e.navigation = Some(serde_json::to_value(&rt.state.nav)?);
    Ok(true)
}
pub(super) fn root(
    e: &mut Evidence,
    rt: &Runtime<State>,
    c: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    if crate::flow_acceptance::observation::session(&rt.state, &c.root_symbol)?.is_none() {
        return Ok(false);
    }
    let s = session(rt)?;
    ensure!(
        s.expanded_flow().is_none(),
        "Ordinary root unexpectedly started expanded"
    );
    exact_source(s, c, &c.root_symbol)?;
    crate::flow_acceptance::observation::controls_visible(rt)?;
    e.ordinary = Some(Saved::capture(s)?);
    r.check("root_drawn");
    r.check("source_snapshot_exact");
    Ok(true)
}
pub(super) fn bootstrap(
    e: &mut Evidence,
    rt: &Runtime<State>,
    c: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    let Some(flow) = expanded(rt)? else {
        return Ok(false);
    };
    ensure!(
        count(flow) == 0 && session(rt)?.selected.is_none(),
        "Enabling expansion changed the collapsed body or retained a step index"
    );
    flow.validate()?;
    let calls = root_calls(flow, &c.helper_symbol);
    ensure!(
        calls.len() == 2 && calls[0].1 != calls[1].1,
        "Expected two distinct compiler-owned root call slots"
    );
    e.sites = calls.into_iter().map(|(_, site)| site).collect();
    r.check("two_distinct_call_slots");
    Ok(true)
}
pub(super) fn first(
    e: &mut Evidence,
    rt: &Runtime<State>,
    c: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    let Some(flow) = expanded(rt)? else {
        return Ok(false);
    };
    if count(flow) != 1 {
        return Ok(false);
    }
    flow.validate()?;
    let child = flow
        .occurrences
        .iter()
        .find(|o| o.outcome == OccurrenceOutcome::Expanded)
        .context("No expanded child")?;
    ensure!(
        child.symbol == c.helper_symbol
            && child.parent == Some(flow::OccurrenceId(0))
            && child.path.0.len() == 1
            && child.path.0.first().map(|choice| &choice.site) == e.sites.first(),
        "Expand selected a different root call site"
    );
    r.check("first_child_expanded");
    Ok(true)
}
pub(super) fn child_source(
    _: &mut Evidence,
    rt: &Runtime<State>,
    c: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    let s = session(rt)?;
    exact_source(s, c, &c.helper_symbol)?;
    let flow = s
        .expanded_flow()
        .context("Child selection lost expansion")?;
    let selected = s.selected.context("Child step is not selected")?;
    let origin = &flow.step_origins[selected];
    let owner = flow
        .sources
        .get(origin.source.context("Child step lacks source ownership")?)
        .context("Missing child source")?;
    ensure!(
        (std::path::Path::new(&owner.owner.file)
            == std::path::Path::new(&c.source(&c.helper_symbol)?.path))
            || c.project.join(
                owner
                    .owner
                    .file
                    .strip_prefix("@repo/")
                    .unwrap_or(&owner.owner.file)
            ) == std::path::Path::new(&c.source(&c.helper_symbol)?.path),
        "Child highlight belongs to another source file"
    );
    ensure!(
        highlighted(rt, s)?,
        "Selected child source highlight was not visible"
    );
    r.check("source_owner_switched");
    Ok(true)
}
pub(super) fn cycle(
    _: &mut Evidence,
    rt: &Runtime<State>,
    _: &Configuration,
    _: &mut Report,
) -> Result<bool> {
    let Some(flow) = expanded(rt)? else {
        return Ok(false);
    };
    let Some(stopped) = flow.occurrences.iter().find(|o| {
        matches!(
            o.outcome,
            OccurrenceOutcome::Stopped {
                reason: ExpansionStop::Cycle,
                ..
            }
        )
    }) else {
        return Ok(false);
    };
    let message = match &stopped.outcome {
        OccurrenceOutcome::Stopped { message, .. } => message,
        _ => unreachable!(),
    };
    let marker = stopped
        .boundaries
        .marker
        .context("Cycle lacks visible stop marker")?;
    let label = &flow.chart.steps[marker].label;
    ensure!(
        !message.is_empty() && !label.is_empty(),
        "Cycle explanation is empty"
    );
    Ok(true)
}
pub(super) fn cycle_visible(
    _: &mut Evidence,
    rt: &Runtime<State>,
    _: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    let s = session(rt)?;
    let flow = s
        .expanded_flow()
        .context("Cycle selection lost expansion")?;
    let selected = s.selected.context("Cycle marker was not selected")?;
    let stopped = flow
        .occurrences
        .iter()
        .find(|occurrence| {
            occurrence.boundaries.marker == Some(selected)
                && matches!(
                    occurrence.outcome,
                    OccurrenceOutcome::Stopped {
                        reason: ExpansionStop::Cycle,
                        ..
                    }
                )
        })
        .context("Selected marker does not own the cycle stop")?;
    let OccurrenceOutcome::Stopped { message, .. } = &stopped.outcome else {
        unreachable!()
    };
    ensure!(
        visible_text(rt, message),
        "Cycle stop reason is not visible in the native details"
    );
    r.check("recursive_cycle_stop_visible");
    Ok(true)
}
pub(super) fn both(
    e: &mut Evidence,
    rt: &Runtime<State>,
    c: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    let Some(flow) = expanded(rt)? else {
        return Ok(false);
    };
    if count(flow) != 2 {
        return Ok(false);
    }
    let children: Vec<_> = flow
        .occurrences
        .iter()
        .filter(|o| o.outcome == OccurrenceOutcome::Expanded && o.symbol == c.helper_symbol)
        .collect();
    ensure!(
        children.len() == 2
            && children[0].path != children[1].path
            && children[0].id != children[1].id,
        "Repeated callee occurrences were merged"
    );
    ensure!(
        e.sites
            .iter()
            .all(|site| children
                .iter()
                .any(|child| child.parent == Some(flow::OccurrenceId(0))
                    && child.path.0.len() == 1
                    && child
                        .path
                        .0
                        .first()
                        .is_some_and(|choice| &choice.site == site))),
        "Expanded sibling paths differ from the selected root sites"
    );
    r.check("second_child_expanded");
    Ok(true)
}
pub(super) fn one(
    e: &mut Evidence,
    rt: &Runtime<State>,
    _: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    let Some(flow) = expanded(rt)? else {
        return Ok(false);
    };
    if count(flow) != 1 {
        return Ok(false);
    }
    let child = flow
        .occurrences
        .iter()
        .find(|o| o.outcome == OccurrenceOutcome::Expanded)
        .context("No sibling remains")?;
    ensure!(
        child
            .path
            .0
            .first()
            .is_some_and(|choice| Some(&choice.site) == e.sites.get(1)),
        "Collapse removed the other sibling"
    );
    r.check("collapse_retains_other_child");
    Ok(true)
}
pub(super) fn before_pan(
    e: &mut Evidence,
    rt: &Runtime<State>,
    _: &Configuration,
    _: &mut Report,
) -> Result<bool> {
    e.pan = Some(Saved::capture(session(rt)?)?);
    Ok(true)
}
pub(super) fn panned(
    e: &mut Evidence,
    rt: &Runtime<State>,
    _: &Configuration,
    _: &mut Report,
) -> Result<bool> {
    let s = session(rt)?;
    let previous = e.pan.as_ref().context("No pre-pan observation")?;
    ensure!(
        !same_camera(&previous.camera, &s.camera) && close(previous.scroll, s.scroll.get()),
        "Pan did not move only the chart camera"
    );
    e.pan = Some(Saved::capture(s)?);
    Ok(true)
}
pub(super) fn scrolled(
    e: &mut Evidence,
    rt: &Runtime<State>,
    _: &Configuration,
    _: &mut Report,
) -> Result<bool> {
    let s = session(rt)?;
    let previous = e.pan.as_ref().context("No post-pan observation")?;
    ensure!(
        !close(previous.scroll, s.scroll.get()) && same_camera(&previous.camera, &s.camera),
        "Source wheel did not move only source scroll"
    );
    e.follow = Some(Saved::capture(s)?);
    Ok(true)
}
pub(super) fn target(
    _: &mut Evidence,
    rt: &Runtime<State>,
    c: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    if crate::flow_acceptance::observation::session(&rt.state, &c.helper_symbol)?.is_none() {
        return Ok(false);
    }
    exact_source(session(rt)?, c, &c.helper_symbol)?;
    ensure!(
        rt.state.flow_workspace.back.len() == 1,
        "Following a target did not retain caller history"
    );
    r.check("callee_followed");
    Ok(true)
}
pub(super) fn restored(
    e: &mut Evidence,
    rt: &Runtime<State>,
    c: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    if crate::flow_acceptance::observation::session(&rt.state, &c.root_symbol)?.is_none() {
        return Ok(false);
    }
    e.follow
        .as_ref()
        .context("No caller snapshot before follow")?
        .restored(session(rt)?)?;
    r.check("back_restores_expansion_camera_scroll");
    Ok(true)
}
pub(super) fn ordinary(
    e: &mut Evidence,
    rt: &Runtime<State>,
    _: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    let s = session(rt)?;
    if s.expanded_flow().is_some() {
        return Ok(false);
    }
    e.ordinary
        .as_ref()
        .context("No original ordinary session")?
        .restored(s)?;
    r.check("collapse_all_restores_ordinary");
    Ok(true)
}
pub(super) fn graph(
    e: &mut Evidence,
    rt: &Runtime<State>,
    _: &Configuration,
    r: &mut Report,
) -> Result<bool> {
    if rt.state.flow_workspace.current.is_some() {
        return Ok(false);
    }
    ensure!(
        Some(serde_json::to_value(&rt.state.nav)?) == e.navigation,
        "Closing flow changed graph navigation"
    );
    r.check("graph_restored");
    Ok(true)
}

pub(super) fn first_call(e: &Evidence, s: &Session, c: &Configuration) -> Result<CallSelection> {
    root_call(e, s, c, 0)
}
pub(super) fn second_call(e: &Evidence, s: &Session, c: &Configuration) -> Result<CallSelection> {
    root_call(e, s, c, 1)
}
fn root_call(e: &Evidence, s: &Session, c: &Configuration, index: usize) -> Result<CallSelection> {
    let address = e.sites.get(index).context("Root call slot unavailable")?;
    find_call(s, &c.helper_symbol, |flow, step, call| {
        flow.step_origins[step].occurrence.0 == 0
            && flow.step_origins[step].calls.get(call) == Some(address)
    })
}
pub(super) fn first_child(_: &Evidence, s: &Session, c: &Configuration) -> Result<CallSelection> {
    find_call(s, &c.recursive_symbol, |flow, step, _| {
        flow.occurrences[flow.step_origins[step].occurrence.0]
            .path
            .0
            .len()
            == 1
    })
}
pub(super) fn first_entry(e: &Evidence, s: &Session, _: &Configuration) -> Result<CallSelection> {
    let flow = s.expanded_flow().context("No expanded chart")?;
    let step = flow
        .occurrences
        .iter()
        .find(|occurrence| {
            occurrence.outcome == OccurrenceOutcome::Expanded
                && occurrence.path.0.len() == 1
                && occurrence.path.0.first().map(|choice| &choice.site) == e.sites.first()
        })
        .and_then(|occurrence| occurrence.boundaries.entry)
        .context("No first expanded entry boundary")?;
    Ok(CallSelection {
        step,
        call: 0,
        target: 0,
    })
}
pub(super) fn marker(_: &Evidence, s: &Session, _: &Configuration) -> Result<CallSelection> {
    let flow = s.expanded_flow().context("No expanded flow")?;
    let step = flow
        .occurrences
        .iter()
        .find(|o| {
            matches!(
                o.outcome,
                OccurrenceOutcome::Stopped {
                    reason: ExpansionStop::Cycle,
                    ..
                }
            )
        })
        .and_then(|o| o.boundaries.marker)
        .context("No cycle marker")?;
    Ok(CallSelection {
        step,
        call: 0,
        target: 0,
    })
}
fn find_call(
    s: &Session,
    symbol: &str,
    accept: impl Fn(&flow::ExpandedFlow, usize, usize) -> bool,
) -> Result<CallSelection> {
    let flow = s
        .expanded_flow()
        .context("No expanded chart for call selection")?;
    for (step, value) in flow.chart.steps.iter().enumerate() {
        for (call, site) in value.calls.iter().enumerate() {
            if let Some(target) = site
                .targets
                .iter()
                .position(|target| target.symbol == symbol)
                && accept(flow, step, call)
            {
                return Ok(CallSelection { step, call, target });
            }
        }
    }
    anyhow::bail!("Expected exact call occurrence is missing")
}
fn root_calls(flow: &flow::ExpandedFlow, symbol: &str) -> Vec<(usize, SiteAddress)> {
    flow.chart
        .steps
        .iter()
        .enumerate()
        .flat_map(|(index, step)| {
            step.calls
                .iter()
                .enumerate()
                .filter_map(move |(call, site)| {
                    let origin = &flow.step_origins[index];
                    (origin.occurrence.0 == 0
                        && site.targets.iter().any(|target| target.symbol == symbol))
                    .then(|| origin.calls.get(call).cloned().map(|site| (index, site)))
                    .flatten()
                })
        })
        .collect()
}
fn exact_source(s: &Session, c: &Configuration, symbol: &str) -> Result<()> {
    let source = s
        .displayed_source()
        .context("No displayed source snapshot")?;
    ensure!(
        source.text == c.source(symbol)?.content,
        "Displayed source differs from exact compiler snapshot"
    );
    Ok(())
}
fn highlighted(rt: &Runtime<State>, s: &Session) -> Result<bool> {
    let source = s.displayed_source().context("No displayed source")?;
    let span = s
        .selected_source_span()
        .context("Selected child has no source span")?;
    let mut visible = false;
    let elements = rt.elements();
    for element in &elements {
        let Some(index) = highlighted_index(element) else {
            continue;
        };
        let line = source
            .lines
            .get(index)
            .context("Visible source row is outside its owner")?;
        let end = source
            .lines
            .get(index + 1)
            .map_or(line.range.end, |line| line.range.start);
        ensure!(
            line.range.start < span.end && end > span.start,
            "Source highlight belongs to an unrelated row"
        );
        if element.visible[3] > 1.0 {
            require_drawn_line(&elements, source, index)?;
            visible = true;
        }
    }
    Ok(visible)
}
fn highlighted_index(element: &rune::ElementInfo) -> Option<usize> {
    (element.fill.a > 0.1).then_some(())?;
    element
        .key
        .as_deref()?
        .strip_prefix("flow-source-line-")?
        .parse()
        .ok()
}
fn require_drawn_line(
    elements: &[rune::ElementInfo],
    source: &crate::source::SourceFile,
    index: usize,
) -> Result<()> {
    let key = format!("flow-source-text-{index}");
    let expected = source.text[source.lines[index].range.clone()].trim_end_matches('\r');
    ensure!(
        elements.iter().any(|text| text.key.as_deref() == Some(&key)
            && text.visible[2] > 1.0
            && text.visible[3] > 1.0
            && text.text.as_deref() == Some(expected)),
        "Drawn highlighted source text belongs to another snapshot"
    );
    Ok(())
}
fn visible_text(rt: &Runtime<State>, text: &str) -> bool {
    rt.elements().iter().any(|element| {
        element.visible[2] > 1.0
            && element.visible[3] > 1.0
            && element
                .text
                .as_deref()
                .is_some_and(|actual| actual.contains(text))
    })
}
fn close(a: [f32; 2], b: [f32; 2]) -> bool {
    a.into_iter().zip(b).all(|(a, b)| (a - b).abs() < 1.0)
}
fn same_camera(a: &Camera, b: &Camera) -> bool {
    close(a.pan, b.pan) && (a.zoom - b.zoom).abs() < 0.001
}
