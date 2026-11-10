//! Real build-tool coverage for duplicate definitions and caller compile contexts.
#[path = "support/java.rs"]
mod support;
use codegraph::{
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

fn symbol<'a>(graph: &'a GraphData, owner: (&str, &str), method: &str) -> &'a Node {
    let signature = format!("duplicate.{method}()");
    graph
        .nodes
        .iter()
        .find(|node| {
            node.project.ends_with(owner.0)
                && node.signature.starts_with(&signature)
                && node.id.contains(&format!("\"{}\"]|", owner.1))
        })
        .unwrap_or_else(|| panic!("Missing {owner:?}/{method}"))
}

fn assert_call(graph: &GraphData, caller: &Node, target: &Node) {
    let from = graph
        .nodes
        .iter()
        .position(|node| node.id == caller.id)
        .unwrap() as u32;
    let to = graph
        .nodes
        .iter()
        .position(|node| node.id == target.id)
        .unwrap() as u32;
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.from == from && edge.to == to),
        "Missing {} -> {}",
        caller.id,
        target.id
    );
    let count = graph
        .edges
        .iter()
        .filter(|edge| {
            edge.from == from && graph.nodes[edge.to as usize].container == target.container
        })
        .count();
    assert_eq!(count, 1, "Duplicate or incorrectly resolved call");
}

fn assert_module_calls(graph: &GraphData, reversed: bool) {
    let left = symbol(graph, ("left", "main"), "Choice:pick");
    let right = symbol(graph, ("right", "main"), "Choice:pick");
    assert_ne!(left.id, right.id);
    assert_ne!(left.file, right.file);
    assert_call(
        graph,
        symbol(graph, ("left", "main"), "LeftCaller:run"),
        left,
    );
    assert_call(
        graph,
        symbol(graph, ("right", "main"), "RightCaller:run"),
        right,
    );
    assert_call(
        graph,
        left,
        symbol(graph, ("left", "main"), "LeftLeaf:leaf"),
    );
    assert_call(
        graph,
        right,
        symbol(graph, ("right", "main"), "RightLeaf:leaf"),
    );
    let selected = if reversed { right } else { left };
    assert_call(
        graph,
        symbol(graph, ("consumer", "main"), "Consumer:run"),
        selected,
    );
}

fn assert_source_set(graph: &GraphData, source_set: &str, caller: &str, leaf: &str) {
    let owner = ("consumer", source_set);
    let choice = symbol(graph, owner, "Choice:pick");
    assert!(choice.is_test);
    assert_call(graph, symbol(graph, owner, caller), choice);
    assert_call(graph, choice, symbol(graph, owner, leaf));
}

fn assert_copies_and_diagnostics(graph: &GraphData) {
    let copies = graph
        .nodes
        .iter()
        .filter(|node| node.signature.starts_with("duplicate.Common:shared()"))
        .collect::<Vec<_>>();
    assert_eq!(copies.len(), 2);
    assert_ne!(copies[0].id, copies[1].id);
    for expected in [
        "identical bytes",
        "differing bytes",
        "incompatible consumer contexts",
    ] {
        assert!(
            graph.diagnostics.iter().any(|line| line.contains(expected)),
            "Missing {expected} diagnostic"
        );
    }
}

fn assert_duplicates(graph: &GraphData, gradle: bool, reversed: bool) {
    graph.validate().unwrap();
    assert_module_calls(graph, reversed);
    assert_source_set(graph, "test", "TestCaller:run", "TestLeaf:leaf");
    assert_copies_and_diagnostics(graph);
    if gradle {
        assert_source_set(
            graph,
            "integrationTest",
            "IntegrationCaller:run",
            "IntegrationLeaf:leaf",
        );
    }
}

fn duplicate_fixture(gradle: bool) -> tempfile::TempDir {
    let fixture = fixture("java-duplicates");
    if gradle {
        install_wrapper(
            fixture.path(),
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/gradle/gradlew"),
        );
    } else if let Some(command) = std::env::var_os("CODEGRAPH_TEST_MAVEN") {
        install_wrapper(fixture.path(), &PathBuf::from(command));
    }
    fixture
}

fn reverse_dependencies(root: &Path, gradle: bool) {
    let config = root.join(if gradle {
        "build.gradle"
    } else {
        "consumer/pom.xml"
    });
    let text = fs::read_to_string(&config).unwrap();
    let reversed = if gradle {
        text.replace(
            "implementation project(':left'); implementation project(':right')",
            "implementation project(':right'); implementation project(':left')",
        )
    } else {
        text.replace(
            "<artifactId>left</artifactId>",
            "<artifactId>swap</artifactId>",
        )
        .replace(
            "<artifactId>right</artifactId>",
            "<artifactId>left</artifactId>",
        )
        .replace(
            "<artifactId>swap</artifactId>",
            "<artifactId>right</artifactId>",
        )
    };
    fs::write(config, reversed).unwrap();
}

fn duplicates_acceptance(gradle: bool) {
    let fixture = duplicate_fixture(gradle);
    let build = target(fixture.path(), if gradle { "gradle" } else { "maven" });
    let first = run(vec![build.clone()]).unwrap();
    assert_duplicates(&first, gradle, false);
    assert!(!fingerprint::is_stale(&first.fingerprints).unwrap());
    let cache = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(cache.path()).unwrap();
    store.publish(&first, &AtomicBool::new(false)).unwrap();
    assert_duplicates(&store.load_active().unwrap().unwrap(), gradle, false);
    let second = run(vec![build.clone()]).unwrap();
    assert_eq!(
        first.nodes.iter().map(|node| &node.id).collect::<Vec<_>>(),
        second.nodes.iter().map(|node| &node.id).collect::<Vec<_>>()
    );
    reverse_dependencies(fixture.path(), gradle);
    assert!(fingerprint::is_stale(&first.fingerprints).unwrap());
    let third = run(vec![build]).unwrap();
    assert_duplicates(&third, gradle, true);
    assert_ne!(first.engine_versions, third.engine_versions);
    assert_external_shadowing(fixture.path(), gradle);
}

