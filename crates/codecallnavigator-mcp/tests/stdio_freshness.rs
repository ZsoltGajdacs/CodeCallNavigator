//! Typed observations cross only the installed CLI/MCP process boundary.
mod support;
use serde_json::{Value, json};
use std::fs;
use support::{Client, cli_query, command, engine, typescript};

fn without_timing(mut envelope: Value) -> Value {
    envelope["result"]
        .as_object_mut()
        .unwrap()
        .remove("verification_milliseconds");
    envelope
}

#[test]
#[ignore = "requires built real CLI, Node24 and pinned TypeScript6 modules"]
fn real_freshness_preserves_cli_observations_and_generation_errors() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    typescript(&root);
    let engine = engine();
    let mut client = Client::launch(command(&engine, &root));
    assert_ne!(client.call("refresh_graph", json!({}))["isError"], true);
    let graph = root.join(".codecallnavigator/graph.json");
    let before = fs::read(&graph).unwrap();
    let generation = client.graph("graph_status", json!({}))["generation"].clone();
    let initial = client.graph("graph_freshness", json!({"expected_generation":generation}));
    let direct = cli_query(
        &engine,
        &root,
        json!({"kind":"freshness","observation_version":1}),
    );
    assert_eq!(without_timing(initial.clone()), without_timing(direct));
    assert_eq!(initial["result"]["generation"], generation);
    assert_eq!(initial["result"]["coverage"]["certifies_build"], false);
    assert_eq!(initial["result"]["mode"]["kind"], "verified_polling");
    fs::write(
        root.join("entry.ts"),
        "export function start(){return 3;}\n",
    )
    .unwrap();
    let stale = client.graph("graph_freshness", json!({"expected_generation":generation}));
    assert_eq!(stale["result"]["input_status"], "stale");
    assert_ne!(
        stale["result"]["input_revision"],
        initial["result"]["input_revision"]
    );
    fs::write(root.join("new.ts"), "export function added(\n").unwrap();
    let invalid = client.graph("graph_freshness", json!({}));
    fs::write(root.join("new.ts"), "export function added(){return 4;}\n").unwrap();
    let repaired = client.graph("graph_freshness", json!({}));
    assert_ne!(
        invalid["result"]["input_revision"],
        repaired["result"]["input_revision"]
    );
    assert_eq!(repaired["result"]["input_status"], "stale");
    assert_eq!(
        fs::read(&graph).unwrap(),
        before,
        "Queries published a graph"
    );
    assert_ne!(client.call("refresh_graph", json!({}))["isError"], true);
    let mismatch = client.call("graph_freshness", json!({"expected_generation":generation}));
    assert_eq!(mismatch["isError"], true);
    assert_eq!(
        mismatch["structuredContent"]["error"]["code"],
        "generation_mismatch"
    );
    let latest = client.graph("graph_freshness", json!({}));
    assert_ne!(latest["generation"], generation);
    assert_eq!(latest["result"]["generation"], latest["generation"]);
    assert!(client.request("ping", json!({})).get("result").is_some());
}
