//! Discard synthetic loop anchors unreachable from the function or a deferred region.
use super::FlowChart;

/// One old-to-new mapping is used for every chart reference. Removed steps stay absent.
pub(super) struct StepRemap(Vec<Option<usize>>);
impl StepRemap {
    pub(super) fn get(&self, old: usize) -> Option<usize> {
        self.0[old]
    }
    fn retained(&self, old: usize) -> usize {
        self.get(old).expect("reachable entry is retained")
    }
}
pub(super) fn unreachable(chart: &mut FlowChart) -> StepRemap {
    let visible = visible_steps(chart);
    let mut positions = StepRemap(vec![None; chart.steps.len()]);
    chart.steps.retain(|step| visible[step.id]);
    for (index, step) in chart.steps.iter_mut().enumerate() {
        positions.0[step.id] = Some(index);
        step.id = index;
    }
    chart.links.retain_mut(
        |link| match (positions.get(link.from), positions.get(link.to)) {
            (Some(from), Some(to)) => {
                link.from = from;
                link.to = to;
                true
            }
            _ => false,
        },
    );
    remap_regions(chart, &positions);
    chart.entry = positions.retained(chart.entry);
    positions
}
fn visible_steps(chart: &FlowChart) -> Vec<bool> {
    let mut outgoing = vec![Vec::new(); chart.steps.len()];
    for link in &chart.links {
        outgoing[link.from].push(link.to);
    }
    let mut queue = vec![chart.entry];
    queue.extend(chart.regions.iter().map(|region| region.entry));
    let mut visible = vec![false; chart.steps.len()];
    while let Some(index) = queue.pop() {
        if visible[index] {
            continue;
        }
        visible[index] = true;
        queue.extend(&outgoing[index]);
    }
    visible
}
fn remap_regions(chart: &mut FlowChart, positions: &StepRemap) {
    for region in &mut chart.regions {
        region.entry = positions.retained(region.entry);
        region.steps.retain_mut(|step| {
            if let Some(index) = positions.get(*step) {
                *step = index;
                true
            } else {
                false
            }
        });
    }
}
