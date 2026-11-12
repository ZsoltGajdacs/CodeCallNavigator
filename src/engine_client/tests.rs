use super::*;
use codecallnavigator::artifact::{Artifact, GRAPH_PATH};
use std::fs;
use std::{
    os::unix::fs::symlink,
    sync::Arc,
    time::{Duration, Instant},
};

#[test]
fn conversation_rejects_mismatched_and_out_of_order_events() {
    assert!(
        Conversation::default()
            .receive(Event::Hello {
                version: VERSION + 1
            })
            .is_err()
    );
    assert!(
        Conversation::default()
            .receive(Event::Discovered {
                targets: Vec::new()
            })
            .is_err()
    );
    let mut conversation = Conversation::default();
    conversation
        .receive(Event::Hello { version: VERSION })
        .unwrap();
    conversation
        .receive(Event::Discovered {
            targets: Vec::new(),
        })
        .unwrap();
    assert!(
        conversation
            .receive(Event::Progress {
                message: "late".into()
            })
            .is_err()
    );
}

#[test]
fn terminal_result_requires_prompt_exit_and_reaps_engine() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("engine");
    let marker = root.path().join("pid");
    fs::write(
        &script,
        r#"#!/usr/bin/env python3
import json, os, pathlib, sys, time
sys.stdin.readline()
pathlib.Path(__file__).with_name('pid').write_text(str(os.getpid()))
print(json.dumps({'event': 'hello', 'version': 1}), flush=True)
print(json.dumps({'event': 'discovered', 'targets': []}), flush=True)
time.sleep(12)
"#,
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let runtime = Runtime {
        engine: script,
        resources: root.path().to_owned(),
        dot: Default::default(),
    };
    let request = Request::Discover {
        version: VERSION,
        project: root.path().to_owned(),
    };
    let started = Instant::now();
    let result = exchange(&runtime, &request, &AtomicBool::new(false), &|_| {});
    assert!(
        result.is_err(),
        "Terminal event accepted without prompt engine exit"
    );
    assert!(started.elapsed() < Duration::from_secs(8));
    let pid: i32 = fs::read_to_string(marker).unwrap().parse().unwrap();
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "Engine was not reaped after terminal timeout"
    );
}

#[test]
fn crashed_engine_with_inherited_output_pipe_is_bounded_and_stops_its_group() {
    failed_engine_child("subprocess.Popen(['sleep', '8'])", Duration::from_secs(7));
}

#[test]
#[ignore = "requires process-table access outside the macOS sandbox for detached descendant cleanup"]
fn forced_cancellation_reaps_engine_and_its_separate_child_group() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("engine");
    let marker = root.path().join("child");
    fs::write(
        &script,
        r#"#!/usr/bin/env python3
import json, pathlib, signal, subprocess, sys, time
sys.stdin.readline()
signal.signal(signal.SIGTERM, signal.SIG_IGN)
child = subprocess.Popen(['sleep', '30'], start_new_session=True)
pathlib.Path(__file__).with_name('child').write_text(str(child.pid))
print(json.dumps({'event': 'hello', 'version': 1}), flush=True)
while True: time.sleep(1)
"#,
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let runtime = Runtime {
        engine: script,
        resources: root.path().to_owned(),
        dot: Default::default(),
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let trigger = cancel.clone();
    let trigger_marker = marker.clone();
    let cancellation = std::thread::spawn(move || {
        let started = Instant::now();
        while !trigger_marker.exists() && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(20));
        }
        trigger.store(true, Ordering::Relaxed);
    });
    let request = Request::Discover {
        version: VERSION,
        project: root.path().to_owned(),
    };
    let started = Instant::now();
    assert!(exchange(&runtime, &request, &cancel, &|_| {}).is_err());
    cancellation.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(9));
    let pid: i32 = fs::read_to_string(marker).unwrap().parse().unwrap();
    let started = Instant::now();
    while unsafe { libc::kill(pid, 0) } == 0 && started.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "Detached engine child survived forced cancellation"
    );
}

fn linked_typescript_project(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let project = root.canonicalize().unwrap().join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    let linked = root.canonicalize().unwrap().join("linked.ts");
    fs::write(&linked, "export function linkedLeaf() { return 2; }").unwrap();
    fs::write(project.join("tsconfig.json"), r#"{"compilerOptions":{"strict":true,"target":"ES2022","module":"NodeNext","moduleResolution":"NodeNext","skipLibCheck":true},"include":["src/**/*.ts","../linked.ts"]}"#).unwrap();
    fs::write(
        project.join("src/main.ts"),
        "import {linkedLeaf} from '../../linked.js'; export function leaf() { return linkedLeaf(); } export function start() { return leaf(); }",
    )
    .unwrap();
    let modules = std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../PacificFateMapEditor/node_modules")
        });
    symlink(modules, project.join("node_modules")).unwrap();
    (project, linked)
}

