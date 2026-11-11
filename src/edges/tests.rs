use super::*;
use crate::projection::{GraphView, ViewNode};

#[test]
fn dim_edges_multiply_existing_alpha_without_changing_relationship_styles() {
    use codecallnavigator::model::RelationshipClassification;
    for dark in [false, true] {
        let kit = theme::kit(dark);
        for (kind, resolution, color, dashed) in [
            (
                RelationshipKind::Call,
                TargetResolution::Direct,
                kit.c.text_muted.alpha(0.7),
                false,
            ),
            (
                RelationshipKind::Call,
                TargetResolution::Possible,
                theme::uncertain(&kit),
                true,
            ),
            (
                RelationshipKind::FunctionReference,
                TargetResolution::Direct,
                theme::callback(&kit),
                true,
            ),
            (
                RelationshipKind::FrameworkLifecycle,
                TargetResolution::Direct,
                theme::uncertain(&kit),
                true,
            ),
            (
                RelationshipKind::Other,
                TargetResolution::Direct,
                theme::uncertain(&kit),
                true,
            ),
        ] {
            let edge = ViewEdge {
                classification: RelationshipClassification { kind, resolution },
                count: 10,
                ..Default::default()
            };
            let bright = style(&edge, &kit, 1.0);
            let dim = style(&edge, &kit, 0.08);
            assert_eq!(bright.color, color);
            assert_eq!(bright.dashed, dashed);
            assert_eq!(dim.color, color.alpha(color.a * 0.08));
            assert_eq!(dim.width, bright.width);
            assert_eq!(dim.dashed, bright.dashed);
        }
    }
}

