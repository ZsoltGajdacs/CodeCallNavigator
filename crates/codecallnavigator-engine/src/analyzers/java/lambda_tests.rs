//! Acceptance coverage through javac, the bytecode engine, normalization and SQLite.
use super::*;
use crate::{
    analyzers::AnalyzerRuntime,
    graph::Filter,
    store::{SqliteStore, SymbolSearch},
};
use std::sync::atomic::AtomicBool;

fn analyze(debug: &str) -> (tempfile::TempDir, GraphData) {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AnalyzerRuntime::discover().unwrap();
    let input = tests::compile_source_fixture(
        &runtime,
        directory.path(),
        include_str!("../../../../../fixtures/java-lambda-names/Flow.java"),
        debug,
    );
    let output = tests::run_callback_engine(&runtime, directory.path(), &input);
    let build = BuildTarget {
        path: directory.path().into(),
        language: "java".into(),
        kind: "gradle".into(),
        label: "lambda names".into(),
    };
    let graph = normalize_bytecode(&build, &output, &[input], None).unwrap();
    (directory, graph)
}

#[test]
#[ignore = "requires the pinned bytecode distribution and installed JDK 25"]
fn compiled_lambdas_have_readable_searchable_names() {
    let (directory, graph) = analyze("-g");
    for expected in [
        "COST_TYPE_VALIDATION",
        "instanceAction",
        "localAction",
        "lambda$manual$99",
        "validate · lambda at L17",
    ] {
        assert!(
            graph.nodes.iter().any(|node| node.label == expected),
            "Missing {expected}: {:?}",
            graph.nodes.iter().map(|n| &n.label).collect::<Vec<_>>()
        );
    }
    assert!(
        !graph
            .nodes
            .iter()
            .any(|n| matches!(n.label.as_str(), "first" | "second" | "nested"))
    );
    let lambdas: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.signature.contains(":lambda$") && !n.signature.contains(":lambda$manual$"))
        .collect();
    assert_eq!(lambdas.len(), 8);
    assert!(
        lambdas
            .iter()
            .all(|n| !n.label.starts_with("lambda$") && !n.is_generated && n.line > 0)
    );
    assert!(lambdas.iter().all(|n| {
        graph.edges.iter().any(|edge| {
            graph.nodes[edge.from as usize].id == n.id || graph.nodes[edge.to as usize].id == n.id
        })
    }));
    let store = SqliteStore::open(directory.path().join("store")).unwrap();
    store.publish(&graph, &AtomicBool::new(false)).unwrap();
    let restored = store.load_active().unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(&graph.nodes).unwrap(),
        serde_json::to_value(&restored.nodes).unwrap()
    );
    for prefix in ["COST_TYPE_VALIDATION", "lambda"] {
        let hits = store
            .search(&SymbolSearch {
                prefix,
                filter: &Filter::default(),
                limit: 100,
                offset: 0,
            })
            .unwrap();
        assert!(
            hits.iter()
                .any(|&id| restored.nodes[id as usize].label == "COST_TYPE_VALIDATION")
        );
    }
}

#[test]
#[ignore = "requires the pinned bytecode distribution and installed JDK 25"]
fn no_debug_lambdas_use_contextual_fallbacks() {
    let (_directory, graph) = analyze("-g:none");
    let lambdas: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.signature.contains(":lambda$") && !n.signature.contains(":lambda$manual$"))
        .collect();
    assert_eq!(lambdas.len(), 8);
    assert!(
        lambdas
            .iter()
            .all(|n| n.label.contains(" · lambda #") && n.line == 0)
    );
    assert!(
        lambdas
            .iter()
            .any(|n| n.label.starts_with("Static initializer · lambda #"))
    );
}

#[test]
#[ignore = "requires the pinned bytecode distribution and installed JDK 25"]
fn direct_assignments_and_wrapped_lambdas_keep_their_source_names() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AnalyzerRuntime::discover().unwrap();
    let input = tests::compile_source_fixture(
        &runtime,
        directory.path(),
        r#"
package fixture;
public class Flow {
    Runnable assigned;
    Flow() {
        assigned = (Runnable) (() -> {
            leaf();
        });
    }
    void validate() {
        Runnable temporary;
        temporary = () -> {
            leaf();
        };
    }
    class Inner {
        Runnable inside = () -> {
            leaf();
        };
    }
    static void leaf() {}
}
"#,
        "-g",
    );
    let output = tests::run_callback_engine(&runtime, directory.path(), &input);
    let build = BuildTarget {
        path: directory.path().into(),
        language: "java".into(),
        kind: "maven".into(),
        label: "assignments".into(),
    };
    let graph = normalize_bytecode(&build, &output, &[input], None).unwrap();
    for expected in ["assigned", "temporary", "inside"] {
        let node = graph
            .nodes
            .iter()
            .find(|node| node.label == expected)
            .unwrap_or_else(|| panic!("Missing {expected}"));
        assert!(node.signature.contains(":lambda$"));
        assert!(node.id.ends_with(&node.signature.replace(" -> ", "|")));
        assert_eq!(node.language, "java");
    }
    assert!(
        graph
            .nodes
            .iter()
            .any(|node| node.container == "fixture.Flow$Inner" && node.label == "inside")
    );
}

#[test]
#[ignore = "requires the pinned bytecode distribution and installed JDK 25"]
fn unresolved_stream_method_references_keep_lambdas_in_their_declaring_files() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AnalyzerRuntime::discover().unwrap();
    let source = "package fixture; public class Flow { enum Item { VALUE; String getValue() { return name(); } } Object values() { return java.util.stream.Stream.of(Item.VALUE).filter(item -> item.getValue().equals(\"VALUE\")).map(Item::getValue).toList(); } }";
    let second = directory.path().join("src/fixture/Other.java");
    fs::create_dir_all(second.parent().unwrap()).unwrap();
    fs::write(&second, source.replace("class Flow", "class Other")).unwrap();
    let input = tests::compile_source_fixture(&runtime, directory.path(), source, "-g");
    for path in [second, directory.path().join("src/fixture/Flow.java")] {
        let text = fs::read_to_string(&path).unwrap();
        fs::write(
            path,
            text.replace("String getValue() { return name(); }", ""),
        )
        .unwrap();
    }
    let output = tests::run_callback_engine(&runtime, directory.path(), &input);
    let build = BuildTarget {
        path: directory.path().into(),
        language: "java".into(),
        kind: "gradle".into(),
        label: "unresolved stream references".into(),
    };
    let graph = normalize_bytecode(&build, &output, &[input], None).unwrap();
    graph.validate().unwrap();
    for owner in ["Flow", "Other"] {
        assert!(graph.nodes.iter().any(|node| {
            node.signature.contains(&format!("fixture.{owner}:lambda$"))
                && node.file.ends_with(&format!("/{owner}.java"))
                && node.label.starts_with("values · lambda")
        }));
        assert!(graph.diagnostics.iter().any(|message| {
            message.contains(&format!("fixture.{owner}"))
                && message.contains("Source attribution prevents compiler replay")
        }));
    }
}
