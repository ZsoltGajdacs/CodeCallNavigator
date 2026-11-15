//! Comparison-owned regression identities for the shared background lifetime owner.
pub(super) use crate::background_work::{Tracked, Workers};
#[cfg(test)]
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_queued_request_releases_its_shutdown_lease_when_dropped_before_start() {
        let workers = Arc::new(Workers::default());
        let lease = workers.lease();
        let queued = move || {
            let _lease = lease;
        };
        assert!(!workers.stopped());
        drop(queued);
        assert!(
            workers.stopped(),
            "Dropped queued work must release shutdown ownership"
        );
    }
}
#[cfg(test)]
mod ownership_tests {
    use super::*;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    fn stopped(workers: &Workers) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !workers.stopped() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn latest_cancellation_does_not_release_a_running_request_before_cleanup() {
        let workers = Arc::new(Workers::default());
        let latest = rune::Latest::default();
        let (started, ready) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let tracked = workers.track(());
        latest.request(tracked, move |tracked| {
            tracked.map(|()| {
                started.send(()).unwrap();
                wait.recv().unwrap();
                42
            })
        });
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        latest.cancel();
        assert!(!latest.is_pending());
        assert!(!workers.stopped());
        release.send(()).unwrap();
        stopped(&workers);
    }
    #[test]
    fn dropping_a_queued_latest_request_releases_ownership_without_running_it() {
        let workers = Arc::new(Workers::default());
        let latest = rune::Latest::<Tracked<()>>::default().debounce(Duration::from_secs(30));
        latest.request(workers.track(()), |_| panic!("Cancelled queued work ran"));
        latest.cancel();
        stopped(&workers);
    }
    #[test]
    fn a_completed_payload_is_destroyed_before_the_final_shutdown_wake() {
        struct Payload {
            workers: Arc<Workers>,
        }
        impl Drop for Payload {
            fn drop(&mut self) {
                assert!(!self.workers.stopped());
            }
        }
        let workers = Arc::new(Workers::default());
        let wakes = Arc::new(AtomicUsize::new(0));
        let count = wakes.clone();
        workers.set_waker(Arc::new(move || {
            count.fetch_add(1, Ordering::Release);
        }));
        let output = workers
            .track(Payload {
                workers: workers.clone(),
            })
            .map(|payload| payload);
        assert!(!workers.stopped());
        drop(output);
        assert!(workers.stopped());
        assert_eq!(wakes.load(Ordering::Acquire), 1);
    }
}
