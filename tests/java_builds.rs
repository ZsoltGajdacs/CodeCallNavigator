//! Real build-tool acceptance checks.
#[path = "support/java.rs"]
mod support;
use codecallnavigator_engine::{
    analyzers::{AnalyzerRuntime, analyze},
    fingerprint,
    model::{GraphData, Node},
    store::SqliteStore,
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use support::{fixture, install_wrapper, run, target};

fn method<'a>(graph: &'a GraphData, class: &str, method: &str) -> &'a Node {
    graph
        .nodes
        .iter()
        .find(|node| {
            node.signature
                .contains(&format!("fixture.{class}:{method}("))
        })
        .unwrap_or_else(|| panic!("Missing {class}:{method}"))
}

fn assert_project_graph(graph: &GraphData) {
    let main = method(graph, "App", "main");
    let service = method(graph, "Service", "work");
    let generated = method(graph, "GeneratedThing", "generated");
    let test = method(graph, "AppTest", "testOnly");
    let disconnected = method(graph, "App", "disconnected");
    for name in [
        "constructorOnly",
        "packageMethod",
        "protectedMethod",
        "toString",
    ] {
        method(graph, "App", name);
    }
    for fragment in [
        ":<init>(",
        ":<clinit>(",
        ":toString()",
        ":hashCode()",
        ":equals(java.lang.Object)",
    ] {
        assert!(
            graph
                .nodes
                .iter()
                .all(|node| !node.signature.contains(fragment)),
            "Unexpected {fragment}"
        );
    }
    assert_ne!(main.project, service.project);
    assert_eq!(main.project, generated.project);
    assert_eq!(main.project, test.project);
    assert!(!main.is_test && !main.is_generated);
    assert!(test.is_test && !test.is_generated);
    assert!(generated.is_generated && !generated.is_test);
    assert!(Path::new(&main.file).is_file());
    assert!(main.line > 0);
    let entries = graph.entry_points.as_ref().unwrap();
    assert!(entries.application_starts.contains(&main.id));
    assert!(entries.candidates.contains(&disconnected.id));
    let main_index = graph
        .nodes
        .iter()
        .position(|node| node.id == main.id)
        .unwrap() as u32;
    let service_index = graph
        .nodes
        .iter()
        .position(|node| node.id == service.id)
        .unwrap() as u32;
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.from == main_index && edge.to == service_index)
    );
    assert!(
        graph
            .nodes
            .iter()
            .all(|node| node.namespace.starts_with("fixture"))
    );
}

#[test]
#[ignore = "requires pinned Java analyzer, Gradle wrapper and JDK 25"]
fn gradle_refactoring_preserves_module_graph_and_classification() {
    let fixture = fixture("gradle-regression");
    let wrapper = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/gradle/gradlew");
    install_wrapper(fixture.path(), &wrapper);
    let build = target(fixture.path(), "gradle");
    let first = run(vec![build.clone()]).unwrap();
    assert_project_graph(&first);
    assert!(!fixture.path().join("tests-executed").exists());
    let second = run(vec![build]).unwrap();
    let ids = |graph: &GraphData| {
        graph
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&first), ids(&second));
    assert_eq!(first.entry_points, second.entry_points);
}

