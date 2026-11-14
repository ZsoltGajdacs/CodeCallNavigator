//! Independent real-Git edge checks; malformed bytes are intentional endpoint fixtures.
use super::support::*;
use codecallnavigator_core::{
    artifact::{self, Artifact},
    flow::MAX_SOURCE_BYTES,
};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn linked(saved: &Artifact, root: &Path, missing: &Path) -> Artifact {
    let mut graph = saved.hydrate(&[("repo".into(), root.into())]).unwrap();
    let file = missing.join("F.java").to_string_lossy().into_owned();
    let build = missing.join("pom.xml").to_string_lossy().into_owned();
    for node in &mut graph.nodes {
        node.file = file.clone();
        node.build = build.clone();
    }
    for function in &mut graph.flow_inventory.as_mut().unwrap().functions {
        function.file = file.clone();
        function.build = build.clone();
    }
    for source in &mut graph.architecture.as_mut().unwrap().files {
        source.file = file.clone();
        source.build = build.clone();
    }
    artifact::canonical(
        &graph,
        &[
            ("repo".into(), root.into()),
            ("linked".into(), missing.into()),
        ],
    )
    .unwrap()
}

fn commit_as(root: &Path, saved: &Artifact, tag: &str) {
    save(root, saved);
    git(root, &["add", artifact::GRAPH_PATH]);
    git(root, &["commit", "--allow-empty", "-qm", tag]);
    git(root, &["tag", tag]);
}

#[test]
fn unmapped_linked_endpoint_source_needs_neither_checkout_nor_current_graph() {
    let repo = Repository::new();
    let root = repo.root();
    let missing = root.join("absent linked checkout");
    let before = linked(&repo.before, root, &missing);
    let after = linked(&repo.after, root, &missing);
    commit_as(root, &before, "linked-base");
    commit_as(root, &after, "linked-head");
    fs::remove_file(root.join(artifact::GRAPH_PATH)).unwrap();
    let revisions = Revisions {
        base: "linked-base",
        head: "linked-head",
    };
    let listing = query(root, revisions.catalog());
    assert!(listing["error"].is_null(), "{listing}");
    let detail = query(root, revisions.detail(&symbol(&before, "same")));
    assert!(detail["error"].is_null(), "{detail}");
    for (side, saved) in [("before", &before), ("after", &after)] {
        let actual = &detail["result"][side];
        assert_eq!(
            json!({"status":actual["status"],"file":actual["source"]["file"],"content":actual["source"]["content"]}),
            json!({"status":"ready","file":"@linked/F.java","content":saved.graph["architecture"]["files"][0]["content"]})
        );
    }
    assert!(detail["freshness"].is_null());
    assert!(!missing.exists() && !root.join(artifact::GRAPH_PATH).exists());
}

