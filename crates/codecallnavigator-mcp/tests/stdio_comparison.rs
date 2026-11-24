//! Real CLI and MCP processes; no workspace graph-library dependencies.
mod support;
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
use support::{Client, LIBRARY, cli_query, cli_query_with, command, engine, typescript};

const GRAPH_CHANGES: (&str, &str) = ("compare_changes", "comparison_changes");
const SOURCE_CHANGES: (&str, &str) = ("compare_source_changes", "git_comparison_changes");

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
#[ignore = "requires built real CLI with Node and TypeScript"]
fn real_stdio_comparison_preserves_cli_pages_metadata_and_errors() {
    let directory = tempfile::tempdir().unwrap();
    let root = support::project(&directory);
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "test@example.invalid"]);
    git(&root, &["config", "user.name", "MCP comparison acceptance"]);
    fs::write(root.join(".git/info/exclude"), "/node_modules\n").unwrap();
    typescript(&root);
    fs::write(root.join("services.ts"), services("LegacyTax")).unwrap();
    git(
        &root,
        &[
            "add",
            "tsconfig.json",
            "entry.ts",
            "other.ts",
            "services.ts",
        ],
    );
    git(&root, &["commit", "-qm", "baseline"]);
    let engine = engine();
    let mut client = Client::launch(command(&engine, &root));
    check_comparison_tools(&mut client);
    check_source_changes_without_graphs(&mut client, &engine, &root);
    check_oversized_source_preview(&mut client, &engine, &root);
    check_retained_baseline(&mut client);

    replace_entry(&root, 7);
    fs::write(root.join("services.ts"), services("TaxService")).unwrap();
    let current = client.graph("refresh_graph", json!({}));
    assert_eq!(
        current["result"]["retention"]["status"], "working_only",
        "{current}"
    );
    assert!(current["result"]["retention"]["reason"].is_string());
    let changes_binding = check_change_summary(&mut client, &engine, &root);
    let checkout = check_class_graph(&mut client, &engine, &root, &changes_binding);
    check_pages_and_errors(&mut client, &engine, &root, &current["generation"]);
    let stale = client.call(
        "compare_changes",
        json!({
            "base":"HEAD", "expected_comparison":changes_binding,
            "selection":{"kind":"list","offset":1,"limit":1}
        }),
    );
    assert_eq!(
        stale["structuredContent"]["error"]["code"],
        "comparison_mismatch"
    );
    let stale_class = client.call(
        "compare_class_flows",
        json!({"base":"HEAD","expected_comparison":changes_binding,"id":checkout}),
    );
    assert_eq!(
        stale_class["structuredContent"]["error"]["code"],
        "comparison_mismatch"
    );
    client.disconnect();
    client.assert_shutdown();
}

fn check_comparison_tools(client: &mut Client) {
    let reply = client.request("tools/list", json!({}));
    let tools = reply["result"]["tools"].as_array().unwrap();
    for name in ["compare_graphs", SOURCE_CHANGES.0] {
        let tool = tools.iter().find(|tool| tool["name"] == name).unwrap();
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
    }
}

fn check_source_changes_without_graphs(client: &mut Client, engine: &Path, root: &Path) {
    let entry = root.join("entry.ts");
    let original = fs::read(&entry).unwrap();
    replace_entry(root, 17);
    let notes = root.join("source-notes.txt");
    fs::write(&notes, "worktree only\n").unwrap();

    let first = compare_changes(
        client,
        (engine, root),
        SOURCE_CHANGES,
        json!({"base":"HEAD","selection":{"kind":"list","limit":1}}),
    );
    assert!(first["generation"].is_null());
    assert!(first["result"]["graph_comparison_id"].is_null());
    assert_eq!(first["result"]["total"], 2);
    let base = &first["result"]["base"]["commit"];
    let binding = &first["result"]["comparison_id"];
    let next = compare_changes(
        client,
        (engine, root),
        SOURCE_CHANGES,
        json!({
            "base":base, "expected_comparison":binding,
            "selection":{"kind":"list","offset":1,"limit":1}
        }),
    );
    let first_row = &first["result"]["changes"][0];
    let next_row = &next["result"]["changes"][0];
    let added = [first_row, next_row]
        .into_iter()
        .find(|row| row["file"] == "source-notes.txt")
        .expect("untracked file row");
    assert_eq!(added["status"], "added");
    let request = json!({
        "base":base, "expected_comparison":binding,
        "selection":{"kind":"source_diff","id":added["id"]}
    });
    let diff = compare_changes(client, (engine, root), SOURCE_CHANGES, request.clone());
    assert_eq!(diff["result"]["status"], "ready");
    assert_eq!(diff["result"]["hunks"][0]["lines"][0]["kind"], "insert");
    assert_eq!(
        diff["result"]["hunks"][0]["lines"][0]["text"],
        "worktree only\n"
    );

    replace_entry(root, 18);
    let stale = client.call("compare_source_changes", request);
    assert_eq!(
        stale["structuredContent"]["error"]["code"],
        "comparison_mismatch"
    );
    fs::write(&entry, original).unwrap();
    fs::remove_file(&notes).unwrap();
}

