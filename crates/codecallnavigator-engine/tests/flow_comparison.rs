//! Public saved-flow comparison over Git-addressed library graphs.
#[cfg(unix)]
#[path = "flow_comparison/cancellation.rs"]
mod cancellation;
#[path = "flow_comparison/independent.rs"]
mod independent;
#[path = "flow_comparison/limits.rs"]
mod limits;
#[path = "flow_comparison/session.rs"]
mod session;
#[path = "flow_comparison/support.rs"]
mod support;
use codecallnavigator_core::artifact;
use serde_json::json;
use std::fs;
use support::*;

#[test]
fn change_pages_and_source_diff_use_the_same_saved_revision_binding() {
    let repo = Repository::revisions_only();
    let root = repo.root();
    let mut request = json!({
        "kind": "comparison_changes",
        "base": SAVED.base,
        "head": SAVED.head,
        "selection": {"kind": "list", "limit": 1}
    });
    let first = query(root, request.clone());
    assert!(first["error"].is_null(), "{first}");
    let result = &first["result"];
    assert_eq!(result["kind"], "list");
    assert_eq!(result["version"], 1);
    assert_eq!(first["generation"], result["head"]["digest"]);
    assert_eq!(result["returned"], 1);
    assert_eq!(result["more"], true);
    assert_eq!(result["next_offset"], 1);

    request["selection"]["offset"] = result["next_offset"].clone();
    assert_eq!(
        query(root, request.clone())["error"]["code"],
        "invalid_request"
    );
    request["expected_comparison"] = result["comparison_id"].clone();
    request["selection"]["limit"] = json!(100);
    let rest = query(root, request.clone());
    assert!(rest["error"].is_null(), "{rest}");
    assert_eq!(rest["result"]["offset"], 1);
    assert_eq!(rest["result"]["more"], false);
    let records: Vec<_> = result["changes"]
        .as_array()
        .unwrap()
        .iter()
        .chain(rest["result"]["changes"].as_array().unwrap())
        .collect();
    let added = symbol(&repo.after, "added");
    let removed = symbol(&repo.before, "removed");
    for (symbol, status) in [(added, "added"), (removed, "removed")] {
        assert!(records.iter().any(|record| {
            record["kind"] == "method" && record["symbol"] == symbol && record["status"] == status
        }));
    }
    let owner = records
        .into_iter()
        .find(|record| matches!(record["kind"].as_str(), Some("class" | "file")))
        .expect("changed source has a class or file row");
    request["selection"] = json!({"kind": "source_diff", "id": owner["id"]});
    let diff = query(root, request.clone());
    assert!(diff["error"].is_null(), "{diff}");
    assert_eq!(diff["result"]["kind"], "source_diff");
    assert_eq!(diff["result"]["id"], owner["id"]);
    assert_eq!(diff["result"]["status"], "ready");
    assert!(diff["result"]["hunks"].as_array().is_some_and(|hunks| {
        hunks.iter().any(|hunk| {
            hunk["lines"].as_array().unwrap().iter().any(|line| {
                line["kind"] == "insert" && line["text"].as_str().unwrap().contains("added")
            })
        })
    }));
    request["expected_comparison"] = json!("0".repeat(64));
    assert_eq!(query(root, request)["error"]["code"], "comparison_mismatch");
    assert!(artifact::working_manifest_path(root).is_err());
}

#[test]
fn committed_catalog_and_detail_use_endpoint_source_and_targets_without_working_graph() {
    let repo = Repository::revisions_only();
    let root = repo.root();
    let listing = query(root, SAVED.catalog());
    assert!(listing["error"].is_null(), "{listing}");
    let records = listing["result"]["records"].as_array().unwrap();
    let memberships: Vec<_> = records
        .iter()
        .map(|record| record["membership"].as_str().unwrap())
        .collect();
    assert!(
        memberships.contains(&"added")
            && memberships.contains(&"removed")
            && memberships.contains(&"both")
    );
    let selected = symbol(&repo.before, "same");
    let mut operation = SAVED.detail(&selected);
    operation["expected_comparison"] = listing["result"]["comparison_id"].clone();
    let response = query(root, operation);
    assert!(response["error"].is_null(), "{response}");
    let result = &response["result"];
    assert_eq!(result["correspondence"]["kind"], "exact");
    for (side, marker) in [("before", "before"), ("after", "after")] {
        assert_eq!(result[side]["status"], "ready");
        assert!(
            result[side]["source"]["content"]
                .as_str()
                .unwrap()
                .contains(marker)
        );
        assert_eq!(
            result[side]["targets"][0]["label"],
            format!("target {marker}")
        );
        assert_eq!(result[side]["source"]["origin"], "analyzed");
    }
    assert!(artifact::working_manifest_path(root).is_err());
}

#[test]
fn detail_exact_add_remove_manual_and_incompatible_membership_are_explicit() {
    let repo = Repository::new();
    let root = repo.root();
    let removed = symbol(&repo.before, "removed");
    let added = symbol(&repo.after, "added");
    let exact = query(root, SAVED.detail(&removed));
    assert!(exact["error"].is_null(), "{exact}");
    assert_eq!(exact["result"]["before"]["membership"], "present");
    assert_eq!(exact["result"]["after"]["status"], "missing");
    let mut manual = SAVED.detail(&removed);
    manual["selection"]["head_symbol"] = json!(added);
    assert_eq!(
        query(root, manual)["result"]["correspondence"]["kind"],
        "manual"
    );
    let mut invalid = repo.after.clone();
    invalid.schema = "future.graph.v99".into();
    fs::write(
        working_manifest(root),
        serde_json::to_vec(&invalid).unwrap(),
    )
    .unwrap();
    let listing = query(root, WORKTREE.catalog());
    assert!(listing["error"].is_null(), "{listing}");
    assert_eq!(listing["result"]["complete"], false);
    assert!(
        listing["result"]["records"]
            .as_array()
            .unwrap()
            .iter()
            .all(|record| record["membership"] == "unknown"
                && record["after"]["status"] == "unknown")
    );
}

#[test]
fn bound_worktree_changes_and_moved_refs_reject_followups_without_writes() {
    let repo = Repository::new();
    let root = repo.root();
    let first = query(root, WORKTREE.catalog());
    assert!(first["error"].is_null(), "{first}");
    let mut operation = WORKTREE.detail(&symbol(&repo.before, "same"));
    operation["expected_comparison"] = first["result"]["comparison_id"].clone();
    let mut changed = repo.after.clone();
    changed.graph["diagnostics"] = json!(["new bytes"]);
    save(root, &changed);
    assert_eq!(
        query(root, operation)["error"]["code"],
        "comparison_mismatch"
    );
    let before = fs::read(working_manifest(root)).unwrap();
    let committed = query(root, SAVED.catalog());
    let mut operation = SAVED.detail(&symbol(&repo.before, "same"));
    operation["expected_comparison"] = committed["result"]["comparison_id"].clone();
    commit_as(root, &repo.after, "moved-head");
    git(root, &["tag", "-f", "head"]);
    assert_eq!(
        query(root, operation)["error"]["code"],
        "comparison_mismatch"
    );
    assert_eq!(fs::read(working_manifest(root)).unwrap(), before);
    assert!(!root.join(".git/FETCH_HEAD").exists());
    assert!(!root.join(".codecallnavigator/analysis.lock").exists());
}
