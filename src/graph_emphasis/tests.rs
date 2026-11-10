use super::*;
use crate::projection::{ViewEdge, ViewNode};

fn graph() -> GraphView {
    GraphView {
        nodes: ["caller", "center", "callee", "second-hop", "isolated"]
            .into_iter()
            .map(|key| ViewNode {
                key: key.into(),
                ..Default::default()
            })
            .collect(),
        edges: [(0, 1), (1, 2), (2, 3), (0, 2), (1, 1), (1, 2)]
            .into_iter()
            .map(|(from, to)| ViewEdge {
                from,
                to,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

#[test]
fn only_anchors_and_their_immediate_neighbors_are_bright() {
    let emphasis = GraphEmphasis::new(&graph(), Some("center"));
    assert_eq!(
        (0..5)
            .map(|node| emphasis.card_opacity(node))
            .collect::<Vec<_>>(),
        [1.0, 1.0, 1.0, 0.35, 0.35]
    );
    assert_eq!(
        (0..5)
            .map(|node| emphasis.is_anchor(node))
            .collect::<Vec<_>>(),
        [false, true, false, false, false]
    );
    // Second-hop and neighbor-to-neighbor links stay dim; loops and parallel
    // links touching the anchor retain emphasis.
    assert_eq!(
        (0..6)
            .map(|edge| emphasis.edge_opacity(edge))
            .collect::<Vec<_>>(),
        [1.0, 1.0, 0.08, 0.08, 1.0, 1.0]
    );
}

#[test]
fn visible_selection_overrides_focus_and_rebuilding_moves_emphasis() {
    let mut view = graph();
    view.focus_keys = vec!["center".into()];
    let emphasis = GraphEmphasis::new(&view, Some("second-hop"));
    assert!(!emphasis.is_anchor(1));
    assert!(emphasis.is_anchor(3));
    assert_eq!(emphasis.card_opacity(0), 0.35);
    assert_eq!(emphasis.card_opacity(1), 0.35);
    assert_eq!(emphasis.card_opacity(2), 1.0);
    assert_eq!(emphasis.edge_opacity(1), 0.08);
    assert_eq!(emphasis.edge_opacity(2), 1.0);
    let moved = GraphEmphasis::new(&view, Some("isolated"));
    assert!(moved.is_anchor(4));
    assert_eq!(moved.card_opacity(4), 1.0);
    assert!((0..6).all(|edge| moved.edge_opacity(edge) == 0.08));
}

#[test]
fn missing_selection_falls_back_to_all_visible_focus_anchors() {
    let mut view = graph();
    view.focus_keys = vec!["caller".into(), "second-hop".into(), "missing".into()];
    for selection in [None, Some("missing")] {
        let emphasis = GraphEmphasis::new(&view, selection);
        assert!(emphasis.is_anchor(0));
        assert!(emphasis.is_anchor(3));
        assert!(!emphasis.is_anchor(1));
        assert_eq!(emphasis.card_opacity(4), 0.35);
        assert_eq!(emphasis.edge_opacity(0), 1.0);
        assert_eq!(emphasis.edge_opacity(1), 0.08);
        assert_eq!(emphasis.edge_opacity(2), 1.0);
        assert_eq!(emphasis.edge_opacity(3), 1.0);
    }
}

#[test]
fn no_visible_anchor_restores_normal_brightness() {
    let mut view = graph();
    view.focus_keys = vec!["missing".into()];
    for selection in [None, Some("missing")] {
        let emphasis = GraphEmphasis::new(&view, selection);
        assert!((0..5).all(|node| !emphasis.is_anchor(node)));
        assert!((0..5).all(|node| emphasis.card_opacity(node) == 1.0));
        assert!((0..6).all(|edge| emphasis.edge_opacity(edge) == 1.0));
    }
    let empty = GraphEmphasis::new(&GraphView::default(), Some("missing"));
    assert!(!empty.active);
}
