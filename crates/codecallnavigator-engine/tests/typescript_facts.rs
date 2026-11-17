//! Real CLI/compiler facts survive canonical identity mapping, queries and policy pruning.
#![cfg(unix)]
use codecallnavigator_core::{
    artifact::{self, Artifact},
    flow::{self, CallSite, ExecutionTiming, FunctionFlowIR, InvocationDispatch},
    method_policy,
};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs,
    io::Write,
    os::unix::fs::symlink,
    path::Path,
    process::{Command, Stdio},
};

#[path = "support/compiler_expansion.rs"]
mod compiler_expansion;

#[test]
#[ignore = "requires installed TypeScript 5.9.x or 6.0.x and Node 24+"]
fn real_typescript_facts_survive_stable_ids_cached_queries_and_excluded_targets() {
    let directory = fixture();
    let root = directory.path().canonicalize().unwrap();
    let (saved, bytes) = analyze(&root);
    let mut graph = saved.hydrate(&[("repo".into(), root.clone())]).unwrap();
    let caller = graph
        .nodes
        .iter()
        .find(|node| node.label == "run")
        .unwrap()
        .id
        .clone();
    let function = graph
        .flow_inventory
        .as_ref()
        .unwrap()
        .functions
        .iter()
        .find(|f| f.symbol == caller)
        .unwrap();
    assert_eq!(function.execution, ExecutionTiming::Synchronous);
    let original = sites(function)
        .into_iter()
        .find(|site| site.invocation.isolated_ordered)
        .unwrap();
    assert_eq!(original.invocation.dispatch, InvocationDispatch::Direct);
    assert_eq!(original.targets.len(), 1);
    let excluded = original.targets[0].symbol.clone();
    assert!(graph.nodes.iter().any(|node| node.id == excluded));
    method_policy::prune(&mut graph, &HashSet::from([excluded])).unwrap();
    let pruned = graph
        .flow_inventory
        .as_ref()
        .unwrap()
        .functions
        .iter()
        .find(|f| f.symbol == caller)
        .unwrap();
    let site = sites(pruned)
        .into_iter()
        .find(|site| site.span == original.span)
        .unwrap();
    assert_eq!(site.invocation, original.invocation);
    assert!(site.targets.is_empty());
    flow::build(pruned).unwrap();
    assert_cached_queries_and_expansion(&root, &caller, &saved, &original);
    assert_eq!(fs::read(root.join(artifact::GRAPH_PATH)).unwrap(), bytes);
}

fn assert_cached_queries_and_expansion(
    root: &Path,
    caller: &str,
    saved: &Artifact,
    original: &CallSite,
) {
    let caller_source = fs::read_to_string(root.join("main.ts")).unwrap();
    let callee_source = fs::read_to_string(root.join("leaf.ts")).unwrap();
    fs::remove_file(root.join("main.ts")).unwrap();
    fs::remove_file(root.join("leaf.ts")).unwrap();
    let response = query(root, caller);
    assert_eq!(response["result"]["function"]["execution"], "synchronous");
    assert!(
        response["result"]["source"]["content"]
            .as_str()
            .unwrap()
            .contains("猫 😀")
    );
    let queried: FunctionFlowIR =
        serde_json::from_value(response["result"]["function"].clone()).unwrap();
    assert!(
        sites(&queried)
            .iter()
            .any(|site| site.invocation == original.invocation)
    );
    compiler_expansion::assert_cached_expansion(
        saved,
        compiler_expansion::Pair {
            caller,
            call_text: "alias()",
            label: "alias",
            callee_label: "leaf",
            callee_signature: "(value?: number): number",
            content: [&caller_source, &callee_source],
        },
    );
}

fn fixture() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/flow/typescript-execution");
    for entry in fs::read_dir(fixture).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), root.join(entry.file_name())).unwrap();
    }
    symlink(
        std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES")
            .expect("Set installed TypeScript modules"),
        root.join("node_modules"),
    )
    .unwrap();
    directory
}

fn analyze(root: &Path) -> (Artifact, Vec<u8>) {
    let output = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .args(["analyze", "--project"])
        .arg(root)
        .arg("tsconfig.json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(root.join(artifact::GRAPH_PATH)).unwrap();
    let saved = artifact::load(root).unwrap();
    assert_eq!(saved.encode().unwrap().as_ref(), bytes);
    (saved, bytes)
}

fn sites(function: &FunctionFlowIR) -> Vec<CallSite> {
    let mut sites = Vec::new();
    flow::visit_calls_mut(&mut function.clone(), &mut |site| sites.push(site.clone()));
    sites
}
fn query(root: &Path, symbol: &str) -> Value {
    let mut process = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
        .arg("query")
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(
            json!({"version":2,"project":root,"query":{"kind":"flow","symbol":symbol}})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Compiler resolution must retain hidden source inputs even when broad scans
/// skip hidden folders; this needs the real TypeScript worker's file inventory.
#[test]
#[ignore = "requires installed TypeScript 5.9.x or 6.0.x and Node 24+"]
fn compiler_required_hidden_typescript_sources_remain_observed() {
    let temporary = fixture();
    let root = temporary.path().canonicalize().unwrap();
    fs::create_dir(root.join(".required")).unwrap();
    fs::rename(root.join("leaf.ts"), root.join(".required/leaf.ts")).unwrap();
    let source = fs::read_to_string(root.join("main.ts")).unwrap();
    fs::write(
        root.join("main.ts"),
        source.replace("'./leaf'", "'./.required/leaf'"),
    )
    .unwrap();
    let config = fs::read_to_string(root.join("tsconfig.json")).unwrap();
    fs::write(
        root.join("tsconfig.json"),
        config.replace("\"leaf.ts\"", "\".required/leaf.ts\""),
    )
    .unwrap();
    let (saved, _) = analyze(&root);
    let analysis = artifact::provenance::read(&saved.graph).unwrap().unwrap();
    assert!(
        analysis
            .inputs
            .iter()
            .any(|input| input.path == "@repo/.required/leaf.ts")
    );
    fs::write(
        root.join(".required/leaf.ts"),
        "export function leaf(value = 0) { return value + 2; }",
    )
    .unwrap();
    let report = artifact::verification::verify(&root, &[("repo".into(), root.clone())]).unwrap();
    assert_eq!(
        report.input_status,
        artifact::verification::InputStatus::Stale
    );
    assert!(
        report
            .reasons
            .iter()
            .any(|reason| reason.path.as_deref() == Some("@repo/.required/leaf.ts"))
    );
}
