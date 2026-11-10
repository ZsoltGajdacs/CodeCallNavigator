use codegraph_engine::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::BuildTarget,
    model::GraphData,
};
use std::{fs, path::Path, sync::atomic::AtomicBool};
pub fn fixture(name: &str) -> tempfile::TempDir {
    let temp = tempfile::Builder::new()
        .prefix("codegraph Java fixture ")
        .tempdir()
        .unwrap();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(name),
        temp.path(),
    );
    temp
}

pub fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if matches!(name.to_str(), Some("target" | "build" | ".gradle")) {
            continue;
        }
        let to = destination.join(name);
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), to).unwrap();
        }
    }
}

pub fn target(root: &Path, kind: &str) -> BuildTarget {
    BuildTarget {
        path: root.canonicalize().unwrap(),
        language: "java".into(),
        kind: kind.into(),
        label: "acceptance".into(),
    }
}

pub fn run(builds: Vec<BuildTarget>) -> anyhow::Result<GraphData> {
    analyze(
        builds,
        &AnalyzerRuntime::discover()?,
        &AtomicBool::new(false),
        &|message| eprintln!("{message}"),
    )
}

pub fn install_wrapper(root: &Path, command: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let path = root.join(if command.file_name().unwrap() == "gradlew" {
        "gradlew"
    } else {
        "mvnw"
    });
    // Quote a literal executable path, preserving the actual Maven/Gradle arguments unchanged.
    let quoted = command.to_string_lossy().replace('\'', "'\\''");
    fs::write(&path, format!("#!/bin/sh\nexec '{quoted}' \"$@\"\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
