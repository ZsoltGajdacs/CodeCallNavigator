//! The public query interface is usable without a desktop or compiler process.
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn query(request: Value) -> (i32, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("query")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

#[test]
fn capabilities_and_missing_graph_are_machine_readable() {
    let output = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("capabilities")
        .output()
        .unwrap();
    assert!(output.status.success());
    let capabilities: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(capabilities["query_version"], 1);
    let root = tempfile::tempdir().unwrap();
    let (code, result) =
        query(json!({"version":1,"project":root.path(),"query":{"kind":"status"}}));
    assert_eq!(code, 2);
    assert_eq!(result["error"]["code"], "graph_missing");
}

fn saved_graph(root: &Path) -> Vec<u8> {
    use codecallnavigator_core::{
        artifact,
        model::{Edge, EntryPoints, GraphData, Node},
    };
    let file = root.join("Example.java").to_string_lossy().into_owned();
    fs::write(
        &file,
        "class Example { void Start() { Leaf(); } void Leaf() {} }\r\n",
    )
    .unwrap();
    let node = |label: &str| Node {
        id: label.into(),
        label: label.into(),
        language: "java".into(),
        kind: "method".into(),
        build: root.join("pom.xml").to_string_lossy().into_owned(),
        project: "Example".into(),
        container: "Example".into(),
        file: file.clone(),
        line: 1,
        ..Default::default()
    };
    let graph = GraphData {
        nodes: vec![node("Start"), node("Leaf")],
        edges: vec![Edge {
            from: 0,
            to: 1,
            kind: "call".into(),
            certainty: "possible".into(),
            provenance: "fixture".into(),
            count: 1,
            evidence: vec![],
        }],
        entry_points: Some(EntryPoints {
            candidates: vec!["Start".into()],
            ..Default::default()
        }),
        architecture: Some(source_fixture(root, &file)),
        flow_inventory: Some(flow_fixture(root, file)),
        ..Default::default()
    };
    let artifact = artifact::canonical(&graph, &[("repo".into(), root.into())]).unwrap();
    let bytes = artifact.bytes().unwrap();
    fs::create_dir(root.join(".codecallnavigator")).unwrap();
    fs::write(root.join(artifact::GRAPH_PATH), &bytes).unwrap();
    bytes
}

fn source_fixture(
    root: &Path,
    file: &str,
) -> codecallnavigator_core::architecture::ArchitectureFacts {
    use codecallnavigator_core::architecture::{ArchitectureFacts, FileFacts};
    ArchitectureFacts {
        version: 1,
        files: vec![FileFacts {
            file: file.into(),
            build: root.join("pom.xml").to_string_lossy().into_owned(),
            project: "Example".into(),
            language: "java".into(),
            content: fs::read_to_string(file).unwrap(),
            ..Default::default()
        }],
    }
}

fn flow_fixture(root: &Path, file: String) -> codecallnavigator_core::flow::FlowInventory {
    use codecallnavigator_core::flow::{
        FlowInventory, FunctionFlowIR, SourceSpan, Statement, StatementKind,
    };
    FlowInventory {
        version: 1,
        functions: vec![FunctionFlowIR {
            symbol: "Start".into(),
            file,
            build: root.join("pom.xml").to_string_lossy().into_owned(),
            project: "Example".into(),
            language: "java".into(),
            span: SourceSpan { start: 0, end: 54 },
            body: vec![Statement {
                span: SourceSpan { start: 0, end: 4 },
                kind: StatementKind::Action {
                    text: "Leaf()".into(),
                    calls: vec![],
                },
            }],
            ..Default::default()
        }],
    }
}

fn ask(root: &Path, operation: Value) -> Value {
    let (code, response) = query(json!({"version":1,"project":root,"query":operation}));
    assert_eq!(code, 0, "{response}");
    response
}

#[test]
fn queries_preserve_stable_ids_uncertainty_source_and_bounds() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let before = saved_graph(&root);
    let search = ask(
        &root,
        json!({"kind":"search","text":"Start","category":"methods"}),
    );
    let symbol = search["result"]["hits"][0]["target"]["symbol"]
        .as_str()
        .unwrap();
    assert!(symbol.contains("@repo/"));
    let section = ask(
        &root,
        json!({"kind":"selection","selection":{"kind":"neighborhood","symbols":[symbol],"direction":"Outgoing","depth":2}}),
    );
    assert_eq!(section["result"]["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(section["result"]["edges"][0]["from"], symbol);
    assert_eq!(section["result"]["edges"][0]["certainty"], "possible");
    assert_eq!(section["result"]["edges"][0]["provenance"], "fixture");
    let mut filter =
        serde_json::to_value(codecallnavigator_core::graph::Filter::default()).unwrap();
    filter["relationship_kinds"] = json!([]);
    let filtered = ask(
        &root,
        json!({"kind":"selection","selection":{"kind":"overview"},"filter":filter}),
    );
    assert!(filtered["result"]["edges"].as_array().unwrap().is_empty());
    let bounded = ask(
        &root,
        json!({"kind":"selection","selection":{"kind":"neighborhood","symbols":[symbol],"direction":"Outgoing","depth":2},"limits":{"nodes":1,"edges":1}}),
    );
    assert_eq!(bounded["result"]["nodes"][0]["id"], symbol);
    assert_eq!(bounded["result"]["omitted_nodes"], 1);
    assert_eq!(bounded["result"]["omitted_edges"], 1);
    let analyzed = ask(&root, json!({"kind":"source","symbol":symbol}));
    assert!(
        analyzed["result"]["content"]
            .as_str()
            .unwrap()
            .contains("\r\n")
    );
    let live = ask(
        &root,
        json!({"kind":"source","symbol":symbol,"origin":"live"}),
    );
    assert!(!live["result"]["content"].as_str().unwrap().contains("\r\n"));
    let flow = ask(&root, json!({"kind":"flow","symbol":symbol}));
    assert_eq!(flow["result"]["chart"]["symbol"], symbol);
    assert!(flow["result"]["chart"]["steps"].as_array().unwrap().len() > 1);
    assert_eq!(
        fs::read(root.join(codecallnavigator_core::artifact::GRAPH_PATH)).unwrap(),
        before
    );
    let (code, result) = query(
        json!({"version":1,"project":root,"expected_generation":"wrong","query":{"kind":"status"}}),
    );
    assert_eq!(code, 2);
    assert_eq!(result["error"]["code"], "generation_mismatch");
}

#[test]
fn explorer_includes_hidden_entries_and_does_not_follow_symlinks() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    saved_graph(&root);
    fs::write(root.join(".hidden"), "x").unwrap();
    fs::create_dir(root.join("directory")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("directory"), root.join("link")).unwrap();
    let result = ask(&root, json!({"kind":"explorer","path":root,"limit":100}));
    let entries = result["result"]["entries"].as_array().unwrap();
    assert!(entries.iter().any(|e| e["label"] == ".hidden"));
    #[cfg(unix)]
    assert!(
        entries
            .iter()
            .any(|e| e["label"] == "link" && e["kind"] == "symlink")
    );
    let (code, result) = query(
        json!({"version":1,"project":root,"query":{"kind":"explorer","path":root.parent().unwrap()}}),
    );
    assert_eq!(code, 2);
    assert_eq!(result["error"]["code"], "outside_project");
}

#[test]
fn invalid_requests_and_unsupported_flow_remain_explicit() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let bytes = saved_graph(&root);
    let mut artifact = codecallnavigator_core::artifact::Artifact::from_bytes(&bytes).unwrap();
    for node in artifact.graph["nodes"].as_array_mut().unwrap() {
        node["language"] = "csharp".into();
    }
    fs::write(
        root.join(codecallnavigator_core::artifact::GRAPH_PATH),
        artifact.bytes().unwrap(),
    )
    .unwrap();
    let snapshot = ask(&root, json!({"kind":"snapshot"}));
    let symbol = snapshot["result"]["nodes"][0]["id"].as_str().unwrap();
    let (code, response) =
        query(json!({"version":1,"project":root,"query":{"kind":"flow","symbol":symbol}}));
    assert_eq!(code, 2);
    assert_eq!(response["error"]["code"], "flow_unavailable");
    assert!(
        response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("TASK-22")
    );
    let (code, response) = query(json!({"version":2,"project":root,"query":{"kind":"status"}}));
    assert_eq!(code, 2);
    assert_eq!(response["error"]["code"], "unsupported_version");
    let (code, response) = query(
        json!({"version":1,"project":root,"query":{"kind":"search","text":"Start","limit":201}}),
    );
    assert_eq!(code, 2);
    assert_eq!(response["error"]["code"], "invalid_request");
    let (code, response) =
        query(json!({"version":1,"project":root,"query":{"kind":"symbol","symbol":"unknown"}}));
    assert_eq!(code, 2);
    assert_eq!(response["error"]["code"], "symbol_missing");
}

