//! Real CLI analysis cancellation preserves the prior compiler-produced artifact.
mod support;
use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use support::{Client, command, engine, typescript};

struct SlowBuild {
    pids: Vec<String>,
}
impl SlowBuild {
    fn start(root: &Path) {
        fs::write(root.join("build.gradle"), "").unwrap();
        let wrapper = root.join("gradlew");
        fs::write(&wrapper,"#!/bin/sh\nprintf '%s' \"$$\" > wrapper.pid.tmp\nmv wrapper.pid.tmp wrapper.pid\nsleep 30 &\nchild=$!\nprintf '%s' \"$child\" > child.pid.tmp\nmv child.pid.tmp child.pid\ntrap 'kill \"$child\" 2>/dev/null; wait \"$child\" 2>/dev/null; exit 143' TERM INT\nwait \"$child\"\n").unwrap();
        fs::set_permissions(wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn observe(root: &Path) -> Self {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !root.join("child.pid").exists() {
            assert!(
                Instant::now() < deadline,
                "Real CLI never reached Gradle build wrapper"
            );
            thread::sleep(Duration::from_millis(10));
        }
        Self {
            pids: ["wrapper.pid", "child.pid"]
                .into_iter()
                .map(|name| fs::read_to_string(root.join(name)).unwrap())
                .collect(),
        }
    }

    fn assert_stopped(&self) {
        let deadline = Instant::now() + Duration::from_secs(8);
        while self.pids.iter().any(|pid| alive(pid)) {
            assert!(
                Instant::now() < deadline,
                "CLI cancellation left build processes: {:?}",
                self.pids
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn alive(pid: &str) -> bool {
    Command::new("/bin/kill")
        .args(["-0", pid])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

impl Drop for SlowBuild {
    fn drop(&mut self) {
        for pid in &self.pids {
            let _ = Command::new("/bin/kill")
                .args(["-TERM", pid])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

#[test]
#[ignore = "requires built real CLI, Node24/TypeScript6 and Java25"]
fn cancelling_real_analysis_reaps_build_and_preserves_the_last_graph() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    typescript(&root);
    let mut client = Client::launch(command(&engine(), &root));
    assert_ne!(client.call("refresh_graph", json!({}))["isError"], true);
    let before = fs::read(root.join(".codecallnavigator/graph.json")).unwrap();
    let generation = client.graph("graph_status", json!({}))["generation"].clone();
    SlowBuild::start(&root);
    let request = client.start(
        "tools/call",
        json!({"name":"refresh_graph","arguments":{"targets":[{"path":root,"kind":"gradle"}]}}),
    );
    let build = SlowBuild::observe(&root);
    client.notify(
        "notifications/cancelled",
        json!({"requestId":request,"reason":"cancel real build"}),
    );
    build.assert_stopped();
    assert_eq!(
        fs::read(root.join(".codecallnavigator/graph.json")).unwrap(),
        before
    );
    let status = client.graph("graph_status", json!({"expected_generation":generation}));
    assert_eq!(status["generation"], generation);
    client.assert_no_response(request);
}

#[test]
#[ignore = "requires built real CLI, Node24/TypeScript6 and Java25"]
fn disconnecting_mcp_drains_real_analysis_before_the_runtime_exits() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    typescript(&root);
    let mut client = Client::launch(command(&engine(), &root));
    assert_ne!(client.call("refresh_graph", json!({}))["isError"], true);
    let before = fs::read(root.join(".codecallnavigator/graph.json")).unwrap();
    SlowBuild::start(&root);
    client.start(
        "tools/call",
        json!({"name":"refresh_graph","arguments":{"targets":[{"path":root,"kind":"gradle"}]}}),
    );
    let build = SlowBuild::observe(&root);
    client.disconnect();
    client.assert_shutdown();
    build.assert_stopped();
    assert_eq!(
        fs::read(root.join(".codecallnavigator/graph.json")).unwrap(),
        before
    );
}
