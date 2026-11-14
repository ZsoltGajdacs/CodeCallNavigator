//! Public saved-flow comparison over real Git artifacts, without analysis or source reconstruction.
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
fn committed_catalog_and_detail_use_endpoint_source_and_targets_without_working_graph() {
    let repo = Repository::new();
    let root = repo.root();
    fs::remove_file(root.join(artifact::GRAPH_PATH)).unwrap();
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
    assert!(!root.join(artifact::GRAPH_PATH).exists());
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
        root.join(artifact::GRAPH_PATH),
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
    let before = fs::read(root.join(artifact::GRAPH_PATH)).unwrap();
    let committed = query(root, SAVED.catalog());
    let mut operation = SAVED.detail(&symbol(&repo.before, "same"));
    operation["expected_comparison"] = committed["result"]["comparison_id"].clone();
    git(
        root,
        &[
            "commit",
            "--allow-empty",
            "-qm",
            "same saved bytes new commit",
        ],
    );
    git(root, &["tag", "-f", "head"]);
    assert_eq!(
        query(root, operation)["error"]["code"],
        "comparison_mismatch"
    );
    assert_eq!(fs::read(root.join(artifact::GRAPH_PATH)).unwrap(), before);
    assert!(!root.join(".git/FETCH_HEAD").exists());
    assert!(!root.join(".codecallnavigator/analysis.lock").exists());
}