fn large_class(root: &Path) {
    use codecallnavigator_core::{
        artifact,
        model::{GraphData, Node},
    };
    let graph = GraphData {
        nodes: (0..1502)
            .map(|i| Node {
                id: format!("method-{i}"),
                label: format!("method{i}"),
                language: "java".into(),
                kind: "method".into(),
                container: "LargeClass".into(),
                build: root.join("pom.xml").to_string_lossy().into_owned(),
                file: root.join("LargeClass.java").to_string_lossy().into_owned(),
                project: "Example".into(),
                is_test: i == 0,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };
    let artifact = artifact::canonical(&graph, &[("repo".into(), root.into())]).unwrap();
    fs::create_dir(root.join(".codecallnavigator")).unwrap();
    fs::write(root.join(artifact::GRAPH_PATH), artifact.bytes().unwrap()).unwrap();
}

#[test]
fn render_membership_is_complete_while_agent_selection_is_bounded() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    large_class(&root);
    let status = ask(&root, json!({"kind":"status"}));
    let bounded = ask(
        &root,
        json!({"kind":"selection","selection":{"kind":"overview"}}),
    );
    assert_eq!(bounded["result"]["nodes"].as_array().unwrap().len(), 1500);
    assert_eq!(bounded["result"]["omitted_nodes"], 1);
    let (code, render) = query(
        json!({"version":1,"project":root,"expected_generation":status["generation"],"query":{"kind":"render_selection","selection":{"kind":"overview"}}}),
    );
    assert_eq!(code, 0, "{render}");
    assert_eq!(render["result"]["nodes"].as_array().unwrap().len(), 1501);
    assert_eq!(render["result"]["omitted_nodes"], 0);
    assert_eq!(render["result"]["truncated"], false);
    assert!(
        render["result"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|node| node["is_test"] == false)
    );
}

#[test]
fn class_search_does_not_expand_all_member_identities() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    large_class(&root);
    let search = ask(
        &root,
        json!({"kind":"search","text":"LargeClass","category":"classes","limit":1}),
    );
    let target = &search["result"]["hits"][0]["target"];
    assert_eq!(target["member_count"], 1502);
    assert!(target.get("symbols").is_none());
    assert!(serde_json::to_vec(target).unwrap().len() < 1000);
}

