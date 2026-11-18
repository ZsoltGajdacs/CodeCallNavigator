//! Real build-tool acceptance checks.
#[path = "support/java_boundaries.rs"]
mod boundaries;
#[path = "support/java.rs"]
mod support;
use codecallnavigator_engine::{
    analyzers::{AnalyzerRuntime, PreparedAnalysis, analyze},
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
    for (node, is_test, is_generated) in [
        (main, false, false),
        (test, true, false),
        (generated, false, true),
    ] {
        assert_eq!(
            (&node.project, node.is_test, node.is_generated),
            (&main.project, is_test, is_generated)
        );
    }
    assert!(Path::new(&main.file).is_file() && main.line > 0);
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
    let produced = boundaries::gradle_producer_and_cache(fixture.path());
    assert!(
        !fixture
            .path()
            .join("app/build/generated/sources/fixture")
            .exists()
    );
    let build = target(fixture.path(), "gradle");
    let first = run(vec![build.clone()]).unwrap();
    assert!(
        fs::read_to_string(produced)
            .unwrap()
            .contains("generated()")
    );
    method(&first, "Produced", "generated");
    let class = fixture
        .path()
        .join("app/build/classes/java/main/fixture/App.class");
    let modified = fs::metadata(&class).unwrap().modified().unwrap();
    let actions = fs::read(fixture.path().join("app/build/compile-actions.txt")).unwrap();
    assert_project_graph(&first);
    assert!(!fixture.path().join("tests-executed").exists());
    let second = run(vec![build.clone()]).unwrap();
    assert_eq!(fs::metadata(&class).unwrap().modified().unwrap(), modified);
    assert_eq!(
        fs::read(fixture.path().join("app/build/compile-actions.txt")).unwrap(),
        actions
    );
    assert_gradle_cache_and_late_mutation(fixture.path(), &build);
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

fn assert_gradle_cache_and_late_mutation(
    root: &Path,
    build: &codecallnavigator_engine::discovery::BuildTarget,
) {
    let classes = root.join("app/build/classes/java/main");
    // The prior native build returned; this TempDir owns these inactive outputs.
    fs::remove_dir_all(&classes).unwrap();
    let restored = run(vec![build.clone()]).unwrap();
    assert_project_graph(&restored);
    let outcomes = fs::read_to_string(root.join("build/native-outcomes.txt")).unwrap();
    assert!(
        outcomes.contains(":app:compileJava\tUP-TO-DATE"),
        "{outcomes}"
    );
    assert!(
        outcomes.contains(":app:compileJava\tFROM-CACHE"),
        "{outcomes}"
    );
    assert!(
        outcomes.contains(":library:compileTestJava\tNO-SOURCE"),
        "{outcomes}"
    );
    let script = boundaries::gradle_late_source_mutation(root);
    let source = root.join("app/src/main/java/fixture/App.java");
    let before = fs::read(&source).unwrap();
    let modified = fs::metadata(&source).unwrap().modified().unwrap();
    let error = run(vec![build.clone()]).unwrap_err();
    assert_input_change_diagnostic(&error, &source);
    assert_equal_metadata_source_mutation(&source, &before, modified);
    fs::write(root.join("build.gradle"), script).unwrap();
    fs::write(source, before).unwrap();
    assert_project_graph(&run(vec![build.clone()]).unwrap());
}

fn assert_input_change_diagnostic(error: &anyhow::Error, source: &Path) {
    assert!(error.is::<fingerprint::InputsChanged>(), "{error:#}");
    assert!(format!("{error:#}").contains(source.to_str().unwrap()));
}

fn assert_equal_metadata_source_mutation(
    source: &Path,
    before: &[u8],
    modified: std::time::SystemTime,
) {
    let after = fs::read(source).unwrap();
    assert_eq!(
        (
            after.len(),
            fs::metadata(source).unwrap().modified().unwrap()
        ),
        (before.len(), modified)
    );
    assert_ne!(
        after, before,
        "The native late-mutation probe must change consumed bytes"
    );
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
    let root = fixture.path().join("reactor");
    let produced = boundaries::maven_producer(&root);
    assert!(!root.join("app/target/generated-sources/fixture").exists());
    count_native_builds(&root);
    let build = target(&root, "maven");
    let graph = run(vec![build.clone()]).unwrap();
    assert!(
        fs::read_to_string(produced)
            .unwrap()
            .contains("generated()")
    );
    method(&graph, "Produced", "generated");
    let class = root.join("app/target/classes/fixture/GeneratedThing.class");
    let modified = fs::metadata(&class).unwrap().modified().unwrap();
    let invocations = native_build_count(&root);
    let repeated = run(vec![build.clone()]).unwrap();
    assert_project_graph(&repeated);
    assert!(
        native_build_count(&root) > invocations,
        "The native lifecycle must actually run"
    );
    assert_eq!(
        fs::metadata(class).unwrap().modified().unwrap(),
        modified,
        "A no-change Maven invocation must retain its native compiler outputs"
    );
    assert_project_graph(&graph);
    let generated = method(&graph, "GeneratedThing", "generated");
    assert!(
        graph
            .fingerprints
            .iter()
            .any(|item| item.path == generated.file)
    );
    let profile = graph
        .nodes
        .iter()
        .any(|node| node.signature.contains("fixture.ProfileOnly:"));
    let inactive = graph
        .nodes
        .iter()
        .any(|node| node.signature.contains("fixture.Inactive:"));
    assert_eq!((profile, inactive), (true, false));
    assert!(!fixture.path().join("reactor/app/tests-executed").exists());
    assert!(!fingerprint::is_stale(&graph.fingerprints).unwrap());
    assert_late_generated_rewrite_preserves_graph(&root, &build, &graph);
    assert_maven_required_inputs(fixture.path(), &graph);
}

fn assert_late_generated_rewrite_preserves_graph(
    root: &Path,
    build: &codecallnavigator_engine::discovery::BuildTarget,
    graph: &GraphData,
) {
    let cache = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(cache.path()).unwrap();
    let generation = store.publish(graph, &AtomicBool::new(false)).unwrap();
    let artifact = cache.path().join(generation);
    let saved = fs::read(&artifact).unwrap();
    let source = root.join("app/target/generated-sources/fixture/fixture/GeneratedThing.java");
    let before = fs::read(&source).unwrap();
    let modified = fs::metadata(&source).unwrap().modified().unwrap();
    let original = boundaries::maven_late_generated_mutation(root);
    let error = run(vec![build.clone()]).unwrap_err();
    assert_input_change_diagnostic(&error, &source);
    assert_equal_metadata_source_mutation(&source, &before, modified);
    assert_eq!(fs::read(artifact).unwrap(), saved);
    assert_eq!(store.load_active().unwrap().unwrap().nodes, graph.nodes);
    fs::write(root.join("app/pom.xml"), original).unwrap();
    fs::write(source, before).unwrap();
    fs::remove_file(root.join("app/SourceMutation.java")).unwrap();
}

fn assert_maven_required_inputs(fixture: &Path, graph: &GraphData) {
    let resource = fixture
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
    let parent = fixture.join("parent/pom.xml").canonicalize().unwrap();
    let parent_fingerprint = graph
        .fingerprints
        .iter()
        .find(|item| Path::new(&item.path) == parent)
        .expect("External parent POM must be fingerprinted");
    fs::write(parent, "changed").unwrap();
    assert!(fingerprint::is_stale(std::slice::from_ref(parent_fingerprint)).unwrap());
}

/// Risk: a native producer rewrites a compiler-required non-Java input. Only two
/// real compiler adapters in one selected analysis can prove preparation order.
#[test]
#[ignore = "requires Maven, JDK 25, pinned analyzers and a supported project TypeScript compiler"]
fn maven_generation_precedes_selected_typescript_consumer() {
    let fixture = maven_fixture();
    let root = fixture.path().join("standalone");
    let config = boundaries::maven_json_producer(&root);
    let typescript = codecallnavigator_engine::discovery::target_from_file(&config).unwrap();
    // The consumer is deliberately first; build selection order is not execution order.
    let graph = run(vec![typescript, target(&root, "maven")]).unwrap();
    method(&graph, "Standalone", "main");
    let produced_kind = graph
        .nodes
        .iter()
        .find(|node| node.language == "typescript" && node.label == "producedKind")
        .unwrap();
    assert_eq!(produced_kind.signature, "(): number");
    let generated = root.join("frontend/value.json");
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&generated).unwrap()).unwrap();
    assert_eq!(value["kind"], 1);
    assert!(
        graph
            .fingerprints
            .iter()
            .any(|item| Path::new(&item.path) == generated)
    );
    assert!(!fingerprint::is_stale(&graph.fingerprints).unwrap());
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven and JDK 25"]
fn maven_standalone_changes_and_failed_refresh_preserve_cached_graph() {
    let fixture = maven_fixture();
    let root = fixture.path().join("standalone");
    let other = maven_fixture();
    let other_root = other.path().join("standalone");
    distinguish_selected_project(&other_root);
    count_native_builds(&root);
    count_native_builds(&other_root);
    let mut session = PreparedAnalysis::new(
        vec![target(&root, "maven"), target(&other_root, "maven")],
        AnalyzerRuntime::discover().unwrap(),
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let progress = |message| eprintln!("{message}");
    session.prepare(&cancel, &progress).unwrap();
    let graph = session.extract(&cancel, &progress).unwrap();
    assert_prepared_extraction_reuses_builds(&mut session, &root, &other_root, &graph);
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
    assert_only_changed_build_is_prepared(&mut session, &root, &other_root);
    assert_changed_bytecode_rejects_extraction(&mut session, &root);
    fs::write(
        root.join("src/main/java/fixture/Standalone.java"),
        "invalid Java",
    )
    .unwrap();
    assert!(fingerprint::is_stale(&graph.fingerprints).unwrap());
    assert!(session.prepare(&cancel, &progress).is_err());
    let loaded = store.load_active().unwrap().unwrap();
    assert_eq!(loaded.nodes.len(), graph.nodes.len());
    assert!(cache.path().join(generation).exists());
}

// These counters observe real native invocations, including no-op lifecycle runs.
// A class-file timestamp alone cannot prove that Maven was never launched.
fn count_native_builds(root: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let wrapper = root.join("mvnw");
    let executable = if wrapper.is_file() {
        // Keep its executable/shebang semantics and original working directory.
        let native = root.join("mvnw-native");
        fs::rename(&wrapper, &native).unwrap();
        native
    } else {
        root.ancestors()
            .skip(1)
            .map(|directory| directory.join("mvnw"))
            .find(|candidate| candidate.is_file())
            .unwrap_or_else(|| PathBuf::from("mvn"))
    };
    let quoted = executable.to_string_lossy().replace('\'', "'\\''");
    fs::write(
        &wrapper,
        format!("#!/bin/sh\nmkdir -p target\nprintf 'invocation\\n' >> target/analysis-invocations\nexec '{quoted}' \"$@\"\n"),
    )
    .unwrap();
    fs::set_permissions(wrapper, fs::Permissions::from_mode(0o755)).unwrap();
}

fn native_build_count(root: &Path) -> usize {
    fs::read_to_string(root.join("target/analysis-invocations"))
        .unwrap()
        .lines()
        .count()
}

fn distinguish_selected_project(root: &Path) {
    let pom = root.join("pom.xml");
    fs::write(
        &pom,
        fs::read_to_string(&pom).unwrap().replace(
            "<artifactId>standalone</artifactId>",
            "<artifactId>other</artifactId>",
        ),
    )
    .unwrap();
    let source = root.join("src/main/java/fixture/Standalone.java");
    fs::write(
        &source,
        fs::read_to_string(&source)
            .unwrap()
            .replace("package fixture;", "package other;"),
    )
    .unwrap();
}

fn assert_prepared_extraction_reuses_builds(
    session: &mut PreparedAnalysis,
    first: &Path,
    second: &Path,
    graph: &GraphData,
) {
    let counts = [native_build_count(first), native_build_count(second)];
    assert!(counts.iter().all(|count| *count > 0));
    let cancel = AtomicBool::new(false);
    let repeated = session.extract(&cancel, &|_| {}).unwrap();
    assert_eq!(graph.nodes, repeated.nodes);
    assert_eq!(
        serde_json::to_value(&graph.edges).unwrap(),
        serde_json::to_value(&repeated.edges).unwrap()
    );
    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(session.extract(&cancel, &|_| {}).is_err());
    cancel.store(false, std::sync::atomic::Ordering::Relaxed);
    let retried = session.extract(&cancel, &|_| {}).unwrap();
    assert_eq!(graph.nodes, retried.nodes);
    assert_eq!(
        counts,
        [native_build_count(first), native_build_count(second)]
    );
}

fn assert_only_changed_build_is_prepared(
    session: &mut PreparedAnalysis,
    first: &Path,
    second: &Path,
) {
    let counts = [native_build_count(first), native_build_count(second)];
    let source = first.join("src/main/java/fixture/Standalone.java");
    fs::write(
        &source,
        fs::read_to_string(&source)
            .unwrap()
            .replace("void leaf()", "void revisedLeaf()")
            .replace("leaf();", "revisedLeaf();"),
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    assert!(session.extract(&cancel, &|_| {}).is_err());
    session.prepare(&cancel, &|_| {}).unwrap();
    let updated = session.extract(&cancel, &|_| {}).unwrap();
    method(&updated, "Standalone", "revisedLeaf");
    assert!(native_build_count(first) > counts[0]);
    assert_eq!(native_build_count(second), counts[1]);
}

fn assert_changed_bytecode_rejects_extraction(session: &mut PreparedAnalysis, root: &Path) {
    let classes = root.join("target/classes/fixture/Standalone.class");
    let mut bytes = fs::read(&classes).unwrap();
    bytes.push(0);
    fs::write(classes, bytes).unwrap();
    let error = session
        .extract(&AtomicBool::new(false), &|_| {})
        .unwrap_err();
    assert!(error.is::<fingerprint::InputsChanged>());
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
    for output in [
        root.join("app/target/classes/fixture/App.class"),
        root.join("app/target/generated-sources/fixture/fixture/GeneratedThing.java"),
        fixture
            .path()
            .join("library/target/bytecode/main/fixture/Service.class"),
    ] {
        assert!(
            !output.exists(),
            "Overlapping builds produced {}",
            output.display()
        );
    }
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
    let graph = analyze(
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
    )
    .unwrap();
    assert!(
        edited.load(Ordering::Relaxed),
        "Compilation checkpoint was not observed"
    );
    assert!(!fingerprint::is_stale(&graph.fingerprints).unwrap());
    let original_pom = boundaries::maven_late_source_mutation(&root, &source);
    let before = fs::read(&source).unwrap();
    let modified = fs::metadata(&source).unwrap().modified().unwrap();
    let error = run(vec![build.clone()]).unwrap_err();
    assert_input_change_diagnostic(&error, &source);
    assert_equal_metadata_source_mutation(&source, &before, modified);
    fs::write(root.join("pom.xml"), original_pom).unwrap();
    fs::write(&source, before).unwrap();
    let recovered = run(vec![build.clone()]).unwrap();
    method(&recovered, "Standalone", "disconnected");
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
    let original = fs::read_to_string(&pom).unwrap();
    let skipped = original.replace(
        "</project>",
        "<build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-compiler-plugin</artifactId><configuration><skipMain>true</skipMain></configuration></plugin></plugins></build></project>",
    );
    fs::write(&pom, &skipped).unwrap();
    let error = format!("{:#}", run(vec![target(&root, "maven")]).unwrap_err());
    assert!(
        error.contains("Unsupported Java compiler observation"),
        "{error}"
    );
    assert!(error.contains("skipMain=true"), "{error}");
    let class = root.join("target/classes/fixture/Standalone.class");
    assert!(!class.exists());
    fs::write(&pom, &original).unwrap();
    let graph = run(vec![target(&root, "maven")]).unwrap();
    method(&graph, "Standalone", "leaf");
    let bytecode = fs::read(&class).unwrap();
    let source = root.join("src/main/java/fixture/Standalone.java");
    let authored = fs::read_to_string(&source).unwrap();
    assert!(authored.contains("void leaf() {}"));
    fs::write(
        &source,
        authored.replace("void leaf() {}", "void leaf() { System.out.println(1); }"),
    )
    .unwrap();
    fs::write(&pom, skipped).unwrap();
    let error = format!("{:#}", run(vec![target(&root, "maven")]).unwrap_err());
    assert!(
        error.contains("Unsupported Java compiler observation"),
        "{error}"
    );
    assert!(error.contains("skipMain=true"), "{error}");
    assert_eq!(fs::read(class).unwrap(), bytecode);
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
