//! CLI operations preserve complete desktop information before presentation.
use codecallnavigator_core::{
    architecture::{ArchitectureFacts, FileFacts},
    artifact,
    model::{Edge, GraphData, Node},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn query(root: &Path, generation: Option<&str>, operation: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("query")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let request =
        json!({"version":1,"project":root,"expected_generation":generation,"query":operation});
    write!(child.stdin.take().unwrap(), "{request}").unwrap();
    let output = child.wait_with_output().unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

fn save(root: &Path, count: u32) -> String {
    let file = root.join("Example.java").to_string_lossy().into_owned();
    fs::write(&file, "class Example {}\r\n").unwrap();
    let build = root.join("pom.xml").to_string_lossy().into_owned();
    let nodes = (0..count)
        .map(|id| Node {
            id: format!("method{id}"),
            label: format!("M{id:04}"),
            language: "java".into(),
            kind: "method".into(),
            file: file.clone(),
            build: build.clone(),
            project: "example".into(),
            container: "Example".into(),
            line: 1,
            ..Default::default()
        })
        .collect();
    let graph = GraphData {
        nodes,
        edges: fixture_edges(count),
        architecture: Some(ArchitectureFacts {
            version: 1,
            files: vec![FileFacts {
                file,
                build,
                project: "example".into(),
                language: "java".into(),
                content: "class Example {}\r\n".into(),
                ..Default::default()
            }],
        }),
        ..Default::default()
    };
    let bytes = artifact::canonical(&graph, &[("repo".into(), root.into())])
        .unwrap()
        .bytes()
        .unwrap();
    fs::create_dir(root.join(".codecallnavigator")).unwrap();
    fs::write(root.join(artifact::GRAPH_PATH), &bytes).unwrap();
    blake3::hash(&bytes).to_hex().to_string()
}

fn fixture_edges(count: u32) -> Vec<Edge> {
    let mut edges = (0..count)
        .map(|to| Edge {
            from: 0,
            to,
            kind: "call".into(),
            certainty: "exact".into(),
            provenance: "first".into(),
            count: 1,
            evidence: vec![],
        })
        .collect::<Vec<_>>();
    edges.push(Edge {
        from: 0,
        to: 1,
        kind: "call".into(),
        certainty: "possible".into(),
        provenance: "parallel".into(),
        count: 2,
        evidence: vec![],
    });
    edges.push(Edge {
        from: 1,
        to: 2,
        kind: "call".into(),
        certainty: "exact".into(),
        provenance: "neighbor-only".into(),
        count: 1,
        evidence: vec![],
    });
    edges
}

#[test]
fn inspector_and_render_metadata_preserve_large_graph_and_parallel_relationships() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let generation = save(root, 1602);
    let snapshot = query(root, Some(&generation), json!({"kind":"snapshot"}));
    let symbol = snapshot["result"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["label"] == "M0000")
        .unwrap()["id"]
        .as_str()
        .unwrap();
    let inspect = query(
        root,
        Some(&generation),
        json!({"kind":"inspect","symbol":symbol}),
    );
    assert!(inspect["error"].is_null(), "{inspect}");
    assert_eq!(
        inspect["result"]["relationships"].as_array().unwrap().len(),
        1603
    );
    assert_eq!(
        inspect["result"]["snapshot_edges"]
            .as_array()
            .unwrap()
            .len(),
        1603
    );
    assert!(
        !inspect["result"]["relationships"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["provenance"] == "neighbor-only")
    );
    let render = query(
        root,
        Some(&generation),
        json!({"kind":"render_selection","selection":{"kind":"overview"}}),
    );
    assert_eq!(
        render["result"]["snapshot_nodes"].as_array().unwrap().len(),
        1602
    );
    assert!(
        render["result"]["summary"]["cyclic_symbols"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == symbol)
    );
}

#[test]
fn live_file_and_architecture_operations_remain_explicit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let file = root.join("before-analysis.txt");
    fs::write(&file, "live\r\n").unwrap();
    let live = query(root, None, json!({"kind":"live_source","path":file}));
    assert!(live["error"].is_null(), "{live}");
    assert_eq!(live["result"]["content"], "live\n");
    let generation = save(root, 3);
    fs::write(
        root.join(".codecallnavigator/layers.yml"),
        "version: 1\nlanguages: {}\n",
    )
    .unwrap();
    let layers = query(root, Some(&generation), json!({"kind":"architecture"}));
    assert!(layers["error"].is_null(), "{layers}");
    assert_eq!(layers["result"]["selection"], "Repository");
    assert!(layers["result"]["assignments"]["files"].is_object());
    let same = query(
        root,
        Some(&generation),
        json!({"kind":"architecture","previous_signature":layers["result"]["signature"]}),
    );
    assert_eq!(same["result"]["changed"], false);
    assert!(same["result"]["assignments"].is_null());
    fs::write(
        root.join(".codecallnavigator/layers.yml"),
        "version: 1\nlanguages:\n  java: {}\n",
    )
    .unwrap();
    let changed = query(
        root,
        Some(&generation),
        json!({"kind":"render_selection","selection":{"kind":"overview"},"layer":{"expected_signature":layers["result"]["signature"]}}),
    );
    assert_eq!(changed["error"]["code"], "architecture_changed");
}

