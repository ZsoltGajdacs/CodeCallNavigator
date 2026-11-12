//! Public executable comparisons use real Git blobs and never change the checkout.
use codecallnavigator_core::{artifact, fingerprint, model::GraphData};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn git(root: &Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn repository(root: &Path) -> artifact::Artifact {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    git(root, &["config", "user.name", "Comparison acceptance"]);
    fs::write(root.join("input.txt"), "before").unwrap();
    let roots = [("repo".into(), root.to_owned())];
    let inputs = fingerprint::capture_excluding([root.join("input.txt")], &[]).unwrap();
    let mut graph = artifact::canonical(&GraphData::default(), &roots).unwrap();
    graph.graph["analysis"] =
        serde_json::to_value(artifact::provenance::build(&inputs, &[], "", &roots).unwrap())
            .unwrap();
    fs::create_dir(root.join(".codecallnavigator")).unwrap();
    save(root, &graph);
    git(root, &["add", ".codecallnavigator/graph.json"]);
    git(root, &["commit", "-qm", "baseline"]);
    graph
}

fn save(root: &Path, graph: &artifact::Artifact) {
    fs::write(root.join(artifact::GRAPH_PATH), graph.bytes().unwrap()).unwrap();
}

fn query(root: &Path, operation: Value) -> Value {
    let request = json!({"version":1,"project":root,"query":operation});
    let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("query")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    write!(child.stdin.take().unwrap(), "{request}").unwrap();
    let output = child.wait_with_output().unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn comparison_reports_saved_worktree_freshness_and_rejects_mixed_pages() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let mut graph = repository(&root);
    graph.graph["diagnostics"] = json!(["added-a", "added-b"]);
    save(&root, &graph);
    fs::write(root.join("input.txt"), "after").unwrap();
    let before = fs::read(root.join(artifact::GRAPH_PATH)).unwrap();
    let first = query(&root, json!({"kind":"compare","base":"HEAD","limit":1}));
    assert!(first["error"].is_null(), "{first}");
    assert_eq!(first["freshness"]["input_status"], "stale");
    assert_eq!(first["result"]["returned"], 1);
    assert_eq!(first["result"]["more"], true);
    let binding = &first["result"]["comparison_id"];
    let next = query(
        &root,
        json!({"kind":"compare","base":"HEAD","limit":1,"offset":1,"expected_comparison":binding}),
    );
    assert!(next["error"].is_null(), "{next}");
    assert_ne!(first["result"]["changes"], next["result"]["changes"]);
    assert_eq!(before, fs::read(root.join(artifact::GRAPH_PATH)).unwrap());
    graph.graph["diagnostics"] = json!(["new-generation"]);
    save(&root, &graph);
    let mixed = query(
        &root,
        json!({"kind":"compare","base":"HEAD","offset":1,"expected_comparison":binding}),
    );
    assert_eq!(mixed["error"]["code"], "comparison_mismatch");
}

#[test]
fn committed_comparison_needs_no_working_graph_or_linked_checkout() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    repository(&root);
    fs::remove_file(root.join(artifact::GRAPH_PATH)).unwrap();
    let response = query(&root, json!({"kind":"compare","base":"HEAD","head":"HEAD"}));
    assert!(response["error"].is_null(), "{response}");
    assert!(response["freshness"].is_null());
    assert!(response["generation"].is_string());
    assert_eq!(response["result"]["graph_equal"], true);
    assert_eq!(response["result"]["metadata_equal"], true);
    assert_eq!(response["result"]["total"], 0);
    assert!(!root.join(artifact::GRAPH_PATH).exists());
    assert!(!root.join(".codecallnavigator/analysis.lock").exists());
}

#[test]
fn comparison_argument_and_revision_failures_are_structured() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    repository(&root);
    for operation in [
        json!({"kind":"compare","base":"HEAD","limit":0}),
        json!({"kind":"compare","base":"HEAD","limit":501}),
        json!({"kind":"compare","base":"HEAD","offset":1}),
        json!({"kind":"compare","base":"WORKTREE"}),
    ] {
        assert_eq!(query(&root, operation)["error"]["code"], "invalid_request");
    }
    let missing = query(&root, json!({"kind":"compare","base":"nonexistent"}));
    assert_eq!(missing["error"]["code"], "revision_unavailable");
}

#[test]
fn committed_refs_and_nested_paths_are_bound_in_sessions() {
    use std::io::{BufRead, BufReader};
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let graph = repository(&root);
    let nested = root.join("nested project");
    fs::create_dir_all(nested.join(".codecallnavigator")).unwrap();
    save(&nested, &graph);
    git(
        &root,
        &["add", "nested project/.codecallnavigator/graph.json"],
    );
    git(&root, &["commit", "-qm", "nested graph"]);
    git(&root, &["tag", "baseline-tag"]);
    git(&root, &["branch", "baseline-branch"]);
    fs::remove_file(nested.join(artifact::GRAPH_PATH)).unwrap();
    let operation = json!({"kind":"compare","base":"baseline-tag","head":"baseline-branch"});
    let initial = query(&nested, operation.clone());
    assert!(initial["error"].is_null(), "{initial}");
    assert_eq!(
        initial["result"]["head"]["path"],
        "nested project/.codecallnavigator/graph.json"
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("query-session")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut exchange = |operation: Value| {
        writeln!(
            input,
            "{}",
            json!({"version":1,"project":nested,"query":operation})
        )
        .unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        serde_json::from_str::<Value>(&line).unwrap()
    };
    assert_eq!(exchange(operation), initial);
    git(
        &root,
        &["commit", "--allow-empty", "-qm", "same graph new commit"],
    );
    git(&root, &["branch", "-f", "baseline-branch", "HEAD"]);
    let changed = exchange(
        json!({"kind":"compare","base":"baseline-tag","head":"baseline-branch","expected_comparison":initial["result"]["comparison_id"]}),
    );
    assert_eq!(changed["error"]["code"], "comparison_mismatch");
    drop(exchange);
    drop(input);
    assert!(child.wait().unwrap().success());
}

#[test]
fn byte_budget_shortens_pages_and_rejects_an_oversized_record() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let mut graph = repository(&root);
    graph
        .extensions
        .insert("large-a".into(), json!("x".repeat(13 * 1024 * 1024)));
    graph
        .extensions
        .insert("large-b".into(), json!("y".repeat(13 * 1024 * 1024)));
    save(&root, &graph);
    let first = query(&root, json!({"kind":"compare","base":"HEAD"}));
    assert!(first["error"].is_null());
    assert_eq!(first["result"]["returned"], 1);
    assert_eq!(first["result"]["more"], true);
    assert!(serde_json::to_vec(&first).unwrap().len() <= 24 * 1024 * 1024);
    let second = query(
        &root,
        json!({"kind":"compare","base":"HEAD","offset":1,"expected_comparison":first["result"]["comparison_id"]}),
    );
    assert!(second["error"].is_null());
    assert_eq!(second["result"]["returned"], 1);
    assert_eq!(second["result"]["more"], false);
    graph.extensions.clear();
    graph
        .extensions
        .insert("oversized".into(), json!("z".repeat(25 * 1024 * 1024)));
    save(&root, &graph);
    assert_eq!(
        query(&root, json!({"kind":"compare","base":"HEAD"}))["error"]["code"],
        "response_too_large"
    );
}
