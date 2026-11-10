//! Real compiler-owned linked files survive the portable machine boundary.
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 10 SDK"]
fn machine_preserves_linked_sources_and_human_cli_requires_declared_root() {
    let root = linked_project();
    let root = root.path().canonicalize().unwrap();
    let project = root.join("app");
    let executable = env!("CARGO_BIN_EXE_codegraph-engine");
    let strict = Command::new(executable)
        .args(["analyze", "--project"])
        .arg(&project)
        .arg("Linked.csproj")
        .output()
        .unwrap();
    assert_eq!(strict.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&strict.stderr).contains("outside declared roots"),
        "{}",
        String::from_utf8_lossy(&strict.stderr)
    );
    let explicit = Command::new(executable)
        .args(["analyze", "--project"])
        .arg(&project)
        .args(["--root", "shared=..", "Linked.csproj"])
        .output()
        .unwrap();
    assert!(
        explicit.status.success(),
        "{}",
        String::from_utf8_lossy(&explicit.stderr)
    );
    let complete = machine_analysis(&project);
    let roots: Vec<(String, std::path::PathBuf)> =
        serde_json::from_value(complete["roots"].clone()).unwrap();
    assert!(roots.iter().any(|(name, _)| name.starts_with("linked-")));
    let bytes = fs::read(project.join(codegraph_core::artifact::GRAPH_PATH)).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains(&root.to_string_lossy().to_string()));
    let graph = codegraph_core::artifact::Artifact::from_bytes(&bytes)
        .unwrap()
        .hydrate(&roots)
        .unwrap();
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.file == root.join("Shared.cs").to_string_lossy())
    );
}

fn linked_project() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("app");
    fs::create_dir(&project).unwrap();
    fs::write(root.path().join("Shared.cs"), "public class Shared { public static void Leaf() {} public static void Start() { Leaf(); } }").unwrap();
    fs::write(project.join("Linked.csproj"), "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup><ItemGroup><Compile Include=\"../Shared.cs\" Link=\"Shared.cs\" /></ItemGroup></Project>").unwrap();
    root
}

fn machine_analysis(project: &std::path::Path) -> Value {
    let executable = env!("CARGO_BIN_EXE_codegraph-engine");
    let mut child = Command::new(executable)
        .arg("serve-once")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&line).unwrap()["event"],
        "hello"
    );
    writeln!(child.stdin.as_mut().unwrap(), "{}", json!({"command":"analyze", "version":1,
        "project":project, "roots":[], "infer_linked_roots":true,
        "targets":[{"path":project.join("Linked.csproj"), "language":"csharp", "kind":"project", "label":"Linked.csproj"}]})).unwrap();
    child.stdin.as_mut().unwrap().flush().unwrap();
    let mut complete = None;
    loop {
        line.clear();
        if output.read_line(&mut line).unwrap() == 0 {
            break;
        }
        let event: Value = serde_json::from_str(&line).unwrap();
        assert_ne!(event["event"], "failed", "{event}");
        if event["event"] == "complete" {
            complete = Some(event);
        }
    }
    assert!(child.wait().unwrap().success());
    complete.expect("machine must publish a graph")
}
