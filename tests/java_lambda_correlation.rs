//! Real compiler correlation, including shared implementation export identities.
#[path = "support/java.rs"]
mod support;
use codecallnavigator::{artifact, model::GraphData};
use std::{collections::BTreeMap, fs};

fn fixture(no_debug: bool) -> tempfile::TempDir {
    let directory = support::fixture("maven");
    if let Some(command) = std::env::var_os("CODECALLNAVIGATOR_TEST_MAVEN") {
        support::install_wrapper(directory.path(), &std::path::PathBuf::from(command));
    }
    if no_debug {
        let pom = directory.path().join("standalone/pom.xml");
        let text = fs::read_to_string(&pom).unwrap();
        fs::write(pom, text.replace("</project>", "<build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-compiler-plugin</artifactId><configuration><debug>false</debug><compilerArgs><arg>-g:none</arg></compilerArgs></configuration></plugin></plugins></build></project>")).unwrap();
    }
    directory
}

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

fn source(extra: &str, callbacks: &str) -> String {
    format!(
        "package fixture; public final class Standalone {{ static void leaf() {{}} static void other() {{}} void callbacks() {{ {extra} {callbacks} }} }}"
    )
}

fn singleton_keys(graph: &GraphData) -> BTreeMap<String, String> {
    graph
        .export_identities
        .values()
        .filter_map(|key| {
            let value: serde_json::Value = serde_json::from_str(&key[key.find('{')?..]).unwrap();
            let base: serde_json::Value =
                serde_json::from_str(value["source_lambda"].as_str()?).unwrap();
            Some((base["role"].as_str().unwrap().to_owned(), key.clone()))
        })
        .collect()
}

fn assert_round_trip(graph: &GraphData, directory: &tempfile::TempDir) {
    let roots = [("repo".into(), directory.path().canonicalize().unwrap())];
    let original =
        artifact::canonical_with_identities(graph, &roots, &graph.export_identities).unwrap();
    let hydrated = original.hydrate(&roots).unwrap();
    assert_eq!(
        original.bytes().unwrap(),
        artifact::canonical_with_identities(&hydrated, &roots, &hydrated.export_identities)
            .unwrap()
            .bytes()
            .unwrap()
    );
}

fn portable_bytes(graph: &GraphData, directory: &tempfile::TempDir) -> Vec<u8> {
    let roots = [("repo".into(), directory.path().canonicalize().unwrap())];
    artifact::canonical_with_identities(graph, &roots, &graph.export_identities)
        .unwrap()
        .bytes()
        .unwrap()
}

#[test]
#[ignore = "requires pinned Java adapter, Maven and JDK25"]
fn grouped_export_is_identical_after_checkout_relocation() {
    let first = fixture(true);
    let second = fixture(true);
    let text = source(
        "",
        "Runnable first=()->leaf(), second=()->leaf(); first.run(); second.run();",
    );
    let original = run(&first, &text);
    let relocated = run(&second, &text);
    assert_eq!(
        portable_bytes(&original, &first),
        portable_bytes(&relocated, &second)
    );
}

#[test]
#[ignore = "requires pinned Java adapter, Maven and JDK25"]
fn same_line_nested_lambdas_keep_source_keys_after_insertion_reordering_and_body_edits() {
    let directory = fixture(false);
    let callbacks = "Runnable first=()->leaf(), second=()->other(); Runnable outer=()->{ Runnable inner=()->leaf(); inner.run(); }; first.run(); second.run(); outer.run();";
    let before = run(&directory, &source("", callbacks));
    assert_eq!(
        before.export_identities.len(),
        4,
        "Every same-line/nested implementation must have a source key: {:?}",
        before.diagnostics
    );
    let first = singleton_keys(&before);
    let inserted = run(
        &directory,
        &source(
            "Runnable added=()->System.nanoTime(); added.run();",
            callbacks,
        ),
    );
    let second = singleton_keys(&inserted);
    for (role, key) in &first {
        assert_eq!(
            Some(key),
            second.get(role),
            "Unrelated insertion changed {role}"
        );
    }
    let reordered = callbacks.replace(
        "Runnable first=()->leaf(), second=()->other();",
        "Runnable second=()->other(), first=()->leaf();",
    );
    let reordered = run(&directory, &source("", &reordered));
    assert_eq!(first, singleton_keys(&reordered));
    let changed = run(
        &directory,
        &source(
            "",
            &callbacks.replace("first=()->leaf()", "first=()->other()"),
        ),
    );
    let changed_keys = singleton_keys(&changed);
    assert_ne!(
        first["initializer:first"],
        changed_keys["initializer:first"]
    );
    assert_eq!(
        first["initializer:second"],
        changed_keys["initializer:second"]
    );
    assert_round_trip(&before, &directory);
}