fn check_oversized_source_preview(client: &mut Client, engine: &Path, root: &Path) {
    let oversized = root.join("large-capture.jar");
    let file = fs::File::create(&oversized).unwrap();
    let size = 257 * 1024 * 1024;
    file.set_len(size).unwrap();
    let listing = compare_changes(
        client,
        (engine, root),
        SOURCE_CHANGES,
        json!({"base":"HEAD","selection":{"kind":"list"}}),
    );
    let result = &listing["result"];
    let row = result["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["file"] == "large-capture.jar")
        .expect("oversized file remains in the outline");
    let preview = json!({
        "base": result["base"]["commit"],
        "expected_comparison": result["comparison_id"],
        "selection": {"kind":"source_diff","id":row["id"]}
    });
    let diff = compare_changes(client, (engine, root), SOURCE_CHANGES, preview.clone());
    assert_eq!(diff["result"]["status"], "unavailable");
    assert_eq!(diff["result"]["reason"], "source_too_large");
    assert_eq!(diff["result"]["hunks"], json!([]));
    file.set_len(size + 1).unwrap();
    drop(file);
    let stale = client.call(SOURCE_CHANGES.0, preview);
    assert_eq!(
        stale["structuredContent"]["error"]["code"],
        "comparison_mismatch"
    );
    fs::remove_file(oversized).unwrap();
}

fn check_retained_baseline(client: &mut Client) {
    let baseline = client.graph("refresh_graph", json!({}));
    assert_eq!(
        baseline["result"]["retention"]["status"], "verified",
        "{baseline}"
    );
    let status = client.graph("graph_snapshot_status", json!({"revision":"HEAD"}));
    let saved = &status["result"]["snapshots"][0];
    assert_eq!(saved["id"], baseline["result"]["retention"]["snapshot"]);
    assert_eq!(
        saved["identity"]["commit"],
        baseline["result"]["retention"]["commit"]
    );
    assert_eq!(saved["digest"], baseline["generation"]);
    let selection: Value = serde_json::from_str(saved["identity"]["selection"].as_str().unwrap())
        .expect("snapshot has structured analysis metadata");
    assert_eq!(selection["builds"]["targets"][0]["language"], "typescript");
    assert!(selection["analyzers"][0]["backend"].is_string());
    assert!(selection["analyzers"][0]["version"].is_string());
}

fn services(implementation: &str) -> String {
    format!(
        "export class LegacyTax {{ static calculate() {{ return 1; }} }}\n\
         export class TaxService {{ static calculate() {{ return 2; }} }}\n\
         export class Checkout {{ static submit() {{ return {implementation}.calculate(); }} }}\n"
    )
}

fn check_class_graph(client: &mut Client, engine: &Path, root: &Path, binding: &Value) -> Value {
    let changes = compare_changes(
        client,
        (engine, root),
        GRAPH_CHANGES,
        json!({"base":"HEAD","selection":{"kind":"list"}}),
    );
    let checkout = changes["result"]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == "class" && row["name"] == "Checkout")
        .expect("changed Checkout class row")["id"]
        .clone();
    let mut request = json!({"base":"HEAD","expected_comparison":binding,"id":checkout});
    let reply = client.graph("compare_class_flows", request.clone());
    request["kind"] = json!("class_flow_comparison");
    assert_eq!(reply, cli_query(engine, root, request));
    let graph = &reply["result"]["graph"];
    assert_eq!(graph["root"], checkout);
    let links = graph["links"].as_array().unwrap();
    let nodes = graph["nodes"].as_array().unwrap();
    for (name, status, side) in [
        ("LegacyTax", "removed", "before"),
        ("TaxService", "added", "after"),
    ] {
        let node = nodes.iter().find(|node| node["name"] == name).unwrap();
        assert_eq!(node["status"], status);
        let link = links.iter().find(|link| link["to"] == node["id"]).unwrap();
        assert_eq!(link["from"], checkout);
        assert_eq!(link["status"], status);
        assert_eq!(link[side]["direct"], 1);
    }
    checkout
}

