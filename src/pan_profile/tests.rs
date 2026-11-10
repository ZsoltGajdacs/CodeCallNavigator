use super::driver::displacement;

#[test]
fn sustained_sweeps_keep_the_graph_near_its_saved_camera_and_return_home() {
    for drag in 0..3 {
        assert_eq!(displacement(drag, 0.0), [0.0, 0.0]);
        assert_eq!(displacement(drag, 10.0), [0.0, 0.0]);
        for step in 0..=1000 {
            let offset = displacement(drag, step as f64 / 100.0);
            assert!(offset[0].abs() <= 180.0);
            assert!(offset[1].abs() <= 90.0);
        }
    }
    assert_eq!(displacement(0, 2.5), [180.0, 0.0]);
    assert_eq!(displacement(1, 7.5), [-180.0, -90.0]);
    assert_eq!(displacement(2, 2.5), [-180.0, 90.0]);
}