fn shared_key(graph: &GraphData) -> &str {
    graph
        .export_identities
        .values()
        .find(|key| key.contains("source_lambda_group"))
        .expect("Merged implementation must receive a grouped identity")
}

fn assert_single_shared_node(graph: &GraphData) {
    let lambdas: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.visibility.is_none() && node.signature.contains(":lambda$"))
        .collect();
    assert_eq!(
        lambdas.len(),
        1,
        "javac must really merge the two implementations"
    );
    assert_eq!(lambdas[0].line, 0, "Fixture must lack debug lines");
    assert!(shared_key(graph).contains("initializer:first"));
    assert!(shared_key(graph).contains("initializer:second"));
    assert_eq!(graph.export_identities.len(), 1);
    assert!(
        !graph
            .flow_inventory
            .as_ref()
            .unwrap()
            .functions
            .iter()
            .any(|flow| flow.symbol == lambdas[0].id)
    );
    assert!(!graph.diagnostics.iter().any(|message| {
        message.starts_with("Java anonymous portable identity remains compiler-native")
    }));
}

#[test]
#[ignore = "requires pinned Java adapter, Maven and JDK25"]
fn no_debug_shared_implementation_has_one_complete_group_and_no_arbitrary_source_flow() {
    let directory = fixture(true);
    let callbacks = "Runnable first=()->leaf(), second=()->leaf(); first.run(); second.run();";
    let before = run(&directory, &source("", callbacks));
    assert_single_shared_node(&before);
    let repeated = run(&directory, &source("", callbacks));
    assert_eq!(before.nodes, repeated.nodes);
    assert_eq!(
        serde_json::to_value(&before.edges).unwrap(),
        serde_json::to_value(&repeated.edges).unwrap()
    );
    assert_eq!(before.export_identities, repeated.export_identities);
    let inserted = run(
        &directory,
        &source("Runnable added=()->other(); added.run();", callbacks),
    );
    assert_eq!(shared_key(&before), shared_key(&inserted));
    let reordered = run(
        &directory,
        &source(
            "",
            &callbacks.replace(
                "first=()->leaf(), second=()->leaf()",
                "second=()->leaf(), first=()->leaf()",
            ),
        ),
    );
    assert_eq!(shared_key(&before), shared_key(&reordered));
    let expanded = run(
        &directory,
        &source("Runnable third=()->leaf(); third.run();", callbacks),
    );
    assert_ne!(shared_key(&before), shared_key(&expanded));
    let edited = run(
        &directory,
        &source(
            "",
            &callbacks.replace("second=()->leaf()", "second=()->other()"),
        ),
    );
    assert_eq!(edited.export_identities.len(), 2);
    assert!(
        edited
            .export_identities
            .values()
            .all(|key| !key.contains("source_lambda_group"))
    );
    assert_round_trip(&before, &directory);
}

#[test]
#[ignore = "requires pinned Java adapter, Maven and JDK25"]
fn no_debug_distinct_bodies_bind_exact_flow_spans_and_targets() {
    let directory = fixture(true);
    let text = source(
        "",
        "Runnable first=()->leaf(), second=()->other(); first.run(); second.run();",
    );
    let graph = run(&directory, &text);
    assert_eq!(graph.export_identities.len(), 2, "{:?}", graph.diagnostics);
    let functions = &graph.flow_inventory.as_ref().unwrap().functions;
    for (role, body, target) in [
        ("initializer:first", "()->leaf()", ":leaf()"),
        ("initializer:second", "()->other()", ":other()"),
    ] {
        let key = &singleton_keys(&graph)[role];
        let native = graph
            .export_identities
            .iter()
            .find(|(_, value)| *value == key)
            .unwrap()
            .0;
        let function = functions
            .iter()
            .find(|function| &function.symbol == native)
            .unwrap();
        assert_eq!(&text[function.span.start..function.span.end], body);
        let mut copy = function.clone();
        let mut targets = Vec::new();
        codecallnavigator::flow::visit_calls_mut(&mut copy, &mut |call| {
            targets.extend(call.targets.iter().map(|target| target.symbol.clone()))
        });
        assert_eq!(targets.len(), 1);
        assert!(targets[0].contains(target));
        assert!(graph.nodes.iter().any(|node| node.id == targets[0]));
    }
    assert_round_trip(&graph, &directory);
}
