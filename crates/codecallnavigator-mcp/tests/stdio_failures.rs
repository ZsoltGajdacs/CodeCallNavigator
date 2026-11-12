//! Narrow external process doubles exercise malformed CLI and cancellation handling.
mod support;
use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use support::{Client, command};

fn fake_cli(root: &Path, mode: &str) -> PathBuf {
    let executable = root.join("fake_cli.py");
    fs::write(&executable, format!(r#"#!/usr/bin/env python3
import json, os, sys, time
from pathlib import Path
mode={mode:?}
if sys.argv[1]=='capabilities':
    print(json.dumps({{'query_version':99 if mode=='incompatible' else 1,'generation_binding':True,'comparison':{{'version':1,'snapshot_binding':True}},'operations':['compare','status','search','symbol','source','explorer','selection','neighbors','flow']}}))
elif mode=='hung':
    marker=Path(__file__).with_suffix('.pid')
    pending=marker.with_suffix('.pid.tmp')
    pending.write_text(str(os.getpid()))
    pending.replace(marker)
    time.sleep(30)
else:
    json.loads(sys.stdin.read())
    if mode=='malformed': print('{{broken')
    elif mode=='wrong_version': print(json.dumps({{'version':2,'generation':None,'freshness':None,'result':{{}},'error':None}}))
    else: print(json.dumps({{'version':1,'result':{{}},'error':None}}))
"#)).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    executable
}

pub fn alive(pid: &str) -> bool {
    Command::new("/bin/kill")
        .args(["-0", pid])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

pub fn wait_for_marker(path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(pid) = fs::read_to_string(path)
            && !pid.trim().is_empty()
        {
            return pid;
        }
        assert!(
            Instant::now() < deadline,
            "CLI work never started: {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

pub fn assert_stopped(pid: &str) {
    let deadline = Instant::now() + Duration::from_secs(6);
    while alive(pid) {
        assert!(
            Instant::now() < deadline,
            "Owned CLI process {pid} survived cancellation"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn incompatible_cli_is_rejected_before_serving_mcp_tools() {
    let root = tempfile::tempdir().unwrap();
    let executable = fake_cli(root.path(), "incompatible");
    let output = command(&executable, root.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "Startup failure polluted MCP stdout"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Incompatible"));
}

#[test]
fn help_and_version_do_not_require_a_configured_cli_or_project() {
    let executable = env!("CARGO_BIN_EXE_codecallnavigator-mcp");
    for flag in ["--help", "-h", "--version"] {
        let output = Command::new(executable)
            .arg(flag)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{flag}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let output = String::from_utf8(output.stdout).unwrap();
        if flag == "--version" {
            assert!(output.contains(env!("CARGO_PKG_VERSION")));
        } else {
            assert!(output.contains("--engine"));
            assert!(output.contains("--project"));
        }
    }
}

#[test]
fn malformed_and_incompatible_query_responses_are_tool_errors() {
    for mode in ["malformed", "wrong_version", "missing_fields"] {
        let root = tempfile::tempdir().unwrap();
        let executable = fake_cli(root.path(), mode);
        let mut client = Client::launch(command(&executable, root.path()));
        let result = client.call("graph_status", json!({}));
        assert_eq!(
            result["isError"], true,
            "Mode {mode} accepted invalid CLI output: {result}"
        );
        assert!(client.request("ping", json!({})).get("result").is_some());
    }
}

#[test]
fn cancelling_a_hung_cli_query_stops_it_and_keeps_mcp_usable() {
    let root = tempfile::tempdir().unwrap();
    let executable = fake_cli(root.path(), "hung");
    let mut client = Client::launch(command(&executable, root.path()));
    let id = client.start("tools/call", json!({"name":"graph_status","arguments":{}}));
    let pid = wait_for_marker(&executable.with_extension("pid"));
    client.notify(
        "notifications/cancelled",
        json!({"requestId":id,"reason":"acceptance cancellation"}),
    );
    assert_stopped(&pid);
    assert!(client.request("ping", json!({})).get("result").is_some());
    client.assert_no_response(id);
}
