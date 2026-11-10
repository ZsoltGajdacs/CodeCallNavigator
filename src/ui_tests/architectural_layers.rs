use super::{click, runtime};
use codegraph::architecture::{Assignment, LayerAssignments, LayerDefinition, Selection, file_key};
use std::sync::Arc;

#[test]
fn layer_control_exposes_presets_tabs_and_restores_navigation() {
    let mut runtime = runtime();
    runtime.state.architecture = Some(assignments(&runtime.state));
    runtime.state.architecture_selection = Some(Selection::Repository);
    click(&mut runtime, "grouping/segment4");
    assert_eq!(
        runtime.state.nav.grouping,
        crate::projection::Grouping::Layer
    );
    assert!(runtime.find("layer-preset").is_some());
    assert!(runtime.find("layer-tabs").is_some());
    assert!(
        runtime
            .state
            .graph_view
            .nodes
            .iter()
            .any(|node| node.label == "Service")
    );
    let elements = runtime.elements();
    let tabs = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("layer-tabs"))
        .unwrap();
    let canvas = elements
        .iter()
        .find(|e| e.key.as_deref() == Some("graph-canvas-wrapper"))
        .unwrap();
    assert!(
        canvas.rect[1] >= tabs.rect[1] + tabs.rect[3],
        "Canvas overlaps layer tabs during grouping transition"
    );
    click(&mut runtime, "layer-tabs/segment1");
    assert_eq!(runtime.state.nav.layer_tab.as_deref(), Some("Database"));
    assert_eq!(runtime.state.graph_view.nodes.len(), 1);
    assert_eq!(runtime.state.graph_view.nodes[0].label, "Repository");
    click(&mut runtime, "back");
    assert!(runtime.state.nav.layer_tab.is_none());
    assert_eq!(
        runtime.state.architecture_selection,
        Some(Selection::Repository)
    );
    click(&mut runtime, "layer-preset/segment2");
    assert_eq!(runtime.state.architecture_selection, Some(Selection::React));
}

fn assignments(state: &crate::app::State) -> Arc<LayerAssignments> {
    let graph = state.graph.as_ref().unwrap();
    let assignments = LayerAssignments {
        files: graph
            .data
            .nodes
            .iter()
            .map(|node| {
                (
                    file_key(&node.build, &node.project, &node.file),
                    Assignment {
                        layer: if node.label == "SaveOrder" {
                            "Repository"
                        } else {
                            "Service"
                        }
                        .into(),
                        rule: "fixture rule".into(),
                    },
                )
            })
            .collect(),
        layers: vec![
            LayerDefinition {
                name: "Service".into(),
                ..Default::default()
            },
            LayerDefinition {
                name: "Repository".into(),
                tab: Some("Database".into()),
                ..Default::default()
            },
        ],
        origin: "fixture configuration".into(),
    };
    Arc::new(assignments)
}

#[test]
fn layer_controls_stay_inside_the_minimum_window_with_both_panes_open() {
    let mut state = crate::app::State::new();
    crate::scenes::install(&mut state);
    state.ui.sidebar.set_width(400.0);
    state.ui.inspector.set_width(420.0);
    state.architecture = Some(assignments(&state));
    state.architecture_selection = Some(Selection::Repository);
    state.set_grouping(crate::projection::Grouping::Layer);
    let mut runtime = rune::Runtime::new(
        state,
        crate::view::view,
        (1050.0, 680.0),
        rune::Env::new(true, 1.0),
    );
    runtime.poll = Some(crate::jobs::poll);
    for _ in 0..60 {
        runtime.frame(1.0 / 60.0);
    }
    for key in ["grouping", "layer-preset", "layer-tabs"] {
        let element = runtime
            .elements()
            .into_iter()
            .find(|element| element.key.as_deref() == Some(key))
            .unwrap();
        assert!(
            element.visible[2] >= element.rect[2] - 1.0,
            "{key} is clipped"
        );
        assert!(
            element.rect[0] + element.rect[2] <= 1050.5,
            "{key} is outside the window"
        );
    }
    click(&mut runtime, "layer-tabs/segment1");
    assert_eq!(runtime.state.nav.layer_tab.as_deref(), Some("Database"));
}
