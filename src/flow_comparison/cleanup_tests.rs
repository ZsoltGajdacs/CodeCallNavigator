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

#[derive(Clone, Copy)]
enum QueryKind {
    Source,
    Catalog,
}

impl QueryKind {
    fn operation(self) -> &'static str {
        match self {
            Self::Source => "git_comparison_changes",
            Self::Catalog => "flow_comparison",
        }
    }

    fn pending(self, workspace: &Workspace) -> bool {
        match self {
            Self::Source => workspace.changes.pending(),
            Self::Catalog => workspace.catalog_pending(),
        }
    }

    fn published(self, workspace: &Workspace) -> bool {
        match self {
            Self::Source => workspace.changes.loaded,
            Self::Catalog => workspace.header().is_some(),
        }
    }
}

fn wrapper(
    root: &std::path::Path,
    real: &std::path::Path,
    pending: Pending,
    kind: QueryKind,
) -> std::path::PathBuf {
    let path = root.join("comparison-cli");
    let hold = matches!(pending, Pending::Running);
    let script = format!(
        r#"#!/usr/bin/env python3
import json,os,pathlib,signal,subprocess,sys,time
real={real:?}
if sys.argv[1]=='capabilities': os.execv(real,[real,'capabilities'])
signal.signal(signal.SIGTERM,signal.SIG_IGN)
for line in sys.stdin:
 if json.loads(line)['query']['kind']=={operation:?}:
  pathlib.Path(__file__).with_name('comparison.pid').write_text(str(os.getpid()))
  pathlib.Path(__file__).with_name('request.started').write_text('started')
  if {hold}: time.sleep(30)
 answer=subprocess.run([real,'query'],input=line,text=True,stdout=subprocess.PIPE,check=False)
 print(answer.stdout.strip(),flush=True)
time.sleep(30)
"#,
        real = real.to_string_lossy(),
        operation = kind.operation(),
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
    for kind in [QueryKind::Source, QueryKind::Catalog] {
        for pending in [Pending::Running, Pending::Unpolled, Pending::Published] {
            assert_reaped(kind, pending);
        }
    }
}

fn assert_reaped(kind: QueryKind, pending: Pending) {
    let root = tempfile::tempdir().unwrap();
    project(root.path());
    let mut state = state(root.path());
    let mut runtime = state.runtime.as_ref().unwrap().as_ref().clone();
    runtime.engine = wrapper(root.path(), &runtime.engine, pending, kind);
    state.runtime = Some(Arc::new(runtime));
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    let workspace = &mut state.flow_comparison;
    if matches!(kind, QueryKind::Catalog) {
        // Publish the source outcome to schedule the graph catalog, then stop
        // polling so a settled catalog can be closed before publication.
        let serial = workspace.catalog_clock.serial;
        until(|| {
            workspace.poll();
            workspace.catalog_clock.serial != serial
        });
    }
    until(|| root.path().join("request.started").exists());
    if !matches!(pending, Pending::Running) {
        until(|| !kind.pending(workspace));
    }
    if matches!(pending, Pending::Published) {
        workspace.poll();
        assert!(kind.published(workspace));
    } else {
        assert!(!kind.published(workspace));
    }

    let pid: i32 = fs::read_to_string(root.path().join("comparison.pid"))
        .unwrap()
        .parse()
        .unwrap();
    workspace.close();
    assert!(
        !workspace.stopped(),
        "Close must retain cleanup ownership after returning"
    );
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        0,
        "Owned CLI should still be undergoing asynchronous cleanup"
    );
    until(|| workspace.stopped());
    assert_ne!(
        unsafe { libc::kill(pid, 0) },
        0,
        "Stopped means the CLI was reaped"
    );
}