#[test]
#[ignore = "requires TypeScript 6 compiler and Node 24 installed"]
fn real_typescript_analysis_is_queryable_without_reanalysis() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let modules = std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES").expect(
        "Set CODECALLNAVIGATOR_TS_NODE_MODULES to pinned TypeScript/React/Next dependencies",
    );
    #[cfg(unix)]
    std::os::unix::fs::symlink(modules, root.join("node_modules")).unwrap();
    fs::write(
        root.join("tsconfig.json"),
        r#"{"compilerOptions":{"target":"ES2022","types":[]},"include":["example.ts"]}"#,
    )
    .unwrap();
    fs::write(root.join("example.ts"),"export function leaf(): number { return 1; }\nexport function start(value: boolean): number { if(value) return leaf(); return 0; }\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .args(["analyze", "--project"])
        .arg(&root)
        .arg("tsconfig.json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let before = fs::read(root.join(codecallnavigator_core::artifact::GRAPH_PATH)).unwrap();
    let search = ask(
        &root,
        json!({"kind":"search","text":"start","category":"methods"}),
    );
    let symbol = search["result"]["hits"][0]["target"]["symbol"]
        .as_str()
        .unwrap();
    let flow = ask(&root, json!({"kind":"flow","symbol":symbol}));
    assert_eq!(flow["result"]["chart"]["partial"], false);
    assert!(
        flow["result"]["chart"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["kind"] == "decision")
    );
    let neighborhood = ask(
        &root,
        json!({"kind":"selection","selection":{"kind":"neighborhood","symbols":[symbol],"direction":"Outgoing","depth":1}}),
    );
    assert!(
        !neighborhood["result"]["edges"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    fs::write(root.join("example.ts"), "export function changed() {}\n").unwrap();
    assert_eq!(
        ask(&root, json!({"kind":"status"}))["freshness"]["input_status"],
        "stale"
    );
    assert_eq!(
        before,
        fs::read(root.join(codecallnavigator_core::artifact::GRAPH_PATH)).unwrap()
    );
}