#[test]
#[ignore = "requires built standalone engine and installed TypeScript/React/Next dependencies"]
fn real_engine_roundtrip_publishes_and_preserves_graph_on_failure() {
    let root = tempfile::tempdir().unwrap();
    let (project, linked) = linked_typescript_project(root.path());
    let runtime = Runtime::discover().unwrap();
    let targets = discover(&runtime, &project).unwrap();
    assert_eq!(targets.len(), 1);
    let metrics = codecallnavigator::metrics::Metrics::default();
    let _scope = metrics.enter();
    let snapshot = analyze_snapshot(
        &runtime,
        Analysis {
            project: project.clone(),
            targets: targets.clone(),
        },
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    let graph = &snapshot.graph.data;
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.label == "leaf" && Path::new(&node.file).is_absolute())
    );
    let status = snapshot
        .context
        .response(
            codecallnavigator::query::Query::Status,
            &AtomicBool::new(false),
        )
        .unwrap();
    assert_eq!(status.generation, snapshot.context.descriptor.generation);
    assert_eq!(status.freshness.unwrap()["contract_valid"], true);
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.label == "linkedLeaf" && Path::new(&node.file) == linked)
    );
    assert!(metrics.report().stages.contains_key("analysis"));
    let before = fs::read(project.join(GRAPH_PATH)).unwrap();
    Artifact::from_bytes(&before).unwrap();
    fs::write(
        project.join("src/main.ts"),
        "const invalid: number = 'broken';",
    )
    .unwrap();
    assert!(
        analyze(
            &runtime,
            Analysis {
                project: project.clone(),
                targets
            },
            &AtomicBool::new(false),
            &|_| {}
        )
        .is_err()
    );
    assert_eq!(fs::read(project.join(GRAPH_PATH)).unwrap(), before);
}

#[test]
#[ignore = "requires built standalone engine and JDK 25"]
fn real_engine_cancellation_reaps_build_and_preserves_previous_artifact() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let project = root.path().canonicalize().unwrap();
    fs::write(project.join("settings.gradle"), "rootProject.name='cancel'").unwrap();
    fs::write(project.join("build.gradle"), "plugins { id 'java' }").unwrap();
    let marker = project.join("running");
    fs::write(
        project.join("gradlew"),
        "#!/bin/sh\necho $$ > running\nsleep 30\n",
    )
    .unwrap();
    fs::set_permissions(project.join("gradlew"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::create_dir(project.join(".codecallnavigator")).unwrap();
    fs::write(project.join(GRAPH_PATH), b"previous graph").unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let trigger = cancel.clone();
    let trigger_marker = marker.clone();
    let cancellation = std::thread::spawn(move || {
        let started = Instant::now();
        while !trigger_marker.exists() && started.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(20));
        }
        trigger.store(true, Ordering::Relaxed);
    });
    let target = BuildTarget {
        path: project.clone(),
        language: "java".into(),
        kind: "gradle".into(),
        label: "cancel".into(),
    };
    let result = analyze(
        &Runtime::discover().unwrap(),
        Analysis {
            project: project.clone(),
            targets: vec![target],
        },
        &cancel,
        &|_| {},
    );
    cancellation.join().unwrap();
    assert!(result.is_err());
    let pid: i32 = fs::read_to_string(marker)
        .expect("Build must actually start")
        .trim()
        .parse()
        .unwrap();
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "Cancelled build survived engine exit"
    );
    assert_eq!(
        fs::read(project.join(GRAPH_PATH)).unwrap(),
        b"previous graph"
    );
}

#[test]
fn crashed_engine_with_closed_output_stops_remaining_owned_group() {
    failed_engine_child(
        "subprocess.Popen(['sleep', '30'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)",
        Duration::from_secs(2),
    );
}

#[test]
fn crashed_engine_with_chatty_inherited_output_still_observes_leader_exit() {
    let child = r#"subprocess.Popen([sys.executable, '-u', '-c',
"import json,time\nfor i in range(1600):\n print(json.dumps({'event':'progress','message':'inherited'}),flush=True);time.sleep(.005)"])"#;
    failed_engine_child(child, Duration::from_secs(7));
}

fn failed_engine_child(child_expression: &str, bound: Duration) {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("engine");
    let marker = root.path().join("child");
    let source = r#"#!/usr/bin/env python3
import json, pathlib, subprocess, sys
sys.stdin.readline()
child = CHILD_EXPRESSION
pathlib.Path(__file__).with_name('child').write_text(str(child.pid))
print(json.dumps({'event': 'hello', 'version': 1}), flush=True)
sys.exit(7)
"#
    .replace("CHILD_EXPRESSION", child_expression);
    fs::write(&script, source).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let runtime = Runtime {
        engine: script,
        resources: root.path().into(),
        dot: Default::default(),
    };
    let request = Request::Discover {
        version: VERSION,
        project: root.path().into(),
    };
    let started = Instant::now();
    let result = exchange(&runtime, &request, &AtomicBool::new(false), &|_| {});
    assert!(result.is_err());
    let elapsed = started.elapsed();
    let pid: i32 = fs::read_to_string(marker).unwrap().parse().unwrap();
    let cleanup = Instant::now();
    while unsafe { libc::kill(pid, 0) } == 0 && cleanup.elapsed() < Duration::from_secs(1) {
        std::thread::sleep(Duration::from_millis(20));
    }
    let alive = unsafe { libc::kill(pid, 0) } == 0;
    if alive {
        unsafe {
            libc::kill(pid, libc::SIGKILL);
        }
    }
    assert!(
        elapsed < bound,
        "Waited for an exited engine's child: {elapsed:?}"
    );
    assert!(
        !alive,
        "Engine-owned child survived its leader's failed exit"
    );
}