#[test]
fn flow_rejects_source_ranges_outside_the_analyzed_snapshot() {
    use codecallnavigator_core::flow::{FlowInventory, FunctionFlowIR, SourceSpan};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    save(root, 3);
    let path = root.join(artifact::GRAPH_PATH);
    let mut saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let symbol = saved["graph"]["nodes"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    saved["graph"]["flow_inventory"] = serde_json::to_value(FlowInventory {
        version: 1,
        functions: vec![FunctionFlowIR {
            symbol: symbol.clone(),
            file: "@repo/Example.java".into(),
            build: "@repo/pom.xml".into(),
            project: "example".into(),
            language: "java".into(),
            span: SourceSpan {
                start: 0,
                end: 1000000,
            },
            ..Default::default()
        }],
    })
    .unwrap();
    fs::write(path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let response = query(root, None, json!({"kind":"flow","symbol":symbol}));
    assert_eq!(response["error"]["code"], "flow_unavailable", "{response}");
}

#[test]
fn architecture_session_cache_distinguishes_configuration_roots() {
    use std::io::{BufRead, BufReader};
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let root = first.path();
    let generation = save(root, 3);
    let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("query-session")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut exchange = |configuration_root: &Path| {
        let request = json!({"version":1,"project":root,"expected_generation":generation,"query":{"kind":"architecture","selection":"Spring","configuration_root":configuration_root}});
        writeln!(input, "{request}").unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        serde_json::from_str::<Value>(&line).unwrap()
    };
    let original = exchange(root);
    let other = exchange(second.path());
    let restored = exchange(root);
    assert!(original["error"].is_null(), "{original}");
    assert!(other["error"].is_null(), "{other}");
    assert_ne!(
        original["result"]["signature"],
        other["result"]["signature"]
    );
    assert_eq!(
        original["result"]["signature"],
        restored["result"]["signature"]
    );
    drop(exchange);
    drop(input);
    assert!(child.wait().unwrap().success());
}

#[test]
fn review_rules_are_observed_without_creating_configuration_and_render_uses_the_same_root() {
    let project = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let generation = save(project.path(), 3);
    fs::create_dir(config.path().join(".interactive-review")).unwrap();
    fs::write(
        config.path().join(".interactive-review/layers.yml"),
        "java: {}\n",
    )
    .unwrap();
    let layers = query(
        project.path(),
        Some(&generation),
        json!({"kind":"architecture","configuration_root":config.path()}),
    );
    assert!(layers["error"].is_null(), "{layers}");
    assert!(
        !config.path().join(".codecallnavigator").exists(),
        "Read-only query imported configuration onto disk"
    );
    let render = query(
        project.path(),
        Some(&generation),
        json!({"kind":"render_selection","selection":{"kind":"overview"},"layer":{"configuration_root":config.path(),"expected_signature":layers["result"]["signature"]}}),
    );
    assert!(render["error"].is_null(), "{render}");
    assert_eq!(
        render["result"]["snapshot_nodes"].as_array().unwrap().len(),
        3
    );
    assert!(!config.path().join(".codecallnavigator").exists());
}

#[test]
fn global_diagnostics_are_bounded_with_explicit_omissions() {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    save(root, 3);
    let path = root.join(artifact::GRAPH_PATH);
    let mut saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    saved["graph"]["diagnostics"] = json!(vec!["🦀".repeat(1000); 100]);
    fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let status = query(root, None, json!({"kind":"status"}));
    assert!(serde_json::to_vec(&status).unwrap().len() < 40000);
    let characters: usize = status["result"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|message| message.as_str().unwrap().chars().count())
        .sum();
    assert!(characters <= 8192);
    assert_eq!(status["result"]["diagnostics_total"], 100);
    assert!(status["result"]["omitted_diagnostics"].as_u64().unwrap() > 0);
    assert!(status["result"]["shortened_diagnostics"].as_u64().unwrap() > 0);
    let snapshot = query(root, None, json!({"kind":"snapshot"}));
    assert_eq!(
        snapshot["result"]["diagnostics"].as_array().unwrap().len(),
        100
    );
}

#[test]
fn query_freshness_is_bounded_while_verify_retains_the_complete_report() {
    use codecallnavigator_core::{artifact::provenance, fingerprint};
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    save(root, 3);
    let paths = (0..100)
        .map(|id| root.join(format!("{}-{id}.txt", "x".repeat(209))))
        .collect::<Vec<_>>();
    for path in &paths {
        fs::write(path, "before").unwrap();
    }
    let inputs = fingerprint::capture_excluding(paths.clone(), &[]).unwrap();
    let graph_path = root.join(artifact::GRAPH_PATH);
    let mut saved = artifact::Artifact::from_bytes(&fs::read(&graph_path).unwrap()).unwrap();
    saved.graph["analysis"] = serde_json::to_value(
        provenance::build(&inputs, &[], "", &[("repo".into(), root.into())]).unwrap(),
    )
    .unwrap();
    fs::write(&graph_path, saved.bytes().unwrap()).unwrap();
    for path in &paths {
        fs::write(path, "after").unwrap();
    }
    let status = query(root, None, json!({"kind":"status"}));
    assert_eq!(status["freshness"]["input_status"], "stale");
    assert!(status["freshness"]["reasons"].as_array().unwrap().len() <= 50);
    assert!(
        serde_json::to_vec(&status["freshness"]["reasons"])
            .unwrap()
            .len()
            <= 8192
    );
    let output = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .args(["verify", "--project"])
        .arg(root)
        .output()
        .unwrap();
    let full: Value = serde_json::from_slice(&output.stdout).unwrap();
    let total = full["reasons"].as_array().unwrap().len();
    assert!(total > 50);
    assert_eq!(status["freshness"]["reasons_total"], total);
    assert_eq!(
        status["freshness"]["omitted_reasons"],
        total - status["freshness"]["reasons"].as_array().unwrap().len()
    );
}

#[test]
fn snapshot_preserves_recorded_analyzer_versions() {
    use codecallnavigator_core::artifact::provenance;
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    save(root, 3);
    let path = root.join(artifact::GRAPH_PATH);
    let mut saved = artifact::Artifact::from_bytes(&fs::read(&path).unwrap()).unwrap();
    saved.graph["analysis"] = serde_json::to_value(
        provenance::build(
            &[],
            &[],
            r#"{"java":"25","adapter":"fixture"}"#,
            &[("repo".into(), root.into())],
        )
        .unwrap(),
    )
    .unwrap();
    fs::write(path, saved.bytes().unwrap()).unwrap();
    let snapshot = query(root, None, json!({"kind":"snapshot"}));
    let versions: Value =
        serde_json::from_str(snapshot["result"]["engine_versions"].as_str().unwrap()).unwrap();
    assert_eq!(versions["java"], "25");
}
