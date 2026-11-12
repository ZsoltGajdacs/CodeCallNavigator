//! Selection emphasis through production class and method gestures.
use super::*;

fn assert_cards(runtime: &Runtime<State>, expected: &[(u32, f32)]) {
    let actual: Vec<_> = expected
        .iter()
        .map(|&(member, _)| {
            let node = runtime
                .state
                .graph_view
                .nodes
                .iter()
                .find(|node| node.member == Some(member))
                .expect("visible method");
            let key = format!("graph-card-{}", node.key);
            let opacity = runtime
                .elements()
                .into_iter()
                .find(|e| !e.exiting && e.key.as_deref() == Some(&key))
                .expect("visible card")
                .opacity;
            (member, opacity)
        })
        .collect();
    assert!(
        actual
            .iter()
            .zip(expected)
            .all(|(a, b)| a.0 == b.0 && (a.1 - b.1).abs() < 0.001),
        "actual {actual:?}, expected {expected:?}"
    );
}

#[test]
fn depth_expands_visibility_without_extending_selection_emphasis() {
    let mut runtime = runtime();
    runtime.state.focus_graph_method(1);
    frames(&mut runtime, 40);
    assert!(
        !runtime
            .state
            .graph_view
            .nodes
            .iter()
            .any(|n| n.member == Some(17))
    );
    runtime.state.set_depth(2);
    frames(&mut runtime, 40);
    runtime.state.fit_visible_graph();
    runtime.invalidate();
    frames(&mut runtime, 40);
    assert_cards(&runtime, &[(0, 1.0), (1, 1.0), (2, 1.0), (17, 0.35)]);

    let sink = method_control(&runtime, 17);
    click(&mut runtime, &sink);
    runtime.pointer_move(0.0, 0.0);
    frames(&mut runtime, 40);
    assert_eq!(runtime.state.selected(), Some(17));
    assert_cards(&runtime, &[(1, 0.35), (2, 1.0), (17, 1.0)]);
    runtime.state.visual_selection = None;
    runtime.state.graph_view.focus_keys.clear();
    runtime.invalidate();
    frames(&mut runtime, 40);
    assert_cards(&runtime, &[(1, 1.0), (17, 1.0)]);
}

fn assert_class_opacity(runtime: &Runtime<State>, label: &str, opacity: f32) {
    let node = runtime
        .state
        .graph_view
        .nodes
        .iter()
        .find(|n| n.label == label)
        .unwrap();
    let card_key = format!("graph-card-{}", node.key);
    let card = runtime
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(&card_key))
        .unwrap();
    assert!(
        (card.opacity - opacity).abs() < 0.001,
        "{label}: {} expected {opacity}; selection {:?}; focus {:?}",
        card.opacity,
        runtime.state.visual_selection,
        runtime.state.graph_view.focus_keys
    );
    let method_key = format!("graph-method-{}", node.methods[0].member);
    let method = runtime
        .elements()
        .into_iter()
        .find(|e| !e.exiting && e.key.as_deref() == Some(&method_key))
        .unwrap();
    assert!(
        (method.opacity - opacity).abs() < 0.001,
        "method row must inherit card opacity"
    );
}

#[test]
fn dimmed_class_remains_clickable_and_method_drilldown_applies_emphasis() {
    let mut runtime = runtime();
    classes(&mut runtime);
    runtime.pointer_move(0.0, 0.0);
    runtime.state.close_source();
    runtime.state.set_depth(2);
    frames(&mut runtime, 40);
    runtime.state.fit_visible_graph();
    runtime.invalidate();
    frames(&mut runtime, 40);
    let caller = node_control(&runtime, "Caller");
    click(&mut runtime, &caller);
    runtime.pointer_move(0.0, 0.0);
    frames(&mut runtime, 40);
    runtime.state.fit_visible_graph();
    runtime.invalidate();
    frames(&mut runtime, 40);
    assert_class_opacity(&runtime, "Sink", 0.35);

    let sink = node_control(&runtime, "Sink");
    click(&mut runtime, &sink);
    runtime.pointer_move(0.0, 0.0);
    frames(&mut runtime, 40);
    assert_class_opacity(&runtime, "Sink", 1.0);
    runtime.state.fit_visible_graph();
    runtime.invalidate();
    frames(&mut runtime, 40);
    let caller = node_control(&runtime, "Caller");
    click(&mut runtime, &caller);
    runtime.pointer_move(0.0, 0.0);
    frames(&mut runtime, 40);
    runtime.state.fit_visible_graph();
    runtime.invalidate();
    frames(&mut runtime, 40);
    // Its method remains actionable inside the dimmed class.
    click(&mut runtime, "graph-method-17");
    runtime.pointer_move(0.0, 0.0);
    frames(&mut runtime, 40);
    assert_eq!(
        runtime.state.nav.focus_target,
        Some(FocusTarget::Method("m17".into()))
    );
    assert_cards(&runtime, &[(17, 1.0), (2, 1.0), (1, 0.35)]);
}
