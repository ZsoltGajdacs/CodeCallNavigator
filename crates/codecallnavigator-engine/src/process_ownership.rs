//! Adapter-specific shutdown policy; shared descendant identities live in core.
use std::{process::Child, time::Duration};

/// Existing adapter group shutdown; the adapter owns this child's process group.
pub fn stop_group(child: &mut Child) {
    let group = -(child.id() as i32);
    unsafe {
        libc::kill(group, libc::SIGTERM);
    }
    for _ in 0..20 {
        if child.try_wait().ok().flatten().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    unsafe {
        libc::kill(group, libc::SIGKILL);
    }
    let _ = child.wait();
}
