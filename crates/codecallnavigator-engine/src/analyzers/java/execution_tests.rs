//! Real compiler/normalizer proof without Maven or a second production entry point.
use super::*;
#[path = "../../../tests/support/compiler_expansion.rs"]
mod compiler_expansion;
use crate::analyzers::AnalyzerRuntime;
use crate::flow::{self, CallSite, ExecutionTiming, FunctionFlowIR, InvocationDispatch};
use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicBool},
};

#[test]
#[ignore = "requires the matching Java adapter and installed JDK 25"]
fn real_javac_execution_facts_keep_owned_targets_and_external_site_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = AnalyzerRuntime::discover().unwrap();
    let input = tests::compile_source_fixture(
        &runtime,
        directory.path(),
        include_str!("../../../../../fixtures/flow/java-execution/Flow.java"),
        "-g",
    );
    let output = tests::run_callback_engine(&runtime, directory.path(), &input);
    let build = BuildTarget {
        path: directory.path().into(),
        language: "java".into(),
        kind: "javac".into(),
        label: "facts".into(),
    };
    let graph = normalize_bytecode(&build, &output, std::slice::from_ref(&input), None).unwrap();
    let run = function(&graph, ":run(fixture.Contract)");
    assert_eq!(run.execution, ExecutionTiming::Synchronous);
    let calls = sites(run);
    let virtual_site = calls
        .iter()
        .find(|site| site.label == "contract.hit()")
        .unwrap();
    assert_eq!(
        virtual_site.invocation.dispatch,
        InvocationDispatch::Virtual
    );
    assert!(virtual_site.targets.len() <= 1);
    let external = calls
        .iter()
        .find(|site| site.label == "\"x\".length()")
        .unwrap();
    assert!(external.targets.is_empty());
    assert_eq!(external.invocation.dispatch, InvocationDispatch::Direct);
    assert!(external.invocation.isolated_ordered);
    assert_eq!(
        function(&graph, ":future()").execution,
        ExecutionTiming::Synchronous
    );
    graph.validate().unwrap();
    compare_checkpoint_if_requested(
        Checkpoint {
            runtime: &runtime,
            build: &build,
            input: &input,
            output: &output,
        },
        &graph,
    );
    assert_saved_expansion(&graph, directory.path());
}

fn function<'a>(graph: &'a GraphData, signature: &str) -> &'a FunctionFlowIR {
    let node = graph
        .nodes
        .iter()
        .find(|node| node.signature.contains(signature))
        .unwrap();
    graph
        .flow_inventory
        .as_ref()
        .unwrap()
        .functions
        .iter()
        .find(|flow| flow.symbol == node.id)
        .unwrap()
}

fn sites(function: &FunctionFlowIR) -> Vec<CallSite> {
    let mut sites = Vec::new();
    flow::visit_calls_mut(&mut function.clone(), &mut |site| sites.push(site.clone()));
    sites
}

// An immutable pre-v9 helper is optional for ordinary acceptance, and explicitly
// supplied by the Unit 2B compatibility run. Historical reads do not bypass fresh gates.
struct Checkpoint<'a> {
    runtime: &'a AnalyzerRuntime,
    build: &'a BuildTarget,
    input: &'a JavaInput,
    output: &'a Path,
}

fn compare_checkpoint_if_requested(context: Checkpoint<'_>, after: &GraphData) {
    let Checkpoint {
        runtime,
        build,
        input,
        output,
    } = context;
    let Some(library) = std::env::var_os("CODECALLNAVIGATOR_JAVA_EXECUTION_BASELINE") else {
        return;
    };
    let before_path = output.join("before-v9.tsv");
    run_process(
        Command::new(runtime.java_home.as_ref().unwrap().join("bin/java"))
            .args([
                "--add-exports=jdk.compiler/com.sun.tools.javac.code=ALL-UNNAMED",
                "--add-exports=jdk.compiler/com.sun.tools.javac.tree=ALL-UNNAMED",
            ])
            .arg("-cp")
            .arg(PathBuf::from(library).join("*"))
            .arg("codecallnavigator.bytecode.SourceInventory")
            .arg(&before_path)
            .arg(&input.path)
            .arg(&input.path)
            .arg(build.path.join("src")),
        &build.path,
        "baseline-source-inventory",
        &AtomicBool::new(false),
    )
    .unwrap();
    let inventory = source_inventory::SourceInventory::read(&before_path).unwrap();
    assert!(!inventory.has_execution_facts);
    let before = normalize::with_inventories(
        build,
        output,
        normalize::CompiledContext {
            inputs: std::slice::from_ref(input),
            owner: None,
            inventories: HashMap::from([(1, Arc::new(inventory))]),
        },
    )
    .unwrap();
    let mut before = serde_json::to_value(before).unwrap();
    let mut after = serde_json::to_value(after).unwrap();
    let before_flow = before["flow_inventory"].take();
    let mut after_flow = after["flow_inventory"].take();
    remove_facts(&mut after_flow);
    assert_eq!(
        before_flow, after_flow,
        "Source IR changed beyond additive compiler facts"
    );
    assert_eq!(
        before, after,
        "Mapped native graph/source projection changed"
    );
    eprintln!("PRE_V9_MAPPED_PROJECTION_EQUAL=true");
}

fn remove_facts(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            object.remove("execution");
            object.remove("invocation");
            for value in object.values_mut() {
                remove_facts(value);
            }
        }
        serde_json::Value::Array(array) => {
            for value in array {
                remove_facts(value);
            }
        }
        _ => (),
    }
}

fn assert_saved_expansion(graph: &GraphData, root: &Path) {
    use codecallnavigator_core::artifact;
    let roots = vec![("repo".into(), root.canonicalize().unwrap())];
    let bytes = artifact::canonical(graph, &roots).unwrap().bytes().unwrap();
    let saved = artifact::Artifact::from_bytes(&bytes).unwrap();
    let view = artifact::flow::View::new(&saved).unwrap();
    let callers: Vec<_> = view
        .symbols()
        .filter(|symbol| {
            view.describe(symbol)
                .unwrap()
                .is_some_and(|node| node.signature.starts_with("fixture.FinalOwner:run() -> "))
        })
        .collect();
    assert_eq!(
        callers.len(),
        1,
        "Known FinalOwner.run compiler owner must be unique"
    );
    // The compatibility helper above still needs the live file. Delete only the
    // owned source after that probe; all following inputs come from the artifact.
    std::fs::remove_file(root.join("src/fixture/Flow.java")).unwrap();
    compiler_expansion::assert_cached_expansion(
        &saved,
        compiler_expansion::Pair {
            caller: callers[0],
            call_text: "leaf()",
            label: "leaf()",
            callee_label: "leaf",
            callee_signature: "fixture.FinalOwner:leaf()",
            content: [include_str!("../../../../../fixtures/flow/java-execution/Flow.java"); 2],
        },
    );
    assert_eq!(saved.bytes().unwrap(), bytes);
}
