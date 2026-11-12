//! External process failures cannot leak late replies into a restarted CLI session.
use super::*;
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    sync::Arc,
    time::{Duration, Instant},
};

fn executable(root: &Path, source: &str) -> Runtime {
    let engine = root.join("query-engine");
    fs::write(&engine, source).unwrap();
    fs::set_permissions(&engine, fs::Permissions::from_mode(0o755)).unwrap();
    Runtime {
        engine,
        resources: root.into(),
        dot: Default::default(),
    }
}

#[test]
fn incompatible_capabilities_are_rejected_before_query_session_start() {
    let root = tempfile::tempdir().unwrap();
    let runtime = executable(
        root.path(),
        r#"#!/usr/bin/env python3
import json, pathlib, sys
if sys.argv[1] == 'capabilities':
 print(json.dumps({'query_version':1,'operations':['snapshot','render_selection']}))
else:
 pathlib.Path(__file__).with_name('unexpected-query').write_text('started')
"#,
    );
    let context = QueryContext::for_project(&runtime, root.path().into());
    let error = context
        .request::<Value>(
            codecallnavigator::query::Query::Status,
            &AtomicBool::new(false),
        )
        .unwrap_err();
    assert!(
        error.to_string().contains("required desktop operation"),
        "{error:#}"
    );
    assert!(!root.path().join("unexpected-query").exists());
}

