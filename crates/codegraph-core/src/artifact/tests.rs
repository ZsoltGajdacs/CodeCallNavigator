use super::*;
use crate::model::{Edge, EntryPoints, GraphData, Node};
use serde_json::json;
use std::{fs, process::Command, sync::atomic::AtomicBool};

fn fixture(root: &str) -> GraphData {
    let node = |name: &str| Node {
        id: format!("{root}/App.csproj|[App]::Program::{name}()"),
        project: root.rsplit('/').next().unwrap().into(),
        build: format!("{root}/App.csproj"),
        language: "csharp".into(),
        kind: "method".into(),
        label: name.into(),
        signature: format!("[App]::Program::{name}()"),
        container: "Program".into(),
        file: format!("{root}/Program.cs"),
        ..Node::default()
    };
    let nodes = vec![node("Start"), node("Leaf")];
    let entries = EntryPoints {
        candidates: vec![nodes[0].id.clone()],
        ..EntryPoints::default()
    };
    GraphData {
        nodes,
        edges: vec![Edge {
            from: 0,
            to: 1,
            kind: "call".into(),
            certainty: "direct".into(),
            provenance: "compiler".into(),
            count: 1,
        }],
        roots: vec![format!("{root}/App.csproj")],
        entry_points: Some(entries),
        ..GraphData::default()
    }
}

fn roots(root: &str) -> Vec<(String, PathBuf)> {
    vec![("repo".into(), root.into())]
}

#[test]
fn relocating_checkout_and_reordering_nodes_preserves_bytes() {
    let before = canonical(&fixture("/tmp/first"), &roots("/tmp/first")).unwrap();
    let mut graph = fixture("/tmp/second");
    graph.nodes.swap(0, 1);
    graph.edges[0].from = 1;
    graph.edges[0].to = 0;
    let after = canonical(&graph, &roots("/tmp/second")).unwrap();
    assert_eq!(before.bytes().unwrap(), after.bytes().unwrap());
    let hydrated = after.hydrate(&roots("/tmp/local")).unwrap();
    assert!(
        hydrated
            .nodes
            .iter()
            .all(|n| n.file == "/tmp/local/Program.cs")
    );
}

#[test]
fn semantic_edge_attributes_and_same_count_topology_changes_are_visible() {
    let graph = fixture("/tmp/a");
    let before = canonical(&graph, &roots("/tmp/a")).unwrap();
    let mut graph = graph;
    graph.edges[0].count = 3;
    graph.edges[0].certainty = "possible".into();
    let changed = canonical(&graph, &roots("/tmp/a")).unwrap();
    let result = compare(&before, &changed).unwrap();
    assert!(!result.equal);
    assert_eq!(result.edges.changed.len(), 1);
    graph.edges[0].to = 0;
    let moved = canonical(&graph, &roots("/tmp/a")).unwrap();
    let result = compare(&before, &moved).unwrap();
    assert_eq!(result.edges.added.len(), 1);
    assert_eq!(result.edges.removed.len(), 1);
}

#[test]
fn invalid_artifacts_and_unmapped_paths_are_rejected() {
    let artifact = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
    let mut invalid = artifact.clone();
    invalid.schema = "future".into();
    assert!(invalid.bytes().is_err());
    let mut invalid = artifact.clone();
    invalid.graph["edges"][0]["to"] = "missing".into();
    assert!(invalid.validate().is_err());
    let mut invalid = artifact;
    let duplicate = invalid.graph["nodes"][0].clone();
    invalid.graph["nodes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(invalid.validate().is_err());
    assert!(canonical(&fixture("/tmp/a"), &roots("/tmp/b")).is_err());
}

#[test]
fn supplied_typescript_semantic_keys_remain_distinct_across_files() {
    let mut graph = fixture("/tmp/a");
    graph.nodes[0].language = "typescript".into();
    graph.nodes[1].language = "typescript".into();
    graph.nodes[0].file = "/tmp/a/first.ts".into();
    graph.nodes[1].file = "/tmp/a/second.ts".into();
    let keys = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), "function:leaf:():number".into()))
        .collect();
    let artifact = canonical_with_identities(&graph, &roots("/tmp/a"), &keys).unwrap();
    assert_ne!(
        artifact.graph["nodes"][0]["id"],
        artifact.graph["nodes"][1]["id"]
    );
    assert!(artifact.validate().is_ok());
}