fn check_change_summary(client: &mut Client, engine: &Path, root: &Path) -> Value {
    let first = compare_changes(
        client,
        (engine, root),
        GRAPH_CHANGES,
        json!({
            "base":"HEAD", "selection":{"kind":"list","limit":1}
        }),
    );
    assert_eq!(first["result"]["returned"], 1);
    let binding = first["result"]["comparison_id"].clone();
    let next = compare_changes(
        client,
        (engine, root),
        GRAPH_CHANGES,
        json!({
            "base":"HEAD", "expected_comparison":binding,
            "selection":{"kind":"list","offset":first["result"]["next_offset"],"limit":1}
        }),
    );
    assert_eq!(next["result"]["offset"], 1);

    let complete = compare_changes(
        client,
        (engine, root),
        GRAPH_CHANGES,
        json!({
            "base":"HEAD", "selection":{"kind":"list"}
        }),
    );
    let changes = complete["result"]["changes"].as_array().unwrap();
    let changed = changes
        .iter()
        .find(|row| {
            row["kind"] == "method"
                && row["status"] == "added"
                && row["name"]
                    .as_str()
                    .is_some_and(|name| name.contains("added"))
        })
        .expect("the added function was returned");
    let source = compare_changes(
        client,
        (engine, root),
        GRAPH_CHANGES,
        json!({
            "base":"HEAD", "expected_comparison":binding,
            "selection":{"kind":"source_diff","id":changed["id"]}
        }),
    );
    assert_eq!(source["result"]["status"], "ready");
    assert!(
        source["result"]["hunks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|hunk| {
                hunk["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|line| line["kind"] == "insert")
            })
    );
    binding
}

fn compare_changes(
    client: &mut Client,
    (engine, root): (&Path, &Path),
    (tool, operation): (&str, &str),
    mut arguments: Value,
) -> Value {
    let reply = client.graph(tool, arguments.clone());
    arguments["kind"] = json!(operation);
    assert_eq!(reply, cli_query(engine, root, arguments));
    reply
}

fn replace_entry(root: &Path, value: u32) {
    fs::write(
        root.join("entry.ts"),
        format!(
            "export function changed() {{ return {value}; }}\nexport function added() {{ return changed(); }}\n"
        ),
    )
    .unwrap();
}

fn check_pages_and_errors(client: &mut Client, engine: &Path, root: &Path, current: &Value) {
    let first = client.graph("compare_graphs", json!({"base":"HEAD","limit":1}));
    assert_eq!(
        first,
        cli_query(
            engine,
            root,
            json!({"kind":"compare","base":"HEAD","limit":1})
        )
    );
    assert_eq!(&first["generation"], current);
    let mut records = first["result"]["changes"].as_array().unwrap().clone();
    let next = client.graph("compare_graphs", json!({"base":"HEAD","offset":1,"limit":1,"expected_comparison":first["result"]["comparison_id"],"expected_generation":first["generation"]}));
    records.extend(next["result"]["changes"].as_array().unwrap().clone());
    let complete = cli_query(engine, root, json!({"kind":"compare","base":"HEAD"}));
    let all = complete["result"]["changes"].as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records.as_slice(), &all[..2]);
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
    assert_eq!(
        &client.graph("graph_status", json!({"expected_generation":current}))["generation"],
        current
    );
    replace_entry(root, 8);
    let latest = client.graph("refresh_graph", json!({}));
    assert_ne!(&latest["generation"], current);
    let mixed = client.call(
        "compare_graphs",
        json!({"base":"HEAD","offset":1,"expected_comparison":first["result"]["comparison_id"]}),
    );
    assert_eq!(
        mixed["structuredContent"]["error"]["code"],
        "comparison_mismatch"
    );
    let verified = client.graph("compare_graphs", json!({"base":"HEAD","head":"HEAD"}));
    assert!(verified["freshness"].is_null());
    assert_eq!(verified["result"]["graph_equal"], true);
    assert!(!root.join(".codecallnavigator").exists());
}

