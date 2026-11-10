//! Real javac synthetic identities stay native; portable IDs use proven source attribution.
#[path = "support/java.rs"]
mod support;
use codegraph::{artifact, model::GraphData};
use std::fs;

fn run(directory: &tempfile::TempDir, source: &str) -> GraphData {
    fs::write(
        directory
            .path()
            .join("standalone/src/main/java/fixture/Standalone.java"),
        source,
    )
    .unwrap();
    support::run(vec![support::target(
        &directory.path().join("standalone"),
        "maven",
    )])
    .unwrap()
}
fn source(extra: &str) -> String {
    format!(
        r#"package fixture;
public final class Standalone {{
 static void leaf(String value) {{}}
 void run() {{
 {extra}
 Runnable old = () -> {{
    leaf("old");
 }};
 old.run();
 }}
}}
"#
    )
}
fn portable(graph: &GraphData, directory: &tempfile::TempDir, label: &str) -> String {
    let value = artifact::canonical_with_identities(
        graph,
        &[("repo".into(), directory.path().canonicalize().unwrap())],
        &graph.export_identities,
    )
    .unwrap();
    value.graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["label"] == label)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
#[test]
#[ignore = "requires pinned Java adapter, Maven and JDK25"]
fn unrelated_lambda_insertion_keeps_proven_source_identity_and_native_binding_unchanged() {
    let directory = support::fixture("maven");
    if let Some(command) = std::env::var_os("CODEGRAPH_TEST_MAVEN") {
        support::install_wrapper(directory.path(), &std::path::PathBuf::from(command));
    }
    let before = run(&directory, &source(""));
    let after = run(
        &directory,
        &source("Runnable added = () -> leaf(\"added\");"),
    );
    let native = |graph: &GraphData| {
        graph
            .nodes
            .iter()
            .find(|node| node.label == "old")
            .unwrap()
            .id
            .clone()
    };
    assert_ne!(
        native(&before),
        native(&after),
        "javac really renumbered the synthetic method"
    );
    assert_eq!(
        portable(&before, &directory, "old"),
        portable(&after, &directory, "old")
    );
    let formatted = source("Runnable added = () -> leaf(\"added\");").replace(
        "Runnable old = () -> {",
        "/* comment */ Runnable old=()->{ /* body */",
    );
    let formatted = run(&directory, &formatted);
    assert_eq!(
        portable(&after, &directory, "old"),
        portable(&formatted, &directory, "old")
    );
    let changed = run(
        &directory,
        &source("Runnable added = () -> leaf(\"added\");")
            .replace("leaf(\"old\")", "leaf(\"changed\")"),
    );
    assert_ne!(
        portable(&after, &directory, "old"),
        portable(&changed, &directory, "old")
    );
}

#[test]
#[ignore = "requires pinned Java adapter, Maven and JDK25"]
fn same_line_siblings_get_identities_while_local_type_owners_keep_native_fallbacks() {
    let directory = support::fixture("maven");
    if let Some(command) = std::env::var_os("CODEGRAPH_TEST_MAVEN") {
        support::install_wrapper(directory.path(), &std::path::PathBuf::from(command));
    }
    let graph = run(
        &directory,
        r#"package fixture;
public final class Standalone {
 static void leaf(String value) {}
 void callbacks() {
   Runnable first=()->leaf("a"), second=()->leaf("b");
   first.run(); second.run();
   class Local { void get() {
     Runnable localOwner=()->leaf("local");
     localOwner.run();
   }}
   new Local().get();
 }
 public void lambda$manual$99() {}
}
"#,
    );
    let synthetic: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.visibility.is_none() && node.signature.contains(":lambda$"))
        .collect();
    assert!(synthetic.len() >= 3);
    assert_eq!(
        synthetic
            .iter()
            .filter(|node| graph.export_identities.contains_key(&node.id))
            .count(),
        2,
        "The named owner has two verified same-line source lambdas"
    );
    assert!(
        synthetic
            .iter()
            .filter(|node| node.signature.contains("$Local:"))
            .all(|node| !graph.export_identities.contains_key(&node.id))
    );
    assert_eq!(
        graph
            .diagnostics
            .iter()
            .filter(|message| message
                .starts_with("Java anonymous portable identity remains compiler-native"))
            .count(),
        1,
        "Only the local type owner retains a native fallback"
    );
    assert!(
        !graph
            .diagnostics
            .iter()
            .any(|message| message.contains("portable identity remains")
                && message.contains("lambda$manual$99"))
    );
}

#[test]
#[ignore = "requires pinned Java adapter, Maven and JDK25"]
fn identical_source_siblings_have_distinct_deterministic_occurrences() {
    let directory = support::fixture("maven");
    if let Some(command) = std::env::var_os("CODEGRAPH_TEST_MAVEN") {
        support::install_wrapper(directory.path(), &std::path::PathBuf::from(command));
    }
    let graph = run(
        &directory,
        r#"package fixture;
public final class Standalone {
 static void leaf(String value) {}
 static void accept(Runnable callback) {callback.run();}
 void callbacks() {
   accept(() -> leaf("same"));
   accept(() -> leaf("same"));
 }
 void callbacks(int value) {
   accept(() -> leaf("same"));
 }
}
"#,
    );
    let keys: std::collections::HashSet<_> = graph.export_identities.values().collect();
    assert_eq!(keys.len(), 3);
    assert!(keys.iter().any(|key| key.contains("\"occurrence\":0")));
    assert!(keys.iter().any(|key| key.contains("\"occurrence\":1")));
}
