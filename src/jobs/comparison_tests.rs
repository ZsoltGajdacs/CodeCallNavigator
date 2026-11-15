//! Latest cancellation alone cannot authorize quitting while comparison cleanup is running.
use super::*;
use std::time::Instant;
#[test]
fn quitting_waits_for_comparison_cleanup_after_the_latest_request_is_cancelled() {
    let mut state = State::new();
    state.closing = true;
    let (started, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    crate::flow_comparison::tests::hold_request(&mut state.flow_comparison, started, wait);
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    state.flow_comparison.close();
    assert!(!state.flow_comparison.pending());
    assert!(!ready_to_quit(&state));
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !state.flow_comparison.stopped() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(ready_to_quit(&state));
}