fn external_archive(root: &Path) -> PathBuf {
    let java = codegraph_engine::analyzers::AnalyzerRuntime::discover()
        .unwrap()
        .java_home
        .unwrap();
    let source = root.join("external-source/duplicate/Choice.java");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::write(&source, "package duplicate; public class Choice { public static void pick() { ExternalLeaf.leaf(); } } class ExternalLeaf { static void leaf() {} }").unwrap();
    let classes = root.join("external-classes");
    for (tool, arguments) in [
        (
            "javac",
            vec![
                "-g".into(),
                "-d".into(),
                classes.clone().into_os_string(),
                source.into_os_string(),
            ],
        ),
        (
            "jar",
            vec![
                "cf".into(),
                root.join("external.jar").into_os_string(),
                "-C".into(),
                classes.into_os_string(),
                ".".into(),
            ],
        ),
    ] {
        let output = std::process::Command::new(java.join("bin").join(tool))
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    root.join("external.jar")
}

fn assert_external_shadowing(root: &Path, gradle: bool) {
    let artifact = external_archive(root);
    let config = root.join(if gradle {
        "build.gradle"
    } else {
        "consumer/pom.xml"
    });
    let contents = fs::read_to_string(&config).unwrap();
    let contents = if gradle {
        contents.replace(
            "dependencies { implementation",
            "dependencies { implementation files(rootProject.file('external.jar')); implementation",
        )
    } else {
        contents.replace("<dependencies>", &format!("<dependencies><dependency><groupId>external</groupId><artifactId>shadow</artifactId><version>1</version><scope>system</scope><systemPath>{}</systemPath></dependency>", artifact.display()))
    };
    fs::write(config, contents).unwrap();
    let graph = run(vec![target(root, if gradle { "gradle" } else { "maven" })]).unwrap();
    let caller = symbol(&graph, ("consumer", "main"), "Consumer:run");
    let index = graph
        .nodes
        .iter()
        .position(|node| node.id == caller.id)
        .unwrap() as u32;
    assert!(
        !graph.edges.iter().any(|edge| edge.from == index
            && graph.nodes[edge.to as usize].container == "duplicate.Choice")
    );
    assert!(
        !graph
            .nodes
            .iter()
            .any(|node| node.container == "duplicate.ExternalLeaf")
    );
    assert!(
        graph
            .diagnostics
            .iter()
            .any(|line| line.contains("duplicate.Choice selects") && line.contains("external.jar"))
    );
}

#[test]
#[ignore = "requires pinned analyzer, Maven and JDK 25"]
fn maven_duplicate_classes_preserve_compile_contexts_and_classpath_precedence() {
    duplicates_acceptance(false);
}

#[test]
#[ignore = "requires pinned analyzer, Gradle and JDK 25"]
fn gradle_duplicate_classes_preserve_compile_contexts_and_classpath_precedence() {
    duplicates_acceptance(true);
}

#[test]
#[ignore = "requires CODEGRAPH_RECURRING_BILLING, private Maven dependencies and JDK 25"]
fn recurring_billing_retains_duplicate_generated_declarations() {
    let root = PathBuf::from(
        std::env::var_os("CODEGRAPH_RECURRING_BILLING").expect("Set CODEGRAPH_RECURRING_BILLING"),
    );
    let graph = run(vec![target(&root, "maven")]).unwrap();
    graph.validate().unwrap();
    let defaults = graph
        .nodes
        .iter()
        .find(|node| {
            node.container
                == "se.fortnox.recurringbilling.external.f3.defaults.F3DefaultsServiceImpl"
                && node.signature.contains(":buildCommonDefaults(")
                && !node.is_test
        })
        .expect("Authored method referencing the Lombok-generated builder must survive");
    assert!(defaults.signature.ends_with(
        " -> se.fortnox.recurringbilling.external.f3.defaults.F3Defaults$F3DefaultsBuilder"
    ));
    for module in ["recurring-billing-impl", "recurring-billing-test-utils"] {
        assert_recurring_generated(&graph, module);
    }
    if let Some(path) = std::env::var_os("CODEGRAPH_GRAPH_EVIDENCE") {
        fs::write(path, serde_json::to_vec(&graph).unwrap()).unwrap();
    }
    eprintln!(
        "Recurring-billing: {} declarations, {} relationships",
        graph.nodes.len(),
        graph.edges.len()
    );
}

fn assert_recurring_generated(graph: &GraphData, module: &str) {
    let nodes = graph
        .nodes
        .iter()
        .filter(|node| {
            node.project.ends_with(module)
                && (node.namespace == "se.fortnox.messages.v1"
                    || node.namespace == "se.fortnox.notices.v1")
        })
        .collect::<Vec<_>>();
    let classes = nodes
        .iter()
        .map(|node| &node.container)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        classes.len(),
        10,
        "Missing generated declarations for {module}"
    );
    assert!(
        nodes
            .iter()
            .all(|node| node.is_generated && !node.file.is_empty())
    );
}