fn maven_fixture() -> tempfile::TempDir {
    let fixture = fixture("maven");
    if let Some(command) = std::env::var_os("CODECALLNAVIGATOR_TEST_MAVEN") {
        install_wrapper(fixture.path(), &PathBuf::from(command));
    }
    fixture
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven and JDK 25"]
fn maven_reactor_preserves_graph_ownership_and_does_not_execute_tests() {
    let fixture = maven_fixture();
    let graph = run(vec![target(&fixture.path().join("reactor"), "maven")]).unwrap();
    assert_project_graph(&graph);
    let generated = method(&graph, "GeneratedThing", "generated");
    assert!(
        graph
            .fingerprints
            .iter()
            .any(|item| item.path == generated.file)
    );
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.signature.contains("fixture.ProfileOnly:"))
    );
    assert!(
        !graph
            .nodes
            .iter()
            .any(|node| node.signature.contains("fixture.Inactive:"))
    );
    assert!(!fixture.path().join("reactor/app/tests-executed").exists());
    assert!(!fingerprint::is_stale(&graph.fingerprints).unwrap());
    let resource = fixture
        .path()
        .join("resources/fixture.txt")
        .canonicalize()
        .unwrap();
    let resource_fingerprint = graph
        .fingerprints
        .iter()
        .find(|item| Path::new(&item.path) == resource)
        .expect("External resource contents must be fingerprinted");
    fs::write(resource, "edited external resource input").unwrap();
    assert!(fingerprint::is_stale(std::slice::from_ref(resource_fingerprint)).unwrap());
    let parent = fixture
        .path()
        .join("parent/pom.xml")
        .canonicalize()
        .unwrap();
    let parent_fingerprint = graph
        .fingerprints
        .iter()
        .find(|item| Path::new(&item.path) == parent)
        .expect("External parent POM must be fingerprinted");
    fs::write(parent, "changed").unwrap();
    assert!(fingerprint::is_stale(std::slice::from_ref(parent_fingerprint)).unwrap());
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven and JDK 25"]
fn maven_standalone_changes_and_failed_refresh_preserve_cached_graph() {
    let fixture = maven_fixture();
    let root = fixture.path().join("standalone");
    let build = target(&root, "maven");
    let graph = run(vec![build.clone()]).unwrap();
    let main = method(&graph, "Standalone", "main");
    assert!(
        graph
            .entry_points
            .as_ref()
            .unwrap()
            .application_starts
            .contains(&main.id)
    );
    let cache = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(cache.path()).unwrap();
    let generation = store.publish(&graph, &AtomicBool::new(false)).unwrap();
    fs::write(
        root.join("src/main/java/fixture/Standalone.java"),
        "invalid Java",
    )
    .unwrap();
    assert!(fingerprint::is_stale(&graph.fingerprints).unwrap());
    assert!(run(vec![build]).is_err());
    let loaded = store.load_active().unwrap().unwrap();
    assert_eq!(loaded.nodes.len(), graph.nodes.len());
    assert!(cache.path().join(generation).exists());
}

#[test]
#[ignore = "requires Maven and the bundled metadata extension"]
fn overlapping_maven_reactors_fail_before_compilation() {
    use std::sync::atomic::Ordering;
    let fixture = maven_fixture();
    let root = fixture.path().join("reactor");
    let compiled = AtomicBool::new(false);
    let error = analyze(
        vec![target(&root, "maven"), target(&root.join("app"), "maven")],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|message| {
            if message.contains("Compiling") {
                compiled.store(true, Ordering::Relaxed);
            }
        },
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").to_lowercase().contains("overlap"),
        "{error:#}"
    );
    assert!(!compiled.load(Ordering::Relaxed));
    assert!(!root.join("app/target/classes/fixture/App.class").exists());
    assert!(
        !root
            .join("app/target/generated-sources/fixture/fixture/GeneratedThing.java")
            .exists()
    );
    assert!(
        !fixture
            .path()
            .join("library/target/bytecode/main/fixture/Service.class")
            .exists()
    );
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven and JDK 25"]
fn maven_project_configuration_activates_profiles_and_is_fingerprinted() {
    let fixture = maven_fixture();
    fs::create_dir(fixture.path().join(".mvn")).unwrap();
    let config = fixture.path().join(".mvn/maven.config");
    fs::write(&config, "-Pinactive\n").unwrap();
    let graph = run(vec![target(&fixture.path().join("reactor"), "maven")]).unwrap();
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.signature.contains("fixture.Inactive:"))
    );
    assert!(
        !graph
            .nodes
            .iter()
            .any(|node| node.signature.contains("fixture.ProfileOnly:"))
    );
    assert!(!fingerprint::is_stale(&graph.fingerprints).unwrap());
    fs::write(config, "-Pfixture-default\n").unwrap();
    assert!(fingerprint::is_stale(&graph.fingerprints).unwrap());
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven and JDK 25"]
fn maven_rejects_authored_edits_during_compilation_and_honors_cancellation() {
    use std::sync::atomic::Ordering;
    let fixture = maven_fixture();
    let root = fixture.path().join("standalone");
    let source = root.join("src/main/java/fixture/Standalone.java");
    let build = target(&root, "maven");
    let runtime = AnalyzerRuntime::discover().unwrap();
    let edited = AtomicBool::new(false);
    let result = analyze(
        vec![build.clone()],
        &runtime,
        &AtomicBool::new(false),
        &|message| {
            if message.contains("Compiling") && !edited.swap(true, Ordering::Relaxed) {
                let mut content = fs::read_to_string(&source).unwrap();
                content.push_str("\n// concurrent authored edit\n");
                fs::write(&source, content).unwrap();
            }
        },
    );
    assert!(
        edited.load(Ordering::Relaxed),
        "Compilation checkpoint was not observed"
    );
    let error = format!("{:#}", result.unwrap_err());
    assert!(error.contains("changed during analysis"), "{error}");
    let cancel = AtomicBool::new(false);
    let result = analyze(vec![build], &runtime, &cancel, &|message| {
        if message.contains("Compiling") {
            cancel.store(true, Ordering::Relaxed);
        }
    });
    assert!(cancel.load(Ordering::Relaxed));
    assert!(
        format!("{:#}", result.unwrap_err())
            .to_lowercase()
            .contains("cancel")
    );
}

