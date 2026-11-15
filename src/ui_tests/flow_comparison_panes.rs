//! Actual pane controls preserve endpoint-local source, selection and camera coordinates.
use super::{
    click,
    flow_comparison_fixture::{choose, prefixed, runtime},
    flow_comparisons::{load, settle},
};
use crate::{
    app::State,
    flow_comparison::{Side, SideId},
    flow_view::address::Address,
};
use rune::Runtime;
fn session(runtime: &Runtime<State>, side: SideId) -> &crate::flow_workspace::Session {
    let Side::Ready(ready) = runtime
        .state
        .flow_comparison
        .current
        .as_ref()
        .unwrap()
        .side(side)
    else {
        panic!()
    };
    &ready.session
}
fn address(side: SideId) -> Address {
    match side {
        SideId::Before => Address::Before,
        SideId::After => Address::After,
    }
}
fn step(runtime: &Runtime<State>, side: SideId, calls: bool) -> usize {
    let ready = session(runtime, side).ready.as_ref().unwrap();
    if calls {
        ready
            .chart
            .steps
            .iter()
            .position(|step| !step.calls.is_empty())
            .unwrap()
    } else {
        ready
            .chart
            .steps
            .iter()
            .rposition(|step| step.kind == "return")
            .unwrap()
    }
}
fn select(runtime: &mut Runtime<State>, side: SideId, index: usize) {
    click(runtime, &address(side).key(&format!("step-{index}")));
    for _ in 0..4 {
        runtime.frame(1.0 / 60.0);
    }
}
fn set_scroll(runtime: &mut Runtime<State>, side: SideId, y: f32) {
    let key = session(runtime, side).scroll_key.clone();
    runtime.update(move |_, tx| tx.scroll_to_instant(key, 0.0, y));
    for _ in 0..4 {
        runtime.frame(1.0 / 60.0);
    }
}
fn rendered_scroll(runtime: &Runtime<State>, side: SideId) -> f32 {
    let session = session(runtime, side);
    let elements = runtime.elements();
    let viewport = elements
        .iter()
        .find(|element| element.key.as_deref() == Some(&session.scroll_key))
        .unwrap();
    let keys = address(side).keys(session);
    let (index, row) = session
        .ready
        .as_ref()
        .unwrap()
        .source
        .lines
        .iter()
        .enumerate()
        .find_map(|(i, _)| {
            elements
                .iter()
                .find(|element| {
                    element.key.as_deref() == Some(keys.line(i).as_str())
                        && element.visible[3] > 19.0
                })
                .map(|row| (i, row))
        })
        .unwrap();
    viewport.rect[1] + index as f32 * 20.0 - row.rect[1]
}
fn assert_highlight(runtime: &Runtime<State>, side: SideId, index: usize) {
    let session = session(runtime, side);
    let ready = session.ready.as_ref().unwrap();
    let span = ready.chart.steps[index].span;
    let line = ready
        .source
        .lines
        .partition_point(|line| line.range.end < span.start)
        .min(ready.source.lines.len() - 1);
    let key = address(side).keys(session).line(line);
    let row = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some(&key))
        .unwrap();
    assert!(row.fill.a > 0.01, "Wrong endpoint source did not highlight");
}
fn world(camera: &crate::camera::Camera, point: [f32; 2]) -> [f32; 2] {
    [
        (point[0] - camera.pan[0]) / camera.zoom,
        (point[1] - camera.pan[1]) / camera.zoom,
    ]
}
#[test]
fn pane_steps_tabs_scroll_and_zoom_are_independent_and_use_local_coordinates() {
    let (mut runtime, _project) = runtime(prefixed("before", 200), prefixed("after", 350), true);
    load(&mut runtime);
    choose(&mut runtime, "load(");
    let before = step(&runtime, SideId::Before, true);
    select(&mut runtime, SideId::Before, before);
    assert_highlight(&runtime, SideId::Before, before);
    assert!(session(&runtime, SideId::After).selected.is_none());
    set_scroll(&mut runtime, SideId::Before, 120.0);
    assert!((rendered_scroll(&runtime, SideId::Before) - 120.0).abs() < 1.0);
    click(&mut runtime, "comparison-source-after");
    let after = step(&runtime, SideId::After, false);
    select(&mut runtime, SideId::After, after);
    assert_highlight(&runtime, SideId::After, after);
    set_scroll(&mut runtime, SideId::After, 360.0);
    assert!((rendered_scroll(&runtime, SideId::After) - 360.0).abs() < 1.0);
    click(&mut runtime, "comparison-source-before");
    assert!((rendered_scroll(&runtime, SideId::Before) - 120.0).abs() < 1.0);
    assert_eq!(session(&runtime, SideId::Before).selected, Some(before));
    assert_eq!(session(&runtime, SideId::After).selected, Some(after));
    let old_before = serde_json::to_value(&session(&runtime, SideId::Before).camera).unwrap();
    let old_after = session(&runtime, SideId::After).camera.clone();
    let pane = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("comparison-after-canvas"))
        .unwrap();
    let point = [pane.rect[2] * 0.75, pane.rect[3] * 0.5];
    let anchor = world(&old_after, point);
    runtime.wheel_xy(pane.rect[0] + point[0], pane.rect[1] + point[1], 0.0, 30.0);
    runtime.frame(1.0 / 60.0);
    assert_eq!(
        serde_json::to_value(&session(&runtime, SideId::Before).camera).unwrap(),
        old_before
    );
    let actual = world(&session(&runtime, SideId::After).camera, point);
    assert!((actual[0] - anchor[0]).abs() < 0.01 && (actual[1] - anchor[1]).abs() < 0.01);
    assert!(session(&runtime, SideId::After).camera.zoom > old_after.zoom);
}
#[test]
fn target_and_back_restore_active_source_tab_scroll_selection_and_cameras() {
    let (mut runtime, _project) = runtime(prefixed("before", 200), prefixed("after", 350), true);
    load(&mut runtime);
    let symbol = choose(&mut runtime, "load(");
    for (side, y) in [(SideId::Before, 120.0), (SideId::After, 240.0)] {
        let index = step(&runtime, side, true);
        select(&mut runtime, side, index);
        set_scroll(&mut runtime, side, y);
    }
    let cameras = [
        serde_json::to_value(&session(&runtime, SideId::Before).camera).unwrap(),
        serde_json::to_value(&session(&runtime, SideId::After).camera).unwrap(),
    ];
    click(&mut runtime, "comparison-after-target-0-0");
    settle(&mut runtime);
    assert_ne!(
        runtime
            .state
            .flow_comparison
            .current
            .as_ref()
            .unwrap()
            .selection
            .symbol,
        symbol
    );
    click(&mut runtime, "comparison-back");
    for _ in 0..4 {
        runtime.frame(1.0 / 60.0);
    }
    let pair = runtime.state.flow_comparison.current.as_ref().unwrap();
    assert_eq!(pair.selection.symbol, symbol);
    assert_eq!(pair.active_source, SideId::After);
    assert!((rendered_scroll(&runtime, SideId::After) - 240.0).abs() < 1.0);
    for side in [SideId::Before, SideId::After] {
        assert_eq!(
            serde_json::to_value(&session(&runtime, side).camera).unwrap(),
            cameras[side.index()]
        );
        assert!(session(&runtime, side).selected.is_some());
    }
    click(&mut runtime, "comparison-source-before");
    assert!((rendered_scroll(&runtime, SideId::Before) - 120.0).abs() < 1.0);
}
#[test]
fn back_keeps_saved_camera_when_the_restored_pane_is_measured_again() {
    let (mut ui, _project) = runtime(prefixed("before", 20), prefixed("after", 30), true);
    load(&mut ui);
    choose(&mut ui, "load(");
    let index = step(&ui, SideId::After, true);
    select(&mut ui, SideId::After, index);
    ui.update(|state, _| {
        let Side::Ready(side) = state
            .flow_comparison
            .current
            .as_mut()
            .unwrap()
            .side_mut(SideId::After)
        else {
            panic!()
        };
        side.session.camera.zoom = 1.7;
        side.session.camera.pan = [42.0, 31.0];
    });
    let saved = serde_json::to_value(&session(&ui, SideId::After).camera).unwrap();
    click(&mut ui, "comparison-after-target-0-0");
    settle(&mut ui);
    click(&mut ui, "comparison-back");
    ui.set_viewport((1080.0, 700.0), 1.0);
    ui.frame(1.0 / 60.0);
    ui.frame(1.0 / 60.0);
    assert_eq!(
        serde_json::to_value(&session(&ui, SideId::After).camera).unwrap(),
        saved
    );
}
#[test]
fn unlocated_comparison_step_keeps_source_position_and_target_metadata_is_endpoint_owned() {
    use codecallnavigator::flow::{SourceSpan, Statement, StatementKind};
    let mut before = prefixed("before", 200);
    let mut after = before.clone();
    before.flow_inventory.as_mut().unwrap().functions[0]
        .body
        .insert(
            0,
            Statement {
                span: SourceSpan { start: 0, end: 0 },
                kind: StatementKind::Unsupported {
                    text: "derived fixture".into(),
                    reason: "synthetic test step".into(),
                },
            },
        );
    after.nodes[1].body_unavailable = true;
    let (mut ui, _project) = runtime(before, after, true);
    load(&mut ui);
    choose(&mut ui, "load(");
    assert!(ui.find("comparison-before-partial").is_some());
    ui.update(|state, _| {
        let mut decoy = crate::flow_scenes::data();
        for node in &mut decoy.nodes {
            node.signature = "DECOY current graph".into();
        }
        state.graph = Some(std::sync::Arc::new(
            codecallnavigator::graph::GraphIndex::new(std::sync::Arc::new(decoy)).unwrap(),
        ));
    });
    set_scroll(&mut ui, SideId::Before, 120.0);
    let index = session(&ui, SideId::Before)
        .ready
        .as_ref()
        .unwrap()
        .chart
        .steps
        .iter()
        .position(|s| s.label == "derived fixture")
        .unwrap();
    select(&mut ui, SideId::Before, index);
    assert!((rendered_scroll(&ui, SideId::Before) - 120.0).abs() < 1.0);
    assert!(ui.find("comparison-before-source-unlocated").is_some());
    let rowkeys = address(SideId::Before).keys(session(&ui, SideId::Before));
    assert!(!ui.elements().iter().any(|el| {
        el.key
            .as_ref()
            .is_some_and(|key| key.starts_with(rowkeys.line(0).trim_end_matches('0')))
            && el.fill.a > 0.01
    }));
    let index = step(&ui, SideId::After, true);
    select(&mut ui, SideId::After, index);
    let target = ui
        .elements()
        .into_iter()
        .find(|el| el.key.as_deref() == Some("comparison-after-target-0-0"))
        .unwrap();
    assert!(!target.focusable);
    assert!(target.help.unwrap().contains("No body"));
    assert!(!target.label.unwrap().contains("DECOY"));
}
