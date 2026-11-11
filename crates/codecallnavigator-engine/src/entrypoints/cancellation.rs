use std::sync::atomic::{AtomicBool, Ordering};

pub static CANCELLED: AtomicBool = AtomicBool::new(false);

extern "C" fn cancel_signal(_: libc::c_int) {
    CANCELLED.store(true, Ordering::Relaxed);
}

pub fn install() {
    // A process handles one request. The signal handler only writes a lock-free
    // atomic; analyzer supervision observes it and reaps its child groups.
    unsafe {
        libc::signal(
            libc::SIGINT,
            cancel_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            cancel_signal as *const () as libc::sighandler_t,
        );
    }
}
