//! Discard synthetic loop anchors unreachable from the function or a deferred region.
use super::FlowChart;
pub(super) fn unreachable(chart: &mut FlowChart) {
    let visible = visible_steps(chart);
    let mut positions = vec![usize::MAX; chart.steps.len()];
    chart.steps.retain(|step| visible[step.id]);
    for (index, step) in chart.steps.iter_mut().enumerate() {
        positions[step.id] = index;
        step.id = index;
    }
    chart
        .links
        .retain(|link| visible[link.from] && visible[link.to]);
    for link in &mut chart.links {
        link.from = positions[link.from];
        link.to = positions[link.to];
    }
    remap_regions(chart, &visible, &positions);
    chart.entry = positions[chart.entry];
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
fn remap_regions(chart: &mut FlowChart, visible: &[bool], positions: &[usize]) {
    for region in &mut chart.regions {
        region.entry = positions[region.entry];
        region.steps.retain(|&step| visible[step]);
        for step in &mut region.steps {
            *step = positions[*step];
        }
    }
}
