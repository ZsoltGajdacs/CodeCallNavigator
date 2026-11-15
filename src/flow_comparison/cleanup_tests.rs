//! Actual CLI wrappers make synchronous final-owner cleanup observable at close.
use super::tests::{project, state};
use super::*;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};
#[derive(Clone, Copy)]
enum Pending {
    Running,
    Unpolled,
    Published,
}
fn wrapper(root: &std::path::Path, real: &std::path::Path, pending: Pending) -> std::path::PathBuf {
    let path = root.join("comparison-cli");
    let hold = matches!(pending, Pending::Running);
    let script = format!(
        r#"#!/usr/bin/env python3
import os,pathlib,signal,subprocess,sys,time
real={real:?}
if sys.argv[1]=='capabilities': os.execv(real,[real,'capabilities'])
signal.signal(signal.SIGTERM,signal.SIG_IGN)
pathlib.Path(__file__).with_name('comparison.pid').write_text(str(os.getpid()))
for line in sys.stdin:
 pathlib.Path(__file__).with_name('request.started').write_text('started')
 if {hold}: time.sleep(30)
 answer=subprocess.run([real,'query'],input=line,text=True,stdout=subprocess.PIPE,check=False)
 print(answer.stdout.strip(),flush=True)
time.sleep(30)
"#,
        real = real.to_string_lossy(),
        hold = if hold { "True" } else { "False" }
    );
    fs::write(&path, script).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}
fn until(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "Comparison cleanup timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn close_reaps_running_settled_and_published_clients_off_the_ui_thread() {
    for pending in [Pending::Running, Pending::Unpolled, Pending::Published] {
        let root = tempfile::tempdir().unwrap();
        project(root.path());
        let mut state = state(root.path());
        let mut runtime = state.runtime.as_ref().unwrap().as_ref().clone();
        runtime.engine = wrapper(root.path(), &runtime.engine, pending);
        state.runtime = Some(Arc::new(runtime));
        state
            .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
            .unwrap();
        until(|| root.path().join("request.started").exists());
        if !matches!(pending, Pending::Running) {
            until(|| !state.flow_comparison.catalog_job.is_pending());
        }
        if matches!(pending, Pending::Published) {
            state.flow_comparison.poll();
            assert!(state.flow_comparison.header().is_some());
        }
        let pid: i32 = fs::read_to_string(root.path().join("comparison.pid"))
            .unwrap()
            .parse()
            .unwrap();
        state.flow_comparison.close();
        assert!(
            !state.flow_comparison.stopped(),
            "Close must retain cleanup ownership after returning"
        );
        assert_eq!(
            unsafe { libc::kill(pid, 0) },
            0,
            "Owned CLI should still be undergoing asynchronous cleanup"
        );
        until(|| state.flow_comparison.stopped());
        assert_ne!(
            unsafe { libc::kill(pid, 0) },
            0,
            "Stopped means the CLI was reaped"
        );
    }
}