#[test]
#[ignore = "requires built real CLI with Node and TypeScript"]
fn real_stdio_snapshot_preparation_and_comparison_preserve_cli_results() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap().join("repo");
    let cache = root.with_file_name("cache");
    fs::create_dir(&root).unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "test@example.invalid"]);
    git(&root, &["config", "user.name", "MCP local snapshots"]);
    fs::write(root.join("tsconfig.json"), r#"{"files":["entry.ts"]}"#).unwrap();
    fs::write(
        root.join("entry.ts"),
        "export function value() { return 1; }\n",
    )
    .unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "baseline"]);

    std::os::unix::fs::symlink(
        std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES")
            .expect("TypeScript fixture dependencies"),
        root.join("node_modules"),
    )
    .unwrap();

    let engine = engine();
    let mut launch = command(&engine, &root);
    launch.env(LIBRARY, &cache);
    let mut client = Client::launch(launch);
    let status = json!({"revision":"HEAD"});
    let miss = client.graph("graph_snapshot_status", status.clone());
    assert_eq!(miss["result"]["snapshots"], json!([]));
    assert!(miss["generation"].is_null());
    let prepared = client.request(
        "tools/call",
        json!({
            "name":"prepare_graph_snapshot",
            "arguments":{"revision":"HEAD","targets":["tsconfig.json"]},
            "_meta":{"progressToken":"preparation"},
        }),
    );
    assert_ne!(prepared["result"]["isError"], true, "{prepared}");
    let snapshot = &prepared["result"]["structuredContent"]["result"]["snapshot"];
    assert!(snapshot["id"].is_string(), "{prepared}");
    let notification = client
        .notification("notifications/progress")
        .expect("no preparation progress");
    assert_eq!(notification["params"]["progressToken"], "preparation");
    assert!(notification["params"]["message"].is_string());

    let hit = client.graph("graph_snapshot_status", status);
    assert_eq!(hit["result"]["snapshots"], json!([snapshot]));
    let mut cli = Command::new(&engine);
    cli.env(LIBRARY, &cache);
    assert_eq!(
        hit,
        cli_query_with(
            cli,
            &root,
            json!({"kind":"snapshot_status","revision":"HEAD"})
        )
    );
    let mut cli = Command::new(&engine);
    cli.env(LIBRARY, &cache);
    assert_snapshot_comparisons(&mut client, cli, &root, snapshot);
    let mut cli = Command::new(&engine);
    cli.env(LIBRARY, &cache);
    assert_snapshot_impact(&mut client, cli, &root, snapshot);
    assert!(!root.join(".codecallnavigator").exists());
    client.disconnect();
    client.assert_shutdown();
}

fn assert_snapshot_impact(client: &mut Client, cli: Command, root: &Path, snapshot: &Value) {
    fs::write(
        root.join("entry.ts"),
        "export function value() { return 2; }\n",
    )
    .unwrap();
    let arguments = json!({"base":"HEAD","base_snapshot":snapshot["id"]});
    let impact = client.graph("preview_graph_impact", arguments.clone());
    let result = &impact["result"];
    assert_eq!(result["base"]["snapshot"], *snapshot);
    assert_eq!(result["head"]["requested_ref"], "WORKTREE");
    assert_eq!(result["coverage"]["tracked_only"], true);
    assert_eq!(result["coverage"]["mapped_ranges"], 1);
    assert!(
        result["records"][0]["symbols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|symbol| symbol["label"] == "value")
    );
    assert!(impact["generation"].is_null());
    assert!(impact["graph_schema"].is_null());
    assert!(impact["freshness"].is_null());
    let mut request = arguments;
    request["kind"] = json!("impact_preview");
    assert_eq!(impact, cli_query_with(cli, root, request));
}

fn assert_snapshot_comparisons(client: &mut Client, cli: Command, root: &Path, snapshot: &Value) {
    let selectors = json!({"base":"HEAD","head":"HEAD","base_snapshot":snapshot["id"],"head_snapshot":snapshot["id"]});
    let compared = client.graph("compare_graphs", selectors.clone());
    assert_eq!(compared["result"]["graph_equal"], true);
    assert_eq!(compared["result"]["base"]["snapshot"], *snapshot);

    let mut selectors = selectors;
    selectors["selection"] = json!({"kind":"catalog"});
    let flows = client.graph("compare_function_flows", selectors.clone());
    assert_eq!(flows["result"]["head"]["snapshot"], *snapshot);
    let records = flows["result"]["records"].as_array().unwrap();
    let function = records
        .iter()
        .find(|record| record["after"]["node"]["label"] == "value")
        .expect("the cached flow catalog contains the fixture function");
    assert_eq!(function["after"]["flow"]["status"], "recorded");
    let symbol = function["symbol"].as_str().unwrap();
    selectors["expected_comparison"] = flows["result"]["comparison_id"].clone();
    selectors["selection"] = json!({"kind":"detail","symbol":symbol});
    let detailed = client.graph("compare_function_flows", selectors.clone());
    assert_eq!(detailed["result"]["after"]["status"], "ready");

    selectors["kind"] = json!("flow_comparison");
    assert_eq!(detailed, cli_query_with(cli, root, selectors));
}
