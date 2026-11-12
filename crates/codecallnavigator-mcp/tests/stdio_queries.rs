//! Exercise only the installed executable boundary; no application library imports.
mod support;
use serde_json::{Value, json};
use std::{fs, path::Path};
use support::{Client, cli_query, command, engine, typescript};

fn tools(client: &mut Client) {
    let response = client.request("tools/list", json!({}));
    let listed = response["result"]["tools"].as_array().unwrap();
    let names = listed
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    for expected in [
        "graph_status",
        "find_symbols",
        "symbol_details",
        "graph_section",
        "neighbors",
        "symbol_source",
        "function_flow",
        "explore_directory",
        "discover_builds",
        "refresh_graph",
    ] {
        assert!(names.contains(&expected), "Missing {expected}: {names:?}");
    }
    assert!(!names.contains(&"render_selection"));
}

fn selected_start(client: &mut Client) -> (String, Value) {
    let search = client.graph("find_symbols", json!({"text":"start","category":"methods"}));
    let hits = search["result"]["hits"].as_array().unwrap();
    assert_eq!(
        hits.len(),
        2,
        "Same-named exports must remain ambiguous: {search}"
    );
    for hit in hits {
        let symbol = hit["target"]["symbol"].as_str().unwrap();
        let details = client.graph("symbol_details", json!({"symbol":symbol}));
        if details["result"]["file"]
            .as_str()
            .unwrap()
            .ends_with("entry.ts")
        {
            return (symbol.into(), search);
        }
    }
    panic!("entry.ts start was not returned");
}

fn compare_queries(client: &mut Client, engine: &Path, root: &Path, symbol: &str) {
    let generation = client.graph("graph_status", json!({}))["generation"].clone();
    for (name, arguments, operation) in [
        ("graph_status", json!({}), json!({"kind":"status"})),
        (
            "symbol_details",
            json!({"symbol":symbol}),
            json!({"kind":"symbol","symbol":symbol}),
        ),
        (
            "symbol_source",
            json!({"symbol":symbol}),
            json!({"kind":"source","symbol":symbol}),
        ),
        (
            "function_flow",
            json!({"symbol":symbol}),
            json!({"kind":"flow","symbol":symbol}),
        ),
        (
            "neighbors",
            json!({"symbol":symbol}),
            json!({"kind":"neighbors","symbol":symbol}),
        ),
    ] {
        let mut bound = arguments;
        bound["expected_generation"] = generation.clone();
        assert_eq!(
            client.graph(name, bound),
            cli_query(engine, root, operation),
            "{name} changed CLI semantics"
        );
    }
    let section = client.graph("graph_section",json!({"selection":{"kind":"neighborhood","symbols":[symbol],"direction":"Outgoing","depth":2},"limits":{"nodes":1,"edges":1},"expected_generation":generation}));
    assert_eq!(section["result"]["truncated"], true);
    assert_eq!(section["result"]["nodes"].as_array().unwrap().len(), 1);
}

fn failed_refresh_preserves_graph(client: &mut Client, root: &Path) {
    let generation = client.graph("graph_status", json!({}))["generation"].clone();
    let graph = root.join(".codecallnavigator/graph.json");
    let before = fs::read(&graph).unwrap();
    fs::write(root.join("entry.ts"), "const invalid: number = 'broken';\n").unwrap();
    let failure = client.call("refresh_graph", json!({}));
    assert_eq!(failure["isError"], true, "{failure}");
    let envelope = &failure["structuredContent"];
    assert_eq!(envelope["error"]["code"], "analysis_failed", "{failure}");
    assert_eq!(envelope["cli_event"]["event"], "failed");
    assert_eq!(
        envelope["error"]["message"],
        envelope["cli_event"]["message"]
    );
    assert!(
        !envelope["cli_event"]["message"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    assert_eq!(fs::read(&graph).unwrap(), before);
    let status = client.graph("graph_status", json!({"expected_generation":generation}));
    assert_eq!(status["generation"], generation);
    assert_eq!(status["freshness"]["input_status"], "stale");
}

#[test]
#[ignore = "requires built real CLI, Node24 and pinned TypeScript6 modules"]
fn real_stdio_refresh_queries_and_errors_preserve_the_cli_contract() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let external = tempfile::tempdir().unwrap();
    fs::write(external.path().join("visible.txt"), "external root").unwrap();
    typescript(&root);
    let engine = engine();
    let mut configured = command(&engine, &root);
    configured
        .arg("--root")
        .arg(format!("external={}", external.path().display()));
    let mut client = Client::launch(configured);
    tools(&mut client);
    let missing = client.call("graph_status", json!({}));
    assert_eq!(missing["isError"], true);
    assert_eq!(
        missing["structuredContent"]["error"]["code"],
        "graph_missing"
    );
    assert!(!root.join(".codecallnavigator/graph.json").exists());
    let directory = client.graph("explore_directory", json!({"path":external.path()}));
    assert_eq!(directory["result"]["entries"][0]["label"], "visible.txt");
    let discovered = client.call("discover_builds", json!({}));
    assert_ne!(discovered["isError"], true, "{discovered}");
    let refresh = client.call("refresh_graph", json!({}));
    assert_ne!(refresh["isError"], true, "{refresh}");
    let (symbol, _) = selected_start(&mut client);
    // Comparison uses matching configured roots through a separate no-root client.
    drop(client);
    let mut client = Client::launch(command(&engine, &root));
    compare_queries(&mut client, &engine, &root, &symbol);
    let original = client.graph("symbol_source", json!({"symbol":symbol}));
    let previous = original["generation"].clone();
    fs::write(
        root.join("entry.ts"),
        "export function start(): number { return 3; }\n",
    )
    .unwrap();
    assert_eq!(
        client.graph("graph_status", json!({}))["freshness"]["input_status"],
        "stale"
    );
    let analyzed = client.graph("symbol_source", json!({"symbol":symbol}));
    let live = client.graph("symbol_source", json!({"symbol":symbol,"origin":"live"}));
    assert_eq!(analyzed["result"]["content"], original["result"]["content"]);
    assert_ne!(live["result"]["content"], analyzed["result"]["content"]);
    let unknown = client.call("symbol_details", json!({"symbol":"missing"}));
    assert_eq!(
        unknown["structuredContent"]["error"]["code"],
        "symbol_missing"
    );
    assert_ne!(client.call("refresh_graph", json!({}))["isError"], true);
    let stale_generation = client.call("graph_status", json!({"expected_generation":previous}));
    assert_eq!(
        stale_generation["structuredContent"]["error"]["code"],
        "generation_mismatch"
    );
    assert!(client.graph("graph_status", json!({}))["generation"].is_string());
    failed_refresh_preserves_graph(&mut client, &root);
}
