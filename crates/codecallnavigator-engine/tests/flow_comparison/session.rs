use super::support::*;
use codecallnavigator_core::artifact;
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::Stdio,
};

#[test]
fn missing_inventory_source_and_wrong_project_source_are_endpoint_local() {
    let repo = Repository::new();
    let root = repo.root();
    let id = symbol(&repo.before, "same");
    let mut old = repo.after.clone();
    old.graph.as_object_mut().unwrap().remove("flow_inventory");
    save(root, &old);
    let response = query(root, WORKTREE.detail(&id));
    assert_eq!(response["result"]["after"]["reason"], "missing_inventory");
    assert_eq!(response["result"]["after"]["membership"], "present");
    let mut missing = repo.after.clone();
    missing
        .graph
        .as_object_mut()
        .unwrap()
        .remove("architecture");
    save(root, &missing);
    assert_eq!(
        query(root, WORKTREE.detail(&id))["result"]["after"]["reason"],
        "missing_source"
    );
    let mut owned = repo.after.clone();
    let mut wrong = owned.graph["architecture"]["files"][0].clone();
    wrong["project"] = json!("Other");
    wrong["content"] = json!("wrong source");
    owned.graph["architecture"]["files"]
        .as_array_mut()
        .unwrap()
        .insert(0, wrong);
    save(root, &owned);
    let selected = query(root, WORKTREE.detail(&id));
    assert_eq!(
        selected["result"]["after"]["source"]["content"],
        repo.after.graph["architecture"]["files"][0]["content"]
    );
}

#[test]
fn session_and_oneshot_match_without_using_or_mutating_current_graph() {
    let repo = Repository::new();
    let root = repo.root();
    fs::remove_file(root.join(artifact::GRAPH_PATH)).unwrap();
    let before_index = fs::read(root.join(".git/index")).unwrap();
    let before_head = git(root, &["rev-parse", "HEAD"]);
    let mut child = engine()
        .arg("query-session")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    for operation in [SAVED.catalog(), SAVED.detail(&symbol(&repo.before, "same"))] {
        writeln!(
            input,
            "{}",
            json!({"version":2,"project":root,"query":operation})
        )
        .unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        let session: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(session, query(root, operation));
    }
    drop(input);
    assert!(child.wait().unwrap().success());
    let unchanged = json!({"head":git(root,&["rev-parse","HEAD"]),"index":fs::read(root.join(".git/index")).unwrap(),"graph_exists":root.join(artifact::GRAPH_PATH).exists(),"fetch_exists":root.join(".git/FETCH_HEAD").exists(),"lock_exists":root.join(".codecallnavigator/analysis.lock").exists()});
    assert_eq!(
        unchanged,
        json!({"head":before_head,"index":before_index,"graph_exists":false,"fetch_exists":false,"lock_exists":false})
    );
}

#[test]
fn catalog_recorded_facts_do_not_claim_a_chart_was_built() {
    let repo = Repository::new();
    let root = repo.root();
    let mut invalid = repo.after.clone();
    let end = invalid.graph["architecture"]["files"][0]["content"]
        .as_str()
        .unwrap()
        .len()
        + 1;
    invalid.graph["flow_inventory"]["functions"][0]["span"]["end"] = json!(end);
    save(root, &invalid);
    let id = symbol(&repo.before, "same");
    let listing = query(root, WORKTREE.catalog());
    let record = listing["result"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["symbol"] == id)
        .unwrap();
    assert_eq!(record["after"]["flow"]["status"], "recorded");
    assert!(record["after"].get("chart").is_none());
    let detail = query(root, WORKTREE.detail(&id));
    assert_eq!(detail["result"]["after"]["reason"], "invalid_flow");
}

#[test]
fn endpoint_target_summaries_preserve_known_body_availability() {
    let repo = Repository::new();
    let mut saved = repo.after.clone();
    let target = symbol(&saved, "target");
    let nodes = saved.graph["nodes"].as_array_mut().unwrap();
    nodes.iter_mut().find(|node| node["id"] == target).unwrap()["body_unavailable"] = json!(true);
    save(repo.root(), &saved);
    let result = query(repo.root(), WORKTREE.detail(&symbol(&repo.before, "same")));
    assert_eq!(result["result"]["after"]["node"]["body_unavailable"], false);
    assert_eq!(
        result["result"]["after"]["targets"][0]["body_unavailable"],
        true
    );
    let before = &result["result"]["before"]["targets"][0];
    assert_eq!(before["body_unavailable"], false);
}

#[test]
fn live_source_freshness_never_replaces_cached_endpoint_source_or_chart() {
    let repo = Repository::new();
    let root = repo.root();
    let source = root.join("F.java");
    let mut saved = repo.after.clone();
    let content = saved.graph["architecture"]["files"][0]["content"]
        .as_str()
        .unwrap();
    fs::write(&source, content).unwrap();
    let roots = [("repo".into(), root.to_owned())];
    let observed =
        codecallnavigator_core::fingerprint::capture_excluding([source.clone()], &[]).unwrap();
    saved.graph["analysis"] =
        serde_json::to_value(artifact::provenance::build(&observed, &[], "", &roots).unwrap())
            .unwrap();
    save(root, &saved);
    let operation = WORKTREE.detail(&symbol(&repo.before, "same"));
    let initial = query(root, operation.clone());
    assert_eq!(initial["freshness"]["input_status"], "match");
    assert_eq!(initial["result"]["after"]["status"], "ready");
    let bytes = fs::read(root.join(artifact::GRAPH_PATH)).unwrap();
    for deleted in [false, true] {
        if deleted {
            fs::remove_file(&source).unwrap();
        } else {
            fs::write(&source, "different live contents").unwrap();
        }
        let later = query(root, operation.clone());
        assert_eq!(later["freshness"]["input_status"], "stale");
        assert_eq!(later["result"], initial["result"]);
        assert_eq!(fs::read(root.join(artifact::GRAPH_PATH)).unwrap(), bytes);
    }
}