#[test]
fn cancellation_preserves_the_only_project_artifact_and_configuration() {
    let project = tempfile::tempdir().unwrap();
    let artifact = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
    fs::create_dir(project.path().join(".codegraph")).unwrap();
    fs::write(project.path().join(".codegraph/layers.yml"), "version: 1\n").unwrap();
    publish(project.path(), &artifact, &AtomicBool::new(false)).unwrap();
    let before = fs::read(project.path().join(GRAPH_PATH)).unwrap();
    let mut changed = artifact;
    changed.graph["nodes"][0]["label"] = "changed".into();
    assert!(publish(project.path(), &changed, &AtomicBool::new(true)).is_err());
    assert_eq!(before, fs::read(project.path().join(GRAPH_PATH)).unwrap());
    assert_eq!(
        fs::read_to_string(project.path().join(".codegraph/layers.yml")).unwrap(),
        "version: 1\n"
    );
    assert_eq!(
        fs::read_dir(project.path().join(".codegraph"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|e| e == "json"))
            .count(),
        1
    );
}

#[test]
fn whole_analysis_lock_excludes_other_writers_until_released() {
    let project = tempfile::tempdir().unwrap();
    let artifact = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
    let owner = AnalysisLock::acquire(project.path()).unwrap();
    assert!(AnalysisLock::acquire(project.path()).is_err());
    owner.publish(&artifact, &AtomicBool::new(false)).unwrap();
    assert!(AnalysisLock::acquire(project.path()).is_err());
    drop(owner);
    assert!(AnalysisLock::acquire(project.path()).is_ok());
}

#[test]
fn source_text_is_preserved_and_control_flow_order_is_not_sorted() {
    let mut graph = fixture("/tmp/a");
    graph.flow_inventory = Some(
        serde_json::from_value(json!({"version":1,"functions":[{
            "symbol":graph.nodes[0].id, "file":"/tmp/a/Program.cs", "build":"/tmp/a/App.csproj",
            "body":[{"kind":"action","text":"z"},{"kind":"action","text":"a"}]
        }]}))
        .unwrap(),
    );
    graph.architecture = Some(serde_json::from_value(json!({"version":1,"files":[{
        "build":"/tmp/a/App.csproj", "file":"/tmp/a/Program.cs", "content":"literal /tmp/a/source",
        "names":["z","a"]
    }]})).unwrap());
    let artifact = canonical(&graph, &roots("/tmp/a")).unwrap();
    assert_eq!(
        artifact.graph["architecture"]["files"][0]["content"],
        "literal /tmp/a/source"
    );
    assert_eq!(
        artifact.graph["architecture"]["files"][0]["names"],
        json!(["a", "z"])
    );
    assert_eq!(
        artifact.graph["flow_inventory"]["functions"][0]["body"][0]["text"],
        "z"
    );
}

#[test]
fn git_comparison_reads_history_without_writing_snapshots() {
    let project = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(project.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "test@example.invalid"]);
    git(&["config", "user.name", "Artifact test"]);
    let artifact = canonical(&fixture("/tmp/a"), &roots("/tmp/a")).unwrap();
    publish(project.path(), &artifact, &AtomicBool::new(false)).unwrap();
    git(&["add", GRAPH_PATH]);
    git(&["commit", "-qm", "baseline"]);
    assert!(
        compare_git(project.path(), "HEAD", "WORKTREE")
            .unwrap()
            .equal
    );
    let mut changed = artifact;
    changed.graph["nodes"][0]["label"] = "changed".into();
    publish(project.path(), &changed, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        compare_git(project.path(), "HEAD", "WORKTREE")
            .unwrap()
            .nodes
            .changed
            .len(),
        1
    );
    assert!(compare_git(project.path(), "missing-revision", "WORKTREE").is_err());
    let files: Vec<_> = fs::read_dir(project.path().join(".codegraph"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(files.len(), 2); // graph.json and the persistent advisory lock
}
