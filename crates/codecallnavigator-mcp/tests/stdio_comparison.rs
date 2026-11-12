//! Real CLI and MCP processes; no workspace graph-library dependencies.
mod support;
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
use support::{Client, cli_query, command, engine};

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires built real CLI via CODECALLNAVIGATOR_TEST_ENGINE"]
fn real_stdio_comparison_preserves_cli_pages_metadata_and_errors() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "test@example.invalid"]);
    git(&root, &["config", "user.name", "MCP comparison acceptance"]);
    fs::create_dir(root.join(".codecallnavigator")).unwrap();
    let path = root.join(".codecallnavigator/graph.json");
    let mut graph = json!({"schema":"codecallnavigator.graph.v1","graph":{"nodes":[],"edges":[],"roots":[],"diagnostics":[]}});
    fs::write(&path, serde_json::to_vec(&graph).unwrap()).unwrap();
    git(&root, &["add", ".codecallnavigator/graph.json"]);
    git(&root, &["commit", "-qm", "baseline"]);
    graph["graph"]["diagnostics"] = json!(["first", "second"]);
    fs::write(&path, serde_json::to_vec(&graph).unwrap()).unwrap();
    let engine = engine();
    let mut client = Client::launch(command(&engine, &root));
    let tools = client.request("tools/list", json!({}));
    assert!(
        tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "compare_graphs")
    );
    check_pages_and_errors(&mut client, &engine, &root);
    client.disconnect();
    client.assert_shutdown();
}

fn check_pages_and_errors(client: &mut Client, engine: &Path, root: &Path) {
    let path = root.join(".codecallnavigator/graph.json");
    let bytes = fs::read(&path).unwrap();
    let mut graph: Value = serde_json::from_slice(&bytes).unwrap();
    let first = client.graph("compare_graphs", json!({"base":"HEAD","limit":1}));
    assert_eq!(
        first,
        cli_query(
            engine,
            root,
            json!({"kind":"compare","base":"HEAD","limit":1})
        )
    );
    assert_eq!(first["freshness"]["input_status"], "unverifiable");
    let mut records = first["result"]["changes"].as_array().unwrap().clone();
    let next = client.graph("compare_graphs", json!({"base":"HEAD","offset":1,"limit":1,"expected_comparison":first["result"]["comparison_id"],"expected_generation":first["generation"]}));
    records.extend(next["result"]["changes"].as_array().unwrap().clone());
    let complete = cli_query(engine, root, json!({"kind":"compare","base":"HEAD"}));
    assert_eq!(Value::Array(records), complete["result"]["changes"]);
    let invalid = client.call("compare_graphs", json!({"base":"HEAD","offset":1}));
    assert_eq!(
        invalid["structuredContent"]["error"]["code"],
        "invalid_arguments"
    );
    let missing = client.call("compare_graphs", json!({"base":"missing-ref"}));
    assert_eq!(
        missing["structuredContent"]["error"]["code"],
        "revision_unavailable"
    );
    assert_eq!(bytes, fs::read(&path).unwrap());
    graph["graph"]["diagnostics"] = json!(["changed"]);
    fs::write(&path, serde_json::to_vec(&graph).unwrap()).unwrap();
    let mixed = client.call(
        "compare_graphs",
        json!({"base":"HEAD","offset":1,"expected_comparison":first["result"]["comparison_id"]}),
    );
    assert_eq!(
        mixed["structuredContent"]["error"]["code"],
        "comparison_mismatch"
    );
    fs::remove_file(&path).unwrap();
    let committed = client.graph("compare_graphs", json!({"base":"HEAD","head":"HEAD"}));
    assert!(committed["freshness"].is_null());
    assert_eq!(committed["result"]["graph_equal"], true);
    assert!(!path.exists());
    assert_eq!(
        fs::read_dir(root.join(".codecallnavigator"))
            .unwrap()
            .count(),
        0
    );
}