#[derive(Clone, Copy, Debug)]
enum Ambiguity {
    Source,
    Function,
    Owner,
    Node,
}
impl Ambiguity {
    fn corrupt(self, saved: &mut Artifact) {
        match self {
            Self::Source => {
                let duplicate = saved.graph["architecture"]["files"][0].clone();
                saved.graph["architecture"]["files"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            Self::Function => {
                let duplicate = saved.graph["flow_inventory"]["functions"][0].clone();
                saved.graph["flow_inventory"]["functions"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            Self::Owner => {
                saved.graph["flow_inventory"]["functions"][0]["project"] = json!("Wrong owner")
            }
            Self::Node => {
                let duplicate = saved.graph["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|node| node["signature"] == "App::same()")
                    .unwrap()
                    .clone();
                saved.graph["nodes"].as_array_mut().unwrap().push(duplicate);
            }
        }
    }
    fn expected(self) -> Value {
        let (reason, membership, union, endpoint) = match self {
            Self::Source => ("duplicate_source", "present", "both", "available"),
            Self::Function => ("duplicate_function", "present", "both", "available"),
            Self::Owner => ("function_ownership", "present", "both", "available"),
            Self::Node => ("invalid_graph", "unknown", "unknown", "unavailable"),
        };
        json!({"before":"ready","after":"unavailable","reason":reason,"membership":membership,"union":union,"endpoint":endpoint})
    }
}

#[test]
fn strict_invalid_artifacts_and_consumed_owner_ambiguity_have_distinct_membership() {
    let repo = Repository::new();
    let root = repo.root();
    let id = symbol(&repo.before, "same");
    for fault in [
        Ambiguity::Source,
        Ambiguity::Function,
        Ambiguity::Owner,
        Ambiguity::Node,
    ] {
        let mut saved = repo.after.clone();
        fault.corrupt(&mut saved);
        // Bypass fixture serialization validation only to feed intentionally malformed public input.
        fs::write(
            root.join(artifact::GRAPH_PATH),
            serde_json::to_vec(&saved).unwrap(),
        )
        .unwrap();
        let detail = query(root, WORKTREE.detail(&id));
        let listing = query(root, WORKTREE.catalog());
        assert!(
            detail["error"].is_null() && listing["error"].is_null(),
            "{detail} {listing}"
        );
        let record = listing["result"]["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["symbol"] == id)
            .unwrap();
        assert_eq!(
            json!({"before":detail["result"]["before"]["status"],"after":detail["result"]["after"]["status"],"reason":detail["result"]["after"]["reason"],"membership":detail["result"]["after"]["membership"],"union":record["membership"],"endpoint":listing["result"]["endpoints"]["after"]["status"]}),
            fault.expected(),
            "{fault:?}"
        );
    }
}

fn padded_source(saved: &Artifact, size: usize) -> Artifact {
    let mut padded = saved.clone();
    let original = padded.graph["architecture"]["files"][0]["content"]
        .as_str()
        .unwrap();
    let content = format!("{original}/*{}*/", "x".repeat(size - original.len() - 4));
    assert_eq!(content.len(), size);
    padded.graph["architecture"]["files"][0]["content"] = json!(content);
    padded
}

#[test]
fn exact_source_cap_is_ready_and_one_extra_byte_is_explicitly_unavailable() {
    let repo = Repository::new();
    let root = repo.root();
    let id = symbol(&repo.before, "same");
    save(root, &padded_source(&repo.after, MAX_SOURCE_BYTES as usize));
    let boundary = query(root, WORKTREE.detail(&id));
    assert!(boundary["error"].is_null(), "{boundary}");
    assert_eq!(
        boundary["result"]["after"]["source"]["content"]
            .as_str()
            .unwrap()
            .len(),
        MAX_SOURCE_BYTES as usize
    );
    save(
        root,
        &padded_source(&repo.after, MAX_SOURCE_BYTES as usize + 1),
    );
    let over = query(root, WORKTREE.detail(&id));
    assert!(over["error"].is_null(), "{over}");
    assert_eq!(
        json!({"before":over["result"]["before"]["status"],"after":over["result"]["after"]["status"],"reason":over["result"]["after"]["reason"]}),
        json!({"before":"ready","after":"unavailable","reason":"invalid_flow"})
    );
    assert!(
        over["result"]["after"]["message"]
            .as_str()
            .unwrap()
            .contains("16 MiB")
    );
}

#[test]
fn unavailable_schema_metadata_counts_toward_the_complete_response_budget() {
    let repo = Repository::new();
    let root = repo.root();
    let mut invalid = repo.after.clone();
    invalid.schema = format!("future.{}", "s".repeat(12 * 1024 * 1024));
    fs::write(
        root.join(artifact::GRAPH_PATH),
        serde_json::to_vec(&invalid).unwrap(),
    )
    .unwrap();
    for operation in [
        WORKTREE.catalog(),
        WORKTREE.detail(&symbol(&repo.before, "same")),
    ] {
        let response = query(root, operation);
        assert_eq!(response["error"]["code"], "response_too_large");
        assert!(response["result"].is_null());
        assert!(serde_json::to_vec(&response).unwrap().len() < 1024);
    }
}

fn renamed(saved: &Artifact, root: &Path) -> Artifact {
    let mut graph = saved.hydrate(&[("repo".into(), root.into())]).unwrap();
    let node = graph
        .nodes
        .iter_mut()
        .find(|node| node.signature == "App::same()")
        .unwrap();
    let previous = node.id.clone();
    node.id = "next".into();
    node.signature = "App::next()".into();
    node.label = "next".into();
    for function in &mut graph.flow_inventory.as_mut().unwrap().functions {
        if function.symbol == previous {
            function.symbol = "next".into();
        }
    }
    let file = &mut graph.architecture.as_mut().unwrap().files[0];
    file.content = file.content.replace("int same()", "int next()");
    artifact::canonical(&graph, &[("repo".into(), root.into())]).unwrap()
}

#[test]
fn real_ready_manual_counterpart_does_not_invent_a_rename_match() {
    let repo = Repository::new();
    let root = repo.root();
    let after = renamed(&repo.after, root);
    commit_as(root, &after, "renamed-head");
    let revisions = Revisions {
        base: "base",
        head: "renamed-head",
    };
    let before_id = symbol(&repo.before, "same");
    let after_id = symbol(&after, "next");
    assert_ne!(before_id, after_id);
    let listing = query(root, revisions.catalog());
    let records = listing["result"]["records"].as_array().unwrap();
    for (id, membership) in [(&before_id, "removed"), (&after_id, "added")] {
        assert_eq!(
            records
                .iter()
                .find(|record| record["symbol"] == *id)
                .unwrap()["membership"],
            membership
        );
    }
    let exact = query(root, revisions.detail(&before_id));
    assert_eq!(exact["result"]["after"]["status"], "missing");
    let mut operation = revisions.detail(&before_id);
    operation["selection"]["head_symbol"] = json!(after_id);
    operation["expected_comparison"] = listing["result"]["comparison_id"].clone();
    let manual = query(root, operation);
    assert!(manual["error"].is_null(), "{manual}");
    assert_eq!(manual["result"]["correspondence"]["kind"], "manual");
    for (side, id, declaration) in [
        ("before", before_id, "int same()"),
        ("after", after_id, "int next()"),
    ] {
        let selected = &manual["result"][side];
        assert_eq!(
            (
                selected["status"].as_str(),
                selected["function"]["symbol"].as_str()
            ),
            (Some("ready"), Some(id.as_str()))
        );
        assert!(
            selected["source"]["content"]
                .as_str()
                .unwrap()
                .contains(declaration)
        );
    }
}
