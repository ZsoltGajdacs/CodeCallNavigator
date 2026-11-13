//! Public freshness queries and streams over real saved filesystem states.
use codecallnavigator_core::{artifact, fingerprint, model::GraphData};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn publish(root: &Path) {
    fs::write(root.join("app.ts"), "export const value = 1;").unwrap();
    let roots = [("repo".into(), root.into())];
    let fingerprints = fingerprint::capture_excluding(
        [root.into(), root.join("app.ts"), root.join("missing.ts")],
        &[],
    )
    .unwrap();
    let mut graph = artifact::canonical(&GraphData::default(), &roots).unwrap();
    graph.graph["analysis"] = serde_json::to_value(
        artifact::provenance::build(&fingerprints, &[], "{}", &roots).unwrap(),
    )
    .unwrap();
    fs::create_dir(root.join(".codecallnavigator")).unwrap();
    fs::write(root.join(artifact::GRAPH_PATH), graph.bytes().unwrap()).unwrap();
}

fn request(root: &Path, operation: Value) -> (i32, Value) {
    raw_request(json!({"version":1,"project":root,"query":operation}))
}

fn raw_request(request: Value) -> (i32, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("query")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

fn observation(root: &Path) -> Value {
    let (code, response) = request(root, json!({"kind":"freshness","observation_version":1}));
    assert_eq!(code, 0, "{response}");
    response["result"].clone()
}

#[test]
fn freshness_revision_tracks_stale_repairs_and_new_typescript_file_contents() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path());
    let initial = observation(root.path());
    assert_eq!(initial["input_status"], "match");
    let generation = initial["generation"].clone();
    fs::write(root.path().join("app.ts"), "export const value = ").unwrap();
    let invalid = observation(root.path());
    fs::write(root.path().join("app.ts"), "export const value = 2;").unwrap();
    let repaired = observation(root.path());
    assert_eq!(invalid["input_status"], "stale");
    assert_eq!(repaired["input_status"], "stale");
    assert_ne!(invalid["input_revision"], repaired["input_revision"]);
    fs::write(root.path().join("new.ts"), "export const added = ").unwrap();
    let added = observation(root.path());
    fs::write(root.path().join("new.ts"), "export const added = 3;").unwrap();
    let fixed = observation(root.path());
    assert_ne!(
        added["input_revision"], fixed["input_revision"],
        "New file repair must change revision despite unchanged directory membership"
    );
    assert_eq!(fixed["generation"], generation);
    assert_eq!(fixed["coverage"]["complete"], true);
}

#[test]
fn freshness_negotiation_and_generation_errors_do_not_mix_context() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path());
    let (code, unsupported) = request(
        root.path(),
        json!({"kind":"freshness","observation_version":2}),
    );
    assert_eq!(code, 2);
    assert_eq!(
        unsupported["error"]["code"],
        "unsupported_freshness_version"
    );
    let (code, obsolete) = raw_request(
        json!({"version":1,"project":root.path(),"expected_generation":"obsolete","query":{"kind":"freshness","observation_version":1}}),
    );
    assert_eq!(code, 2);
    assert_eq!(obsolete["error"]["code"], "generation_mismatch");
    assert!(obsolete["generation"].is_null());
    assert!(obsolete["freshness"].is_null());
    assert!(obsolete["result"].is_null());
    let initial = observation(root.path());
    let (code, bound) = raw_request(
        json!({"version":1,"project":root.path(),"expected_generation":initial["generation"],"query":{"kind":"freshness","observation_version":1}}),
    );
    assert_eq!(code, 0, "{bound}");
    assert_eq!(bound["result"]["input_revision"], initial["input_revision"]);
    assert_eq!(bound["freshness"]["input_status"], "match");
}

struct ObserverProcess {
    child: std::process::Child,
    input: Option<std::process::ChildStdin>,
}

impl ObserverProcess {
    fn start(request: Value) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
            .arg("observe")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        writeln!(input, "{request}").unwrap();
        Self {
            child,
            input: Some(input),
        }
    }

    fn events(&mut self) -> std::sync::mpsc::Receiver<Value> {
        use std::io::{BufRead, BufReader};
        let (send, receive) = std::sync::mpsc::channel();
        let stdout = self.child.stdout.take().unwrap();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let value = serde_json::from_str(&line.unwrap()).unwrap();
                if send.send(value).is_err() {
                    break;
                }
            }
        });
        receive
    }

    fn eof(&mut self) -> bool {
        use std::time::{Duration, Instant};
        self.input.take();
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.child.try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        self.child.try_wait().unwrap().is_some()
    }
}

impl Drop for ObserverProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn observer_request(root: &Path) -> Value {
    json!({"version":1,"project":root,"query":{"kind":"freshness","observation_version":1}})
}

fn next_event(events: &std::sync::mpsc::Receiver<Value>) -> Value {
    events
        .recv_timeout(std::time::Duration::from_secs(7))
        .expect("observer event")
}

