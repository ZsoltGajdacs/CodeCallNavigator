//! Count ownership from queueing through completion, beyond Latest's publication lifetime.
use rune::Waker;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
#[derive(Default)]
pub(crate) struct Workers {
    active: AtomicUsize,
    waker: Mutex<Option<Waker>>,
}
pub(crate) struct Lease {
    workers: Arc<Workers>,
}
impl Workers {
    pub fn lease(self: &Arc<Self>) -> Lease {
        self.active.fetch_add(1, Ordering::AcqRel);
        Lease {
            workers: self.clone(),
        }
    }
    pub fn stopped(&self) -> bool {
        self.active.load(Ordering::Acquire) == 0
    }
    pub fn wake(&self) {
        let wake = self
            .waker
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        if let Some(wake) = wake {
            wake();
        }
    }
    pub fn set_waker(&self, waker: Waker) {
        *self.waker.lock().unwrap() = Some(waker);
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if self.workers.active.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.workers.wake();
        }
    }
}
/// Payloads (including an idle CLI connection) must drop before shutdown ownership.
pub(crate) struct Tracked<T> {
    pub value: T,
    lease: Lease,
}
impl Workers {
    pub fn track<T>(self: &Arc<Self>, value: T) -> Tracked<T> {
        Tracked {
            value,
            lease: self.lease(),
        }
    }
}
impl<T> Tracked<T> {
    pub fn map<U>(self, work: impl FnOnce(T) -> U) -> Tracked<U> {
        let Self { value, lease } = self;
        let value = work(value);
        Tracked { value, lease }
    }
}
