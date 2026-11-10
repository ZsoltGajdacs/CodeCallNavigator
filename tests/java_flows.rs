//! Real javac/bytecode binding acceptance for detached function source flow.
#[path = "support/java.rs"]
mod support;
use codegraph::{flow, model::GraphData, store::SqliteStore};
use std::{collections::HashSet, fs, sync::atomic::AtomicBool};

fn fixture(source: &str, reactor: bool) -> tempfile::TempDir {
    let directory = support::fixture("maven");
    if let Some(command) = std::env::var_os("CODEGRAPH_TEST_MAVEN") {
        support::install_wrapper(directory.path(), &std::path::PathBuf::from(command));
    }
    fs::write(
        directory
            .path()
            .join("standalone/src/main/java/fixture/Standalone.java"),
        source,
    )
    .unwrap();
    if reactor {
        let pom = directory.path().join("standalone/pom.xml");
        let original = fs::read_to_string(&pom).unwrap();
        fs::write(pom, original.replace("</project>", "<dependencies><dependency><groupId>io.projectreactor</groupId><artifactId>reactor-core</artifactId><version>3.8.7</version></dependency></dependencies></project>")).unwrap();
    }
    directory
}
fn run(directory: &tempfile::TempDir) -> GraphData {
    support::run(vec![support::target(
        &directory.path().join("standalone"),
        "maven",
    )])
    .unwrap()
}
fn function<'a>(graph: &'a GraphData, name: &str) -> &'a flow::FunctionFlowIR {
    let node = graph
        .nodes
        .iter()
        .find(|node| node.signature.contains(name))
        .unwrap();
    graph
        .flow_inventory
        .as_ref()
        .unwrap()
        .functions
        .iter()
        .find(|function| function.symbol == node.id)
        .unwrap()
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven and JDK 25"]
fn java_flow_binds_overloads_preserves_unicode_and_skips_ambiguous_lambdas() {
    let source = "package fixture;\npublic final class Standalone {\n static int leaf(int x) { return x; }\n static int leaf(String x) { return x.length(); }\n int run(int x) { String text = \"é😀\"; outer: for(int i=0;i<2;i++) { try { if(x>i) return leaf(x); if(x<0) continue outer; break; } finally { leaf(0); } } return switch(x) { case 0 -> leaf(1); default -> { yield leaf(\"s\"); } }; }\n void callbacks() {\n  Runnable unique = () -> { leaf(10); };\n  Runnable first = () -> leaf(1), second = () -> leaf(2);\n  unique.run();\n }\n}\n";
    let directory = fixture(source, false);
    let graph = run(&directory);
    let run_function = function(&graph, ":run(int)");
    let chart = flow::build(run_function).unwrap();
    let graph_ids: HashSet<_> = graph.nodes.iter().map(|node| node.id.as_str()).collect();
    let mut function_copy = run_function.clone();
    let mut targets = Vec::new();
    flow::visit_calls_mut(&mut function_copy, &mut |call| {
        for target in &call.targets {
            targets.push(target.symbol.clone());
        }
        let span = call.span;
        assert!(source.is_char_boundary(span.start) && source.is_char_boundary(span.end));
    });
    assert!(
        targets
            .iter()
            .all(|target| graph_ids.contains(target.as_str()))
    );
    assert!(targets.iter().any(|target| target.contains(":leaf(int)")));
    assert!(
        targets
            .iter()
            .any(|target| target.contains(":leaf(java.lang.String)"))
    );
    assert!(chart.steps.iter().any(|step| step.kind == "decision"));
    let inventory = graph.flow_inventory.as_ref().unwrap();
    let lambdas: Vec<_> = inventory
        .functions
        .iter()
        .filter(|function| function.symbol.contains(":lambda$"))
        .collect();
    assert_eq!(
        lambdas.len(),
        1,
        "Same-line callback declarations must remain unbound"
    );
    let callbacks = flow::build(function(&graph, ":callbacks()")).unwrap();
    assert_eq!(callbacks.regions.len(), 3);
    let cache = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(cache.path()).unwrap();
    store.publish(&graph, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        serde_json::to_value(store.load_active().unwrap().unwrap().flow_inventory).unwrap(),
        serde_json::to_value(&graph.flow_inventory).unwrap()
    );
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven, JDK 25 and cached Reactor 3.8.7"]
fn reactor_stages_require_compiler_types_and_keep_callbacks_deferred() {
    let source = include_str!("../fixtures/flow/java-reactor/Standalone.java");
    let directory = fixture(source, true);
    let graph = run(&directory);
    let pipeline = flow::build(function(&graph, ":pipeline()")).unwrap();
    for operation in [
        "defer",
        "map",
        "filter",
        "switchIfEmpty",
        "onErrorResume",
        "then",
    ] {
        assert!(
            pipeline
                .regions
                .iter()
                .any(|region| region.deferred
                    && region.label.contains(&format!("Reactor {operation}"))),
            "Missing {operation}"
        );
    }
    let stream = flow::build(function(&graph, ":stream()")).unwrap();
    assert!(
        stream
            .regions
            .iter()
            .any(|region| region.label.contains("collectList"))
    );
    let concurrent = flow::build(function(&graph, ":concurrent()")).unwrap();
    assert!(
        concurrent
            .regions
            .iter()
            .any(|region| region.label.contains("concurrently"))
    );
    let ordinary = flow::build(function(&graph, ":ordinary()")).unwrap();
    assert!(
        ordinary
            .regions
            .iter()
            .all(|region| !region.label.starts_with("Reactor"))
    );
    assert_reactor_execution(&directory, &graph, &pipeline);
    let deferred: HashSet<_> = pipeline
        .regions
        .iter()
        .flat_map(|region| region.steps.iter().copied())
        .collect();
    assert!(
        pipeline
            .links
            .iter()
            .all(|link| deferred.contains(&link.from) == deferred.contains(&link.to)),
        "Callbacks must not join assembly execution"
    );
}

fn assert_reactor_execution(
    directory: &tempfile::TempDir,
    graph: &GraphData,
    chart: &flow::FlowChart,
) {
    let mut classpath = vec![directory.path().join("standalone/target/classes")];
    classpath.extend(
        graph
            .fingerprints
            .iter()
            .filter(|entry| {
                entry.path.ends_with("reactor-core-3.8.7.jar")
                    || entry.path.ends_with("reactive-streams-1.0.4.jar")
            })
            .map(|entry| std::path::PathBuf::from(&entry.path)),
    );
    let runtime = codegraph_engine::analyzers::AnalyzerRuntime::discover().unwrap();
    let output = std::process::Command::new(runtime.java_home.unwrap().join("bin/java"))
        .arg("-cp")
        .arg(std::env::join_paths(classpath).unwrap())
        .arg("fixture.Standalone")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "2,4,|2,4,1,1,|4"
    );
    let deferred: HashSet<_> = chart
        .regions
        .iter()
        .flat_map(|region| region.steps.iter().copied())
        .collect();
    for (label, expected) in [
        ("leaf(1)", true),
        ("leaf(value)", true),
        ("leaf(3)", true),
        ("leaf(2)", false),
        ("leaf(4)", false),
    ] {
        let step = chart
            .steps
            .iter()
            .find(|step| step.calls.iter().any(|call| call.label == label))
            .unwrap();
        assert_eq!(
            deferred.contains(&step.id),
            expected,
            "Assembly/subscription placement of {label}"
        );
    }
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven and JDK 25"]
fn deeply_nested_source_exports_partial_ir_without_rejecting_the_graph() {
    let source = format!(
        "package fixture; public final class Standalone {{ int deep(int x) {{ {} x++; {} return x; }} }}",
        "if(x>0){".repeat(80),
        "}".repeat(80)
    );
    let directory = fixture(&source, false);
    let graph = run(&directory);
    let function = function(&graph, ":deep(int)");
    let chart = flow::build(function).unwrap();
    assert!(chart.partial);
    assert!(
        function
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("24 levels"))
    );
    assert!(chart.steps.iter().any(|step| step.kind == "unsupported"));
}

