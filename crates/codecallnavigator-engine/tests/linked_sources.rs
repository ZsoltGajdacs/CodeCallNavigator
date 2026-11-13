//! Real compiler-owned linked files survive the portable machine boundary.
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn machine_preserves_linked_sources_and_human_cli_requires_declared_root() {
    let root = linked_project();
    let root = root.path().canonicalize().unwrap();
    let project = root.join("app");
    let executable = env!("CARGO_BIN_EXE_codecallnavigator-engine");
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
    let complete = machine_analysis(
        &project,
        json!({"path":project.join("Linked.csproj"), "language":"csharp", "kind":"project", "label":"Linked.csproj"}),
    );
    let roots: Vec<(String, std::path::PathBuf)> =
        serde_json::from_value(complete["roots"].clone()).unwrap();
    assert!(roots.iter().any(|(name, _)| name.starts_with("linked-")));
    let bytes = fs::read(project.join(codecallnavigator_core::artifact::GRAPH_PATH)).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains(&root.to_string_lossy().to_string()));
    let graph = codecallnavigator_core::artifact::Artifact::from_bytes(&bytes)
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
    fs::write(project.join("Linked.csproj"), "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net11.0</TargetFramework></PropertyGroup><ItemGroup><Compile Include=\"../Shared.cs\" Link=\"Shared.cs\" /></ItemGroup></Project>").unwrap();
    root
}

fn machine_analysis(project: &std::path::Path, target: Value) -> Value {
    let executable = env!("CARGO_BIN_EXE_codecallnavigator-engine");
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
    writeln!(
        child.stdin.as_mut().unwrap(),
        "{}",
        json!({"command":"analyze", "version":1,
        "project":project, "roots":[], "infer_linked_roots":true,
        "targets":[target]})
    )
    .unwrap();
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

#[test]
#[ignore = "requires pinned Java analyzer, Maven and installed JDK25"]
fn machine_external_java_module_identity_roots_relocate() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/maven");
    let mut published = Vec::new();
    for name in ["first", "second"] {
        let checkout = root.join(name);
        copy_fixture(&source, &checkout);
        if let Some(maven) = std::env::var_os("CODECALLNAVIGATOR_TEST_MAVEN") {
            use std::os::unix::fs::PermissionsExt;
            let wrapper = checkout.join("mvnw");
            let executable = maven.to_string_lossy().replace('\'', "'\\''");
            fs::write(&wrapper, format!("#!/bin/sh\nexec '{executable}' \"$@\"\n")).unwrap();
            fs::set_permissions(wrapper, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let project = checkout.join("reactor");
        let response = machine_analysis(
            &project,
            json!({
                "path":project, "language":"java", "kind":"maven", "label":"reactor"
            }),
        );
        let bytes = fs::read(project.join(codecallnavigator_core::artifact::GRAPH_PATH)).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(root.to_str().unwrap()));
        let roots: Vec<(String, std::path::PathBuf)> =
            serde_json::from_value(response["roots"].clone()).unwrap();
        let graph = codecallnavigator_core::artifact::Artifact::from_bytes(&bytes)
            .unwrap()
            .hydrate(&roots)
            .unwrap();
        assert!(
            graph
                .identity_roots
                .iter()
                .any(|root| std::path::Path::new(root) == checkout.join("library"))
        );
        published.push(bytes);
    }
    assert_eq!(published[0], published[1]);
}

fn copy_fixture(source: &std::path::Path, target: &std::path::Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if matches!(
            entry.file_name().to_str(),
            Some("target" | "build" | ".codecallnavigator" | ".gradle")
        ) {
            continue;
        }
        let to = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_fixture(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), to).unwrap();
        }
    }
}
