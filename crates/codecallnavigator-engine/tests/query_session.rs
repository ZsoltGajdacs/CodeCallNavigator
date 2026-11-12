//! A query session keeps an immutable parsed generation, never stale source status.
use codecallnavigator_core::{artifact, fingerprint, model::GraphData};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
fn live_explorer_works_before_first_analysis() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join(".hidden"), "content").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("query")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let request =
        json!({"version":1,"project":root.path(),"query":{"kind":"explorer","path":root.path()}});
    write!(child.stdin.take().unwrap(), "{request}").unwrap();
    let output = child.wait_with_output().unwrap();
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output.status.success(), "{response}");
    assert!(response["generation"].is_null());
    assert_eq!(response["result"]["origin"], "live");
    assert_eq!(response["result"]["entries"][0]["label"], ".hidden");
}

#[test]
fn session_rechecks_inputs_and_rejects_replaced_generations() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input.txt");
    fs::write(&input, "before").unwrap();
    let roots = [("repo".into(), root.path().to_owned())];
    let observations = fingerprint::capture_excluding([input.clone()], &[]).unwrap();
    let mut graph = artifact::canonical(&GraphData::default(), &roots).unwrap();
    graph.graph["analysis"] =
        serde_json::to_value(artifact::provenance::build(&observations, &[], "", &roots).unwrap())
            .unwrap();
    fs::create_dir(root.path().join(".codecallnavigator")).unwrap();
    let path = root.path().join(artifact::GRAPH_PATH);
    let bytes = graph.bytes().unwrap();
    fs::write(&path, &bytes).unwrap();
    let generation = blake3::hash(&bytes).to_hex().to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("query-session")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input_pipe = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let request = json!({"version":1,"project":root.path(),"expected_generation":generation,"query":{"kind":"status"}});
    let mut exchange = |value: &Value| {
        writeln!(input_pipe, "{value}").unwrap();
        input_pipe.flush().unwrap();
        let mut line = String::new();
        assert!(
            output.read_line(&mut line).unwrap() > 0,
            "Session exited without a response"
        );
        serde_json::from_str::<Value>(&line).unwrap()
    };
    let initial = exchange(&request);
    assert_eq!(initial["generation"], generation);
    assert_eq!(initial["freshness"]["input_status"], "match");
    fs::write(&input, "after").unwrap();
    assert_eq!(exchange(&request)["freshness"]["input_status"], "stale");
    graph.extensions.insert("session_test".into(), json!(true));
    fs::write(&path, graph.bytes().unwrap()).unwrap();
    assert_eq!(exchange(&request)["error"]["code"], "generation_mismatch");
    let latest = exchange(&json!({"version":1,"project":root.path(),"query":{"kind":"status"}}));
    assert_ne!(latest["generation"], generation);
    drop(exchange);
    drop(input_pipe);
    assert!(child.wait().unwrap().success());
}
