//! Real TypeScript execution traces must be paths through the compiler-exported flow.
use codecallnavigator_engine::{
    analyzers::{AnalyzerRuntime, analyze},
    discovery::target_from_file,
    flow::{FlowChart, build},
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::AtomicBool,
};

const PROGRAM: &str = r#"const events:string[]=[];
function mark(value:string){return events.push(value)}
function predicate(){return true}
export function work(mode:number){
  const object={value:mode===2?mark("object-yes"):mark("object-no")};
  const template=`value ${mode===2?mark("template-yes"):mark("template-no")}`;
  const unary=!(mode===2?mark("unary-yes"):mark("unary-no"));
  for(const item of mode===2?[1]:[]){mark("iteration");}
  switch(mode){default:mark("switch-default");break;case (mark("switch-test"),2):mark("switch-two");break;}
  mode===2 ? mark("ternary-yes") : mark("ternary-no");
  if(mode===2 && predicate()){mark("and");}
  try { mark("try"); if(mode===0){mark("early");return 1;} if(mode===1){mark("throwing");throw new Error("failure");} mark("body"); }
  catch(error){ mark("catch"); return 2; }
  finally { mark("finally"); }
  mark("after"); return 3;
}
const traces=[0,1,2].map(mode=>{events.length=0;work(mode);return [...events]});
console.log(JSON.stringify(traces));
"#;

fn fixture() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().unwrap();
    let engine = std::env::var_os("CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../PacificFateMapEditor/node_modules/typescript")
        })
        .canonicalize()
        .expect("Set CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT to installed TypeScript 6.0.x");
    fs::create_dir(temporary.path().join("node_modules")).unwrap();
    symlink(engine, temporary.path().join("node_modules/typescript")).unwrap();
    fs::write(temporary.path().join("tsconfig.json"),r#"{"compilerOptions":{"strict":true,"target":"ES2023","module":"esnext","moduleResolution":"bundler","types":[],"noEmit":true},"files":["main.ts"]}"#).unwrap();
    fs::write(temporary.path().join("main.ts"), PROGRAM).unwrap();
    temporary
}
fn marker(label: &str) -> Option<String> {
    label
        .strip_prefix("mark(\"")
        .and_then(|s| s.strip_suffix("\")"))
        .map(str::to_owned)
}
fn chart_traces(chart: &FlowChart) -> HashSet<Vec<String>> {
    let mut traces = HashSet::new();
    let mut pending = vec![(chart.entry, Vec::new(), HashMap::new())];
    while let Some((index, mut events, mut visited)) = pending.pop() {
        let count = visited.entry(index).or_insert(0u8);
        if *count >= 2 {
            continue;
        }
        *count += 1;
        let step = &chart.steps[index];
        events.extend(
            step.calls
                .iter()
                .filter_map(|call| marker(&PROGRAM[call.span.start..call.span.end])),
        );
        if step.kind == "exit" {
            traces.insert(events);
            continue;
        }
        for link in chart.links.iter().filter(|link| link.from == index) {
            pending.push((link.to, events.clone(), visited.clone()));
        }
    }
    traces
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn actual_typescript_traces_follow_finally_and_never_resume_after_return() {
    let fixture = fixture();
    let execution = Command::new("node")
        .args(["--experimental-strip-types", "--no-warnings"])
        .arg(fixture.path().join("main.ts"))
        .output()
        .unwrap();
    assert!(
        execution.status.success(),
        "{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    let actual: Vec<Vec<String>> = serde_json::from_slice(&execution.stdout).unwrap();
    let graph = analyze(
        vec![target_from_file(&fixture.path().join("tsconfig.json")).unwrap()],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    let method = graph
        .nodes
        .iter()
        .find(|node| node.label == "work")
        .unwrap();
    let inventory = graph.flow_inventory.as_ref().unwrap();
    let function = inventory
        .functions
        .iter()
        .find(|function| function.symbol == method.id)
        .unwrap();
    let chart = build(function).unwrap();
    let paths = chart_traces(&chart);
    for trace in actual {
        assert!(paths.contains(&trace), "Missing execution path {trace:?}");
    }
    for trace in paths
        .iter()
        .filter(|trace| trace.iter().any(|e| e == "early" || e == "catch"))
    {
        assert!(
            !trace.iter().any(|event| event == "after"),
            "Abrupt exit resumed after finally: {trace:?}"
        );
    }
}

#[test]
#[ignore = "requires Node 24+ and installed TypeScript 6.0.x"]
fn deep_source_flow_is_partial_without_breaking_graph_deserialization() {
    let fixture = fixture();
    let source = format!(
        "export function work(flag:boolean){{{}return 1;{}return 0;}}",
        "if(flag){".repeat(160),
        "}".repeat(160)
    );
    fs::write(fixture.path().join("main.ts"), source).unwrap();
    let graph = analyze(
        vec![target_from_file(&fixture.path().join("tsconfig.json")).unwrap()],
        &AnalyzerRuntime::discover().unwrap(),
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap();
    assert!(graph.nodes.iter().any(|node| node.label == "work"));
    let function = &graph.flow_inventory.as_ref().unwrap().functions[0];
    assert!(
        function
            .diagnostics
            .iter()
            .any(|message| message.contains("budget"))
    );
    assert!(build(function).unwrap().partial);
}
