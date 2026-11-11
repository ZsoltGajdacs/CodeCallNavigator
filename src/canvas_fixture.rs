//! Explicit dense and saved-workspace scenes for CPU and native pan measurements.
use crate::{
    app::State,
    projection::{ViewEdge, ViewNode},
    scenes,
};

pub const NODES: usize = 300;
pub const EDGES: usize = 6_000;

pub fn install(state: &mut State) {
    scenes::install(state);
    state.deterministic = true;
    state.graph_view.nodes = (0..NODES)
        .map(|index| ViewNode {
            key: format!("canvas-benchmark-{index}"),
            label: format!("Method{index}"),
            detail: "Benchmark fixture".into(),
            position: [(index % 20) as f32 * 160.0, (index / 20) as f32 * 110.0],
            size: [150.0, 90.0],
            count: 1,
            ..Default::default()
        })
        .collect();
    state.graph_view.edges = (0..EDGES)
        .map(|index| {
            let from = index % NODES;
            let to = (from + 1 + (index / NODES) * 13) % NODES;
            let start = state.graph_view.nodes[from].position;
            let end = state.graph_view.nodes[to].position;
            let bend = match index % 3 {
                0 => 0.0,
                1 => 1.0,
                _ => 180.0,
            };
            ViewEdge {
                classification: codecallnavigator::model::RelationshipClassification {
                    kind: codecallnavigator::model::RelationshipKind::Call,
                    resolution: if index % 4 == 0 {
                        codecallnavigator::model::TargetResolution::VirtualDispatch
                    } else {
                        codecallnavigator::model::TargetResolution::Direct
                    },
                },
                from,
                to,
                count: 1,
                points: vec![
                    start,
                    [
                        start[0] + (end[0] - start[0]) / 3.0,
                        start[1] + (end[1] - start[1]) / 3.0 + bend,
                    ],
                    [
                        start[0] + (end[0] - start[0]) * 2.0 / 3.0,
                        start[1] + (end[1] - start[1]) * 2.0 / 3.0 + bend,
                    ],
                    end,
                ],
            }
        })
        .collect();
    state.graph_view.size = [3200.0, 1650.0];
    state.nav.camera.zoom = 0.20;
    state.nav.camera.pan = [24.0, 100.0];
    let view = std::mem::take(&mut state.graph_view);
    state.replace_graph_view(view);
}

/// Restore the copied saved navigation synchronously, without running analyzers or discovery.
pub fn saved(state: &mut State) {
    state.scene = true;
    state.deterministic = true;
    state.jobs.boot.cancel();
    if let Err(error) = restore(state) {
        state.warning = format!("Cannot restore profiling workspace: {error:#}");
    }
}

fn restore(state: &mut State) -> anyhow::Result<()> {
    use std::sync::Arc;
    let boot = crate::workspace::load_boot(crate::workspace::data_directory())?;
    let saved = boot
        .session
        .ok_or_else(|| anyhow::anyhow!("No saved navigation"))?;
    let loaded = boot
        .graph
        .ok_or_else(|| anyhow::anyhow!("No published graph"))?;
    state.root = rune::TextEdit::new(saved.root);
    state.nav = saved.navigation;
    state.ui.restore(saved.presentation);
    state.architecture_selection = saved.architecture_selection;
    state.generation = Some(loaded.generation);
    state.graph = Some(loaded.graph);
    if let Some(selection) = saved.architecture_selection {
        // Normal loading may migrate a review configuration. Profiling must only read it.
        let root = std::path::Path::new(&state.root.text);
        if selection == codecallnavigator::architecture::Selection::Repository {
            anyhow::ensure!(
                root.join(".codecallnavigator/layers.yml").is_file(),
                "Profiling requires an existing .codecallnavigator/layers.yml; importing repository configuration is not a profiling action"
            );
        }
        let config = codecallnavigator::architecture::load(root, selection)?;
        state.architecture = Some(Arc::new(codecallnavigator::architecture::classify(
            &state.graph.as_ref().unwrap().data,
            &config.config,
            config.origin,
        )?));
    }
    let request = state.request().unwrap();
    let view = crate::projection::layout(
        crate::projection::project(state.graph.as_ref().unwrap(), &request),
        &boot.runtime.dot,
    )?;
    state.replace_graph_view(view);
    state.visible_revision = Some((state.epoch, state.revision));
    state.auto_fit = false;
    state.status = format!(
        "Saved profiling view · {} cards · {} relationships",
        state.graph_view.nodes.len(),
        state.graph_view.edges.len()
    );
    Ok(())
}