#[test]
fn cancelled_query_reaps_process_and_restart_cannot_receive_its_late_reply() {
    let root = tempfile::tempdir().unwrap();
    let runtime = executable(
        root.path(),
        r#"#!/usr/bin/env python3
import json, os, pathlib, sys, time
marker=pathlib.Path(__file__).with_name('started')
if sys.argv[1] == 'capabilities':
 print(json.dumps({'query_version':1,'flow_languages':['java','typescript'],'operations':['snapshot','render_selection','inspect','architecture','live_source','explorer','flow','search','status']}))
 sys.exit(0)
for line in sys.stdin:
 json.loads(line)
 if not marker.exists():
  marker.write_text(str(os.getpid()))
  time.sleep(10)
  token='late'
 else:
  token='fresh'
 print(json.dumps({'version':1,'generation':None,'freshness':None,'error':None,'result':{'token':token}}),flush=True)
"#,
    );
    let context = QueryContext::for_project(&runtime, root.path().into());
    let cancel = Arc::new(AtomicBool::new(false));
    let trigger = cancel.clone();
    let marker = root.path().join("started");
    let cancellation = std::thread::spawn(move || {
        let started = Instant::now();
        while !marker.exists() && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(5));
        }
        trigger.store(true, Ordering::Release);
    });
    assert!(
        context
            .request::<Value>(codecallnavigator::query::Query::Status, &cancel)
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
    cancellation.join().unwrap();
    let pid: i32 = fs::read_to_string(root.path().join("started"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "Cancelled query process survived"
    );
    let result: Value = context
        .request(
            codecallnavigator::query::Query::Status,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(result, json!({"token":"fresh"}));
}

#[test]
fn expired_and_precancelled_requests_fail_before_io() {
    assert!(
        super::query_process::active(
            &AtomicBool::new(false),
            Instant::now() - Duration::from_secs(121)
        )
        .unwrap_err()
        .to_string()
        .contains("timed out")
    );
    assert!(
        super::query_process::active(&AtomicBool::new(true), Instant::now())
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
}

#[test]
fn valid_capability_json_from_failed_process_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let runtime = executable(
        root.path(),
        r#"#!/usr/bin/env python3
import json, sys
print(json.dumps({'query_version':1,'flow_languages':['java','typescript'],'operations':['snapshot','render_selection','inspect','architecture','live_source','explorer','flow','search','status']}))
sys.exit(7)
"#,
    );
    let context = QueryContext::for_project(&runtime, root.path().into());
    let error = context
        .request::<Value>(
            codecallnavigator::query::Query::Status,
            &AtomicBool::new(false),
        )
        .unwrap_err();
    assert!(
        error.to_string().contains("capability command failed"),
        "{error:#}"
    );
}

#[test]
fn blocked_stdin_write_is_cancellable_and_restarts_without_a_stale_request() {
    let root = tempfile::tempdir().unwrap();
    let runtime = executable(
        root.path(),
        r#"#!/usr/bin/env python3
import json, os, pathlib, sys, time
marker=pathlib.Path(__file__).with_name('write-blocked')
if sys.argv[1] == 'capabilities':
 print(json.dumps({'query_version':1,'flow_languages':['java','typescript'],'operations':['snapshot','render_selection','inspect','architecture','live_source','explorer','flow','search','status']}))
 sys.exit(0)
if not marker.exists():
 marker.write_text(str(os.getpid()))
 time.sleep(10)
else:
 for line in sys.stdin:
  print(json.dumps({'version':1,'generation':None,'freshness':None,'error':None,'result':{'token':'fresh'}}),flush=True)
"#,
    );
    let context = QueryContext::for_project(&runtime, root.path().into());
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_context = context.clone();
    let worker_cancel = cancel.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let query = codecallnavigator::query::Query::Search {
            text: "x".repeat(1024 * 1024),
            category: codecallnavigator::query::Category::All,
            offset: 0,
            limit: 50,
        };
        let _ = sender.send(worker_context.request::<Value>(query, &worker_cancel));
    });
    let marker = root.path().join("write-blocked");
    let started = Instant::now();
    while !marker.exists() && started.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(5));
    }
    let pid: i32 = fs::read_to_string(marker).unwrap().parse().unwrap();
    cancel.store(true, Ordering::Release);
    let result = receiver.recv_timeout(Duration::from_secs(2));
    if result.is_err() {
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    worker.join().unwrap();
    assert!(
        result
            .expect("Blocked query stdin ignored cancellation")
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
    let result: Value = context
        .request(
            codecallnavigator::query::Query::Status,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(result, json!({"token":"fresh"}));
}

#[test]
fn explorer_rejects_a_more_page_that_makes_no_progress() {
    let root = tempfile::tempdir().unwrap();
    let runtime = executable(
        root.path(),
        r#"#!/usr/bin/env python3
import json, pathlib, sys
if sys.argv[1] == 'capabilities':
 print(json.dumps({'query_version':1,'flow_languages':['java','typescript'],'operations':['snapshot','render_selection','inspect','architecture','live_source','explorer','flow','search','status']}))
 sys.exit(0)
for index, line in enumerate(sys.stdin):
 request=json.loads(line)
 if index:
  pathlib.Path(__file__).with_name('repeated-offset').write_text('requested')
  sys.exit(9)
 print(json.dumps({'version':1,'generation':None,'freshness':None,'error':None,'result':{'origin':'live','path':request['query']['path'],'entries':[],'more':True}}),flush=True)
"#,
    );
    let context = QueryContext::for_project(&runtime, root.path().into());
    let backend = crate::graph_queries::directory::DirectoryQueries::Cli(context);
    let error = backend
        .read(root.path().to_str().unwrap(), &AtomicBool::new(false))
        .unwrap_err();
    assert!(
        error.to_string().contains("pagination made no progress"),
        "{error:#}"
    );
    assert!(!root.path().join("repeated-offset").exists());
}

#[test]
fn fresh_analysis_provenance_overrides_portable_snapshot_metadata() {
    let root = tempfile::tempdir().unwrap();
    let graph = codecallnavigator::model::GraphData {
        engine_versions: "portable-record".into(),
        ..Default::default()
    };
    fs::write(
        root.path().join("snapshot.json"),
        serde_json::to_vec(&graph).unwrap(),
    )
    .unwrap();
    let runtime = executable(
        root.path(),
        r#"#!/usr/bin/env python3
import json, pathlib, sys
if sys.argv[1] == 'capabilities':
 print(json.dumps({'query_version':1,'flow_languages':['java','typescript'],'operations':['snapshot','render_selection','inspect','architecture','live_source','explorer','flow','search','status']}))
 sys.exit(0)
if sys.argv[1] == 'serve-once':
 request=json.loads(sys.stdin.readline())
 print(json.dumps({'event':'hello','version':1}),flush=True)
 print(json.dumps({'event':'complete','roots':[['repo',request['project']]],'digest':'a'*64,'fingerprints':[],'engine_versions':'fresh-runtime-record','metrics':{'stages':{},'counters':{}}}),flush=True)
 sys.exit(0)
for line in sys.stdin:
 graph=json.loads(pathlib.Path(__file__).with_name('snapshot.json').read_text())
 print(json.dumps({'version':1,'generation':'a'*64,'freshness':None,'error':None,'result':graph}),flush=True)
"#,
    );
    let snapshot = analyze_snapshot(
        &runtime,
        Analysis {
            project: root.path().into(),
            targets: Vec::new(),
        },
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert_eq!(snapshot.graph.data.engine_versions, "fresh-runtime-record");
}
