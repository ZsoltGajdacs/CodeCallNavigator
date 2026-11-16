use super::support::*;
use codecallnavigator_core::artifact;
use serde_json::json;
use std::fs;

#[test]
fn catalog_paging_is_bound_progressing_and_complete() {
    let repo = Repository::new();
    let root = repo.root();
    let mut operation = SAVED.catalog();
    operation["selection"]["limit"] = json!(1);
    let mut records = Vec::new();
    let binding = loop {
        let response = query(root, operation.clone());
        assert!(response["error"].is_null(), "{response}");
        let result = &response["result"];
        let binding = result["comparison_id"].clone();
        records.extend(
            result["records"]
                .as_array()
                .unwrap()
                .iter()
                .map(|record| record["symbol"].as_str().unwrap().to_owned()),
        );
        if result["more"] == false {
            assert_eq!(records.len(), result["total"].as_u64().unwrap() as usize);
            break binding;
        }
        let next = result["next_offset"].as_u64().unwrap();
        assert!(next > operation["selection"]["offset"].as_u64().unwrap_or(0));
        operation["selection"]["offset"] = json!(next);
        operation["expected_comparison"] = binding.clone();
    };
    assert_eq!(records.len(), 4);
    assert!(records.windows(2).all(|pair| pair[0] < pair[1]));
    let mut invalid = SAVED.catalog();
    invalid["selection"]["offset"] = json!(1);
    assert_eq!(query(root, invalid)["error"]["code"], "invalid_request");
    let mut filtered = SAVED.catalog();
    filtered["selection"]["text"] = json!("SAME");
    filtered["expected_comparison"] = binding;
    assert_eq!(query(root, filtered)["result"]["total"], 1);
}

#[test]
fn complete_detail_budget_counts_escaping_and_never_drops_a_side() {
    let repo = Repository::new();
    let root = repo.root();
    let mut large = repo.after.clone();
    let content = large.graph["architecture"]["files"][0]["content"]
        .as_str()
        .unwrap()
        .to_owned()
        + "/*"
        + &"\"".repeat(7 * 1024 * 1024)
        + "*/";
    large.graph["architecture"]["files"][0]["content"] = json!(content);
    save(root, &large);
    git(
        root,
        &["add", artifact::GRAPH_PATH, ".codecallnavigator/objects"],
    );
    git(root, &["commit", "-qm", "escaped source"]);
    git(root, &["tag", "large"]);
    let response = query(
        root,
        Revisions {
            base: "large",
            head: "large",
        }
        .detail(&symbol(&large, "same")),
    );
    assert_eq!(response["error"]["code"], "response_too_large");
    assert!(response["result"].is_null());
    assert!(serde_json::to_vec(&response).unwrap().len() < 1024);
}

#[test]
fn oversized_catalog_returns_whole_records_and_progresses_by_bytes() {
    let repo = Repository::new();
    let root = repo.root();
    let mut large = repo.after.clone();
    for node in large.graph["nodes"].as_array_mut().unwrap() {
        node["signature"] = json!("q".repeat(9 * 1024 * 1024));
    }
    save(root, &large);
    let before = fs::read(root.join(artifact::GRAPH_PATH)).unwrap();
    let first = query(root, WORKTREE.catalog());
    assert!(first["error"].is_null(), "{first}");
    let page = &first["result"];
    let returned = page["returned"].as_u64().unwrap();
    assert!(returned > 0 && returned < page["total"].as_u64().unwrap());
    assert!(serde_json::to_vec(&first).unwrap().len() < 24 * 1024 * 1024);
    let mut next = WORKTREE.catalog();
    next["selection"]["offset"] = page["next_offset"].clone();
    next["expected_comparison"] = page["comparison_id"].clone();
    let second = query(root, next);
    assert!(second["error"].is_null(), "{second}");
    assert_eq!(second["result"]["more"], false);
    assert_eq!(
        returned + second["result"]["returned"].as_u64().unwrap(),
        page["total"].as_u64().unwrap()
    );
    assert_eq!(fs::read(root.join(artifact::GRAPH_PATH)).unwrap(), before);
}

#[test]
fn request_bounds_and_unknown_counterparts_are_explicit() {
    let repo = Repository::new();
    let root = repo.root();
    let mut invalids = Vec::new();
    for limit in [0, 501] {
        let mut value = SAVED.catalog();
        value["selection"]["limit"] = json!(limit);
        invalids.push(value);
    }
    let mut text = SAVED.catalog();
    text["selection"]["text"] = json!("x".repeat(1025));
    invalids.push(text);
    for symbol in ["".to_owned(), "x".repeat(16385)] {
        invalids.push(SAVED.detail(&symbol));
    }
    let mut digest = SAVED.catalog();
    digest["expected_comparison"] = json!("invalid");
    invalids.push(digest);
    for operation in invalids {
        assert_eq!(query(root, operation)["error"]["code"], "invalid_request");
    }
    let unknown = query(root, SAVED.detail("not-a-saved-symbol"));
    assert_eq!(unknown["result"]["before"]["status"], "missing");
    assert_eq!(unknown["result"]["after"]["status"], "missing");
}