#[test]
fn rendering_paints_dim_strokes_and_arrows_before_incident_edges() {
    let mut state = State::new();
    state.jobs.boot.cancel();
    state.viewport = [300.0, 300.0];
    state.nav.camera.pan = [0.0, 0.0];
    state.graph_view = GraphView {
        nodes: ["anchor", "neighbor", "context"]
            .into_iter()
            .map(|key| ViewNode {
                key: key.into(),
                ..Default::default()
            })
            .collect(),
        edges: [(0, 1), (1, 2)]
            .into_iter()
            .map(|(from, to)| ViewEdge {
                from,
                to,
                classification: codecallnavigator::model::RelationshipClassification {
                    kind: RelationshipKind::Call,
                    resolution: TargetResolution::Direct,
                },
                points: vec![[10.0, 10.0 + from as f32 * 20.0], [100.0, 100.0]],
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    let kit = theme::kit(true);
    let emphasis = GraphEmphasis::new(&state.graph_view, Some("anchor"));
    let expected = stroke(
        Shape::new(),
        &state.graph_view.edges[1].points,
        Style {
            color: kit.c.text_muted.alpha(0.7 * 0.08),
            width: 1.2,
            dashed: false,
        },
    );
    let expected = stroke(
        expected,
        &state.graph_view.edges[0].points,
        Style {
            color: kit.c.text_muted.alpha(0.7),
            width: 1.2,
            dashed: false,
        },
    );
    assert_eq!(render(&state, &kit, &emphasis), expected);

    let normal = GraphEmphasis::new(&state.graph_view, None);
    let expected = state
        .graph_view
        .edges
        .iter()
        .fold(Shape::new(), |shape, edge| {
            stroke(
                shape,
                &edge.points,
                Style {
                    color: kit.c.text_muted.alpha(0.7),
                    width: 1.2,
                    dashed: false,
                },
            )
        });
    assert_eq!(render(&state, &kit, &normal), expected);
}

#[test]
fn culling_keeps_crossing_splines_and_drops_offscreen_geometry() {
    assert!(visible(&[[-20.0, 50.0], [150.0, 50.0]], [100.0, 100.0]));
    assert!(!visible(&[[-20.0, -50.0], [150.0, -50.0]], [100.0, 100.0]));
}

#[test]
fn straight_cubics_use_one_segment() {
    let controls = [[0.0, 0.0], [30.0, 0.0], [60.0, 0.0], [90.0, 0.0]];
    assert_eq!(spline_points(&controls), vec![controls[0], controls[3]]);
}

#[test]
fn straight_cubic_routes_match_line_routes_including_dashes_and_arrows() {
    let controls = [[0.0, 0.0], [30.0, 0.0], [60.0, 0.0], [90.0, 0.0]];
    let line = [controls[0], controls[3]];
    for dashed in [false, true] {
        let cubic_route = route(Shape::new(), &controls, Color::WHITE, dashed);
        let line_route = route(Shape::new(), &line, Color::WHITE, dashed);
        assert!(!cubic_route.is_empty());
        assert_eq!(cubic_route, line_route);
    }
}

#[test]
fn sampled_curves_stay_within_half_a_logical_screen_unit() {
    for controls in [
        [[0.0, 0.0], [10.0, 20.0], [30.0, 20.0], [40.0, 0.0]],
        [[0.0, 0.0], [10.0, 200.0], [30.0, -200.0], [40.0, 0.0]],
        [[0.0, 0.0], [100.0, 200.0], [-100.0, 200.0], [0.0, 0.0]],
        [[0.0, 0.0], [200.0, 0.0], [-100.0, 0.0], [50.0, 0.0]],
        [[0.0, 0.0], [100.0, 200.0], [100.0, 200.0], [100.0, 200.0]],
    ] {
        let polyline = spline_points(&controls);
        assert_eq!(polyline.first(), Some(&controls[0]));
        assert_eq!(polyline.last(), Some(&controls[3]));
        for step in 0..=2000 {
            let point = sample(controls, step as f32 / 2000.0);
            let distance = polyline
                .windows(2)
                .map(|segment| point_distance(point, segment))
                .fold(f32::INFINITY, f32::min);
            assert!(distance <= 0.501, "distance {distance} for {controls:?}");
        }
    }
}

#[test]
fn zoomed_out_curves_need_less_geometry() {
    let controls = [[0.0, 0.0], [10.0, 200.0], [30.0, 200.0], [40.0, 0.0]];
    let small = controls.map(|p| [p[0] * 0.08, p[1] * 0.08]);
    assert!(spline_points(&small).len() < spline_points(&controls).len());
}

#[test]
fn repeated_points_and_polyline_fallback_remain_valid() {
    assert_eq!(spline_points(&[]), Vec::<[f32; 2]>::new());
    assert_eq!(spline_points(&[[2.0, 3.0]]), vec![[2.0, 3.0]]);
    assert_eq!(spline_points(&[[2.0, 3.0]; 4]), vec![[2.0, 3.0]; 2]);
    let polyline = [[0.0, 0.0], [1.0, 10.0], [2.0, 0.0]];
    assert_eq!(spline_points(&polyline), polyline);
}

#[test]
fn joined_cubics_preserve_the_shared_endpoint() {
    let controls = [
        [0.0, 0.0],
        [10.0, 20.0],
        [30.0, 20.0],
        [40.0, 0.0],
        [50.0, -20.0],
        [70.0, -20.0],
        [80.0, 0.0],
    ];
    let points = spline_points(&controls);
    assert_eq!(points.first(), Some(&controls[0]));
    assert_eq!(points.last(), Some(&controls[6]));
    assert_eq!(
        points.iter().filter(|&&point| point == controls[3]).count(),
        1
    );
}

#[test]
fn arrow_direction_uses_original_tangent_and_skips_repeated_controls() {
    let controls = [[0.0, 0.0], [10.0, 0.0], [10.0, 20.0], [10.0, 20.0]];
    let (tip, angle) = geometry::arrow_direction(&controls).unwrap();
    assert_eq!(tip, controls[3]);
    assert!((angle - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
    assert!(geometry::arrow_direction(&[[1.0, 1.0]; 4]).is_none());
    assert!(geometry::arrow_direction(&[]).is_none());
}

#[test]
fn subdivision_work_is_bounded_for_pathological_geometry() {
    let controls = [[0.0, 0.0], [1.0e30, 1.0e30], [-1.0e30, 1.0e30], [0.0, 0.0]];
    let points = spline_points(&controls);
    assert!(points.len() <= 4097);
    assert!(points.iter().flatten().all(|number| number.is_finite()));
}

fn sample(points: [[f32; 2]; 4], t: f32) -> [f32; 2] {
    let u = 1.0 - t;
    let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    [0, 1].map(|axis| weights.iter().zip(points).map(|(w, p)| w * p[axis]).sum())
}

fn point_distance(point: [f32; 2], segment: &[[f32; 2]]) -> f32 {
    let dx = segment[1][0] - segment[0][0];
    let dy = segment[1][1] - segment[0][1];
    let length = dx * dx + dy * dy;
    let fraction = if length > 0.0 {
        (((point[0] - segment[0][0]) * dx + (point[1] - segment[0][1]) * dy) / length)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    (point[0] - segment[0][0] - fraction * dx).hypot(point[1] - segment[0][1] - fraction * dy)
}
