//! Public opt-in boundaries and owned monitor lifecycle.
use serde_json::{Value, json};
use std::{
    io::Write,
    process::{Command, Stdio},
};

#[test]
fn automatic_refresh_rejects_a_pinned_observation_without_touching_the_project() {
    let root = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .args(["observe", "--auto-refresh"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(child.stdin.take().unwrap(), "{}", json!({"version":1,"project":root.path(),"expected_generation":"old","query":{"kind":"freshness","observation_version":1}})).unwrap();
    let output = child.wait_with_output().unwrap();
    let frame: Value = serde_json::from_slice(&output.stdout)
        .expect("Automatic mode must report a structured invalid_request");
    assert_eq!(frame["kind"], "failed");
    assert_eq!(frame["error"]["code"], "invalid_request");
    assert!(!root.path().join(".codecallnavigator").exists());
}

fn wait_for_exit(child: &mut std::process::Child) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "Idle observer failed: {status}");
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("Idle observer did not exit within five seconds");
}

fn idle_observer() -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .args(["observe", "--auto-refresh"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

#[test]
fn automatic_refresh_eof_before_first_request_exits_successfully() {
    let mut child = idle_observer();
    drop(child.stdin.take());
    wait_for_exit(&mut child);
}

#[test]
fn automatic_refresh_sigterm_before_first_request_exits_successfully() {
    let mut child = idle_observer();
    // Leave stdin open: this tests interrupted initial request waiting, not EOF.
    wait_for_idle_threads(&mut child);
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGTERM) }, 0);
    wait_for_exit(&mut child);
}

fn wait_for_idle_threads(child: &mut std::process::Child) {
    // Main + signal bridge + stdin reader + stdout writer prove that startup
    // passed signal installation while no request bytes have been supplied.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if thread_count(child.id()).is_some_and(|count| count >= 4) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("Observer did not initialize its idle transport threads");
}

#[cfg(target_os = "macos")]
fn thread_count(pid: u32) -> Option<usize> {
    let mut info = std::mem::MaybeUninit::<libc::proc_taskinfo>::uninit();
    let size = std::mem::size_of::<libc::proc_taskinfo>() as i32;
    // proc_pidinfo initializes this task-info value only on an exact-sized read.
    let count = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDTASKINFO,
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    (count == size).then(|| unsafe { info.assume_init() }.pti_threadnum as usize)
}

#[cfg(not(target_os = "macos"))]
fn thread_count(pid: u32) -> Option<usize> {
    std::fs::read_dir(format!("/proc/{pid}/task"))
        .ok()
        .map(Iterator::count)
}