#[test]
#[ignore = "requires JDK 25 and bundled metadata extension"]
fn maven_success_without_metadata_is_rejected() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = fixture("maven");
    let root = fixture.path().join("standalone");
    let wrapper = root.join("mvnw");
    fs::write(&wrapper, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let error = format!("{:#}", run(vec![target(&root, "maven")]).unwrap_err());
    assert!(error.to_lowercase().contains("metadata"), "{error}");
    assert!(!root.join("target").exists());
}

#[test]
#[ignore = "requires Maven, JDK 25 and bundled metadata extension"]
fn maven_skipped_compilation_rejects_missing_expected_outputs() {
    let fixture = maven_fixture();
    let root = fixture.path().join("standalone");
    let pom = root.join("pom.xml");
    let content = fs::read_to_string(&pom).unwrap().replace(
        "</project>",
        "<build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-compiler-plugin</artifactId><configuration><skipMain>true</skipMain></configuration></plugin></plugins></build></project>",
    );
    fs::write(pom, content).unwrap();
    let error = format!("{:#}", run(vec![target(&root, "maven")]).unwrap_err());
    assert!(error.contains("compiled Java output"), "{error}");
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven and JDK 25"]
fn maven_architecture_facts_preserve_graph_and_source_annotations() {
    let fixture = maven_fixture();
    let build = target(&fixture.path().join("standalone"), "maven");
    let before = run(vec![build.clone()]).unwrap();
    let source = fixture
        .path()
        .join("standalone/src/main/java/fixture/Standalone.java");
    let original = fs::read_to_string(&source).unwrap();
    let annotated = original
        .replace(
            "public final class Standalone",
            "@Architecture public final class Standalone",
        )
        .replace(
            "public static void leaf()",
            "@Architecture public static void leaf()",
        );
    fs::write(&source, format!("{annotated}\n@java.lang.annotation.Retention(java.lang.annotation.RetentionPolicy.SOURCE) @interface Architecture {{}}\ninterface Contract {{}}\nrecord Data(String value) {{}}\n")).unwrap();
    let graph = run(vec![build]).unwrap();
    let facts = graph.architecture.as_ref().unwrap();
    assert_eq!(facts.version, 1);
    let file = facts
        .files
        .iter()
        .find(|file| file.file.ends_with("/fixture/Standalone.java"))
        .unwrap();
    assert_eq!(
        (
            file.annotations.contains(&"fixture.Architecture".into()),
            file.method_annotations.contains(&"Architecture".into()),
            file.names.contains(&"Data".into()),
            file.kinds.contains(&"record".into()),
            file.content.contains("RetentionPolicy.SOURCE")
        ),
        (true, true, true, true, true),
        "Source declarations must survive bytecode erasure"
    );
    let ids = |graph: &GraphData| {
        graph
            .nodes
            .iter()
            .filter(|node| node.container == "fixture.Standalone")
            .map(|node| node.id.clone())
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(ids(&before), ids(&graph));
    assert_eq!(before.edges.len(), graph.edges.len());
    let cache = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(cache.path()).unwrap();
    store.publish(&graph, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        store.load_active().unwrap().unwrap().architecture,
        graph.architecture
    );
}
