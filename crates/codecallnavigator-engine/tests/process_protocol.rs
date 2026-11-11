//! Exercise the actual executable and process lifetime, independently of GUI code.
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn engine() -> Child {
    Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("serve-once")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn hello(child: &mut Child) {
    let mut text = String::new();
    BufReader::new(child.stdout.as_mut().unwrap())
        .read_line(&mut text)
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&text).unwrap(),
        json!({"event":"hello","version":1})
    );
}

fn send(child: &mut Child, request: &Value) {
    writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    child.stdin.as_mut().unwrap().flush().unwrap();
}

fn wait(child: &mut Child) -> i32 {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status.code().unwrap();
        }
        if start.elapsed() > Duration::from_secs(5) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("engine did not stop within five seconds");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn initial_request_eof_and_malformed_protocol_fail_without_replacing_graph() {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join(".codecallnavigator")).unwrap();
    let graph = project.path().join(".codecallnavigator/graph.json");
    fs::write(&graph, "previous successful graph").unwrap();
    let requests = [
        None,
        Some("not JSON".to_owned()),
        Some(
            json!({
                "command":"discover", "version":99, "project":project.path()
            })
            .to_string(),
        ),
    ];
    for request in requests {
        let mut child = engine();
        hello(&mut child);
        if let Some(request) = request {
            writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
        }
        drop(child.stdin.take());
        assert_eq!(wait(&mut child), 2);
        let output = child.wait_with_output().unwrap();
        let events: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(events.last().unwrap()["event"], "failed");
        assert_eq!(
            fs::read_to_string(&graph).unwrap(),
            "previous successful graph"
        );
    }
}

#[test]
fn sigterm_before_initial_request_is_observed_without_stdin_eof() {
    let mut child = engine();
    hello(&mut child);
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    assert_eq!(wait(&mut child), 2);
}

#[test]
fn invalid_roots_fail_before_acquiring_project_lock() {
    let project = tempfile::tempdir().unwrap();
    let mut child = engine();
    hello(&mut child);
    send(
        &mut child,
        &json!({"command":"analyze", "version":1, "project":project.path(),
        "roots":[["duplicate",project.path()],["duplicate",project.path()]], "targets":[]}),
    );
    assert_eq!(wait(&mut child), 2);
    assert!(!project.path().join(".codecallnavigator/graph.lock").exists());
}

fn slow_build(root: &Path) {
    fs::create_dir_all(root.join("jdk/bin")).unwrap();
    fs::write(root.join("jdk/bin/javac"), "").unwrap();
    fs::write(root.join("jdk/release"), "JAVA_VERSION=\"25\"\n").unwrap();
    fs::create_dir_all(root.join("resources/tools")).unwrap();
    fs::write(root.join("resources/tools/gradle-metadata.gradle"), "").unwrap();
    fs::write(root.join("build.gradle"), "").unwrap();
    let wrapper = root.join("gradlew");
    fs::write(&wrapper, "#!/bin/sh\nsleep 30 &\nchild=$!\nprintf '%s' \"$child\" > child.pid\ntrap 'kill \"$child\" 2>/dev/null; wait \"$child\" 2>/dev/null; exit 143' TERM INT\nwait \"$child\"\n").unwrap();
    fs::set_permissions(wrapper, fs::Permissions::from_mode(0o755)).unwrap();
}

struct EngineOutputReaders {
    stdout: thread::JoinHandle<Vec<u8>>,
    stderr: thread::JoinHandle<Vec<u8>>,
}

impl EngineOutputReaders {
    fn drain(child: &mut Child) -> Self {
        // Contextual failures can exceed pipe capacity; drain both pipes while waiting.
        Self {
            stdout: read_pipe(child.stdout.take().unwrap()),
            stderr: read_pipe(child.stderr.take().unwrap()),
        }
    }

    fn assert_cancelled(self) {
        let output = self.stdout.join().unwrap();
        let errors = self.stderr.join().unwrap();
        let events: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let failure = events.last().unwrap();
        assert_eq!(failure["event"], "failed");
        let reason = failure["message"].as_str().unwrap();
        assert!(
            reason.contains("cancelled"),
            "{failure}; stderr: {}",
            String::from_utf8_lossy(&errors)
        );
        assert!(!reason.contains("timed out"), "{failure}");
    }
}

fn read_pipe(mut pipe: impl Read + Send + 'static) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        pipe.read_to_end(&mut bytes).unwrap();
        bytes
    })
}

#[test]
fn analysis_eof_and_sigterm_reap_build_descendants_and_preserve_artifact() {
    for signal in [false, true] {
        let project = tempfile::tempdir().unwrap();
        slow_build(project.path());
        fs::create_dir(project.path().join(".codecallnavigator")).unwrap();
        let graph = project.path().join(".codecallnavigator/graph.json");
        fs::write(&graph, "previous successful graph").unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
            .arg("serve-once")
            .env("JAVA_HOME", project.path().join("jdk"))
            .env("CODECALLNAVIGATOR_RESOURCES", project.path().join("resources"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        hello(&mut child);
        let output = EngineOutputReaders::drain(&mut child);
        send(
            &mut child,
            &json!({"command":"analyze", "version":1, "project":project.path(), "roots":[],
            "targets":[{"path":project.path(),"language":"java","kind":"gradle","label":"slow"}]}),
        );
        let start = Instant::now();
        while !project.path().join("child.pid").exists() {
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "build never started"
            );
            thread::sleep(Duration::from_millis(10));
        }
        let descendant: i32 = fs::read_to_string(project.path().join("child.pid"))
            .unwrap()
            .parse()
            .unwrap();
        if signal {
            unsafe {
                libc::kill(child.id() as i32, libc::SIGTERM);
            }
        } else {
            drop(child.stdin.take());
        }
        assert_eq!(wait(&mut child), 2);
        output.assert_cancelled();
        assert_eq!(
            unsafe { libc::kill(descendant, 0) },
            -1,
            "build descendant survived cancellation"
        );
        assert_eq!(
            fs::read_to_string(graph).unwrap(),
            "previous successful graph"
        );
    }
}