#[test]
#[ignore = "requires pinned Java analyzer, Maven and JDK 25"]
fn java_labels_preserve_block_siblings_and_nested_labels() {
    let source = "package fixture; public final class Standalone { static void leaf() {} void labelled(boolean flag) { outer: { while(flag) { break outer; } leaf(); } alpha: beta: while(flag) { break alpha; } leaf(); } }";
    let directory = fixture(source, false);
    let graph = run(&directory);
    let chart = flow::build(function(&graph, ":labelled(boolean)")).unwrap();
    let leaf_calls = chart
        .steps
        .iter()
        .flat_map(|step| &step.calls)
        .filter(|call| call.label == "leaf()")
        .count();
    assert_eq!(
        leaf_calls, 2,
        "Labelled block must retain statements after an inner loop"
    );
    assert!(!chart.partial);
}

fn marker(step: &flow::FlowStep) -> Vec<String> {
    if step.label.starts_with("Close resource ") {
        return vec!["close".into()];
    }
    step.calls
        .iter()
        .filter_map(|call| {
            call.label
                .strip_prefix("mark(\"")
                .and_then(|value| value.strip_suffix("\")"))
                .map(str::to_owned)
        })
        .collect()
}
fn contains_trace(chart: &flow::FlowChart, trace: &[&str]) -> bool {
    let mut pending = std::collections::VecDeque::from([(chart.entry, 0)]);
    let mut seen = HashSet::new();
    while let Some((node, mut position)) = pending.pop_front() {
        if !seen.insert((node, position)) {
            continue;
        }
        let events = marker(&chart.steps[node]);
        if !events.iter().all(|event| {
            let matches = trace.get(position) == Some(&event.as_str());
            position += 1;
            matches
        }) {
            continue;
        }
        if chart.steps[node].kind == "exit" && position == trace.len() {
            return true;
        }
        for link in chart.links.iter().filter(|link| link.from == node) {
            pending.push_back((link.to, position));
        }
    }
    false
}
#[test]
#[ignore = "requires pinned Java analyzer, Maven and JDK 25"]
fn actual_java_completion_and_resource_traces_exist_in_the_source_chart() {
    let directory = fixture(
        include_str!("../fixtures/flow/java-control/Standalone.java"),
        false,
    );
    let graph = run(&directory);
    let runtime = codegraph_engine::analyzers::AnalyzerRuntime::discover().unwrap();
    let java = runtime.java_home.unwrap().join("bin/java");
    for mode in 0..8 {
        let output = std::process::Command::new(&java)
            .args(["-cp"])
            .arg(directory.path().join("standalone/target/classes"))
            .args(["fixture.Standalone", &mode.to_string()])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = String::from_utf8(output.stdout).unwrap();
        let events: Vec<_> = output
            .trim()
            .split(',')
            .filter(|value| !value.is_empty() && *value != "acquire")
            .collect();
        let name = if mode < 5 {
            ":exercise(int)"
        } else if mode < 7 {
            ":resources(boolean)"
        } else {
            ":overriding()"
        };
        let chart = flow::build(function(&graph, name)).unwrap();
        assert!(
            contains_trace(&chart, &events),
            "Mode {mode} runtime trace {events:?} must follow a chart path"
        );
    }
    let chart = flow::build(function(&graph, ":exercise(int)")).unwrap();
    assert!(
        !contains_trace(&chart, &["body", "after"]),
        "finally cannot be bypassed"
    );
}
