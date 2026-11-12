//! Agents invoke the public command from PATH, without configuring a binary path.
use serde_json::{Value, json};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn public_command() -> Command {
    let directory = Path::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .parent()
        .unwrap();
    let mut command = Command::new("ccnavigator");
    command.env("PATH", directory);
    command
}

#[test]
fn path_command_exposes_the_same_public_contract() {
    let capabilities = public_command().arg("capabilities").output().unwrap();
    assert!(capabilities.status.success());
    let internal = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("capabilities")
        .output()
        .unwrap();
    assert_eq!(capabilities.stdout, internal.stdout);
    let help = public_command().arg("--help").output().unwrap();
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("ccnavigator query")
    );
    let root = tempfile::tempdir().unwrap();
    let mut child = public_command()
        .arg("query")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    write!(
        child.stdin.take().unwrap(),
        "{}",
        json!({
            "version": 1, "project": root.path(), "query": {"kind": "status"}
        })
    )
    .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["error"]["code"], "graph_missing");
    assert!(!root.path().join(".codecallnavigator/graph.json").exists());
}
