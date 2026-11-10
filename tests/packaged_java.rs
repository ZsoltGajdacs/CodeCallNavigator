//! Exercise the analyzer resource layout without using repository-local adapter paths.
use codegraph_engine::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::BuildTarget,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if matches!(name.to_str(), Some("target" | "build" | ".gradle")) {
            continue;
        }
        let target = destination.join(name);
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn packaged_runtime(root: &Path) -> AnalyzerRuntime {
    let resources = root.join("CodeGraph.app/Contents/Resources");
    let analyzers = resources.join("analyzers");
    fs::create_dir_all(&analyzers).unwrap();
    fs::copy(
        repository().join("tools/maven-metadata/build/libs/codegraph-maven-metadata.jar"),
        analyzers.join("maven-metadata.jar"),
    )
    .unwrap();
    fs::copy(
        repository().join("tools/gradle-metadata.gradle"),
        analyzers.join("gradle-metadata.gradle"),
    )
    .unwrap();
    copy_tree(
        &repository().join("tools/java-bytecode/build/install/codegraph-bytecode/lib"),
        &analyzers.join("java-bytecode/lib"),
    );
    let mut runtime = AnalyzerRuntime::discover().unwrap();
    runtime.bundle_root = resources;
    runtime
}

fn target(root: PathBuf, kind: &str) -> BuildTarget {
    BuildTarget {
        path: root.canonicalize().unwrap(),
        label: "packaged fixture".into(),
        language: "java".into(),
        kind: kind.into(),
    }
}

#[test]
#[ignore = "requires bootstrapped analyzer artifacts, Maven, Gradle wrapper and JDK 25"]
fn packaged_resources_analyze_maven_and_gradle_without_development_paths() {
    let temporary = tempfile::Builder::new()
        .prefix("CodeGraph packaged resources ")
        .tempdir()
        .unwrap();
    let runtime = packaged_runtime(temporary.path());
    let maven = temporary.path().join("maven fixture");
    let gradle = temporary.path().join("gradle fixture");
    copy_tree(&repository().join("fixtures/maven"), &maven);
    copy_tree(&repository().join("fixtures/gradle"), &gradle);
    let graph = analyze(
        vec![
            target(maven.join("standalone"), "maven"),
            target(gradle, "gradle"),
        ],
        &runtime,
        &AtomicBool::new(false),
        &|message| eprintln!("{message}"),
    )
    .unwrap();
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.signature.contains("fixture.Standalone:main("))
    );
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.signature.contains("fixture.Flow:main("))
    );
    assert_eq!(graph.entry_points.unwrap().application_starts.len(), 2);
    assert_eq!(graph.roots.len(), 2);
}

#[test]
#[ignore = "requires bootstrapped analyzer artifacts and JDK 25"]
fn missing_packaged_extension_does_not_fall_back_to_development_artifacts() {
    let temporary = tempfile::tempdir().unwrap();
    let runtime = packaged_runtime(temporary.path());
    fs::remove_file(runtime.bundle_root.join("analyzers/maven-metadata.jar")).unwrap();
    let maven = temporary.path().join("maven fixture");
    copy_tree(&repository().join("fixtures/maven"), &maven);
    let error = analyze(
        vec![target(maven.join("standalone"), "maven")],
        &runtime,
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("Analyzer artifacts missing"),
        "{error:#}"
    );
}