#[test]
fn observer_stream_emits_initial_and_stale_repairs_then_exits_on_eof() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path());
    let mut process = ObserverProcess::start(observer_request(root.path()));
    let events = process.events();
    let initial = next_event(&events);
    assert_eq!(initial["kind"], "initial");
    fs::write(root.path().join("app.ts"), "invalid").unwrap();
    let stale = next_event(&events);
    assert_eq!(stale["kind"], "change");
    fs::write(root.path().join("app.ts"), "export const repaired = 1;").unwrap();
    let repaired = next_event(&events);
    assert_ne!(
        stale["observation"]["input_revision"],
        repaired["observation"]["input_revision"]
    );
    assert_eq!(
        initial["observation"]["generation"],
        repaired["observation"]["generation"]
    );
    assert!(process.eof(), "EOF must end observer");
}

#[cfg(unix)]
fn fill_pipe(process: &ObserverProcess, paths: &[std::path::PathBuf]) -> bool {
    use std::{os::fd::AsRawFd, time::Duration};
    let stdout = process.child.stdout.as_ref().unwrap().as_raw_fd();
    let (mut previous, mut stable) = (0, 0);
    for edit in 0..30 {
        for path in paths {
            fs::write(path, edit.to_string()).unwrap();
        }
        std::thread::sleep(Duration::from_millis(300));
        let mut available = 0;
        assert_eq!(
            unsafe { libc::ioctl(stdout, libc::FIONREAD, &mut available) },
            0
        );
        stable = if available >= 8192 && available == previous {
            stable + 1
        } else {
            0
        };
        previous = available;
        if stable >= 3 {
            return true;
        }
    }
    false
}

#[cfg(unix)]
#[test]
fn observer_eof_remains_bounded_when_stdout_pipe_is_full() {
    let root = tempfile::tempdir().unwrap();
    let paths = (0..80)
        .map(|index| {
            root.path()
                .join(format!("input-{index:02}-{}.ts", "x".repeat(140)))
        })
        .collect::<Vec<_>>();
    for path in &paths {
        fs::write(path, "before").unwrap();
    }
    let roots = [("repo".into(), root.path().into())];
    let fingerprints = fingerprint::capture_excluding(paths.clone(), &[]).unwrap();
    let mut graph = artifact::canonical(&GraphData::default(), &roots).unwrap();
    graph.graph["analysis"] = serde_json::to_value(
        artifact::provenance::build(&fingerprints, &[], "{}", &roots).unwrap(),
    )
    .unwrap();
    fs::create_dir(root.path().join(".codecallnavigator")).unwrap();
    fs::write(
        root.path().join(artifact::GRAPH_PATH),
        graph.bytes().unwrap(),
    )
    .unwrap();
    let mut process = ObserverProcess::start(observer_request(root.path()));
    assert!(
        fill_pipe(&process, &paths),
        "Repeated large observations must fill the undrained pipe"
    );
    assert!(
        process.eof(),
        "EOF must terminate even with blocked stdout writer"
    );
}

#[test]
fn observer_heartbeats_and_generation_replacement_follow_saved_artifact() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path());
    let mut process = ObserverProcess::start(observer_request(root.path()));
    let events = process.events();
    let initial = next_event(&events);
    let heartbeat = next_event(&events);
    assert_eq!(heartbeat["kind"], "heartbeat");
    assert_eq!(
        heartbeat["observation"]["input_revision"],
        initial["observation"]["input_revision"]
    );
    let path = root.path().join(artifact::GRAPH_PATH);
    let mut graph: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    graph["extension"] = json!("replacement");
    fs::write(&path, serde_json::to_vec(&graph).unwrap()).unwrap();
    let replaced = next_event(&events);
    assert_eq!(replaced["kind"], "change");
    assert_ne!(
        replaced["observation"]["generation"],
        initial["observation"]["generation"]
    );
    assert_eq!(
        replaced["sequence"].as_u64(),
        heartbeat["sequence"].as_u64().map(|value| value + 1)
    );
    assert!(process.eof());
}

#[test]
fn bound_observer_rejects_generation_replacement_without_mixed_observation() {
    let root = tempfile::tempdir().unwrap();
    publish(root.path());
    let mut request = observer_request(root.path());
    request["expected_generation"] = observation(root.path())["generation"].clone();
    let mut process = ObserverProcess::start(request);
    let events = process.events();
    assert_eq!(next_event(&events)["kind"], "initial");
    let path = root.path().join(artifact::GRAPH_PATH);
    fs::write(&path, "{}").unwrap();
    let failed = next_event(&events);
    assert_eq!(failed["kind"], "failed");
    assert_eq!(failed["error"]["code"], "generation_mismatch");
    assert!(failed["observation"].is_null());
    assert!(process.eof());
}
