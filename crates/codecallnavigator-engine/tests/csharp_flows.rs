//! Production C# flow ingestion and compiler-free source queries.
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[path = "support/compiler_expansion.rs"]
mod compiler_expansion;

const PROJECT: &str = "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net11.0</TargetFramework></PropertyGroup></Project>";
const SOURCE: &str = "\u{feff}class App {\r\n static int Leaf(int x)=>x; static int Leaf(string x)=>x.Length; public int Run(bool flag) { string text=\"é😀\"; if(flag) return Leaf(1); return Leaf(text); }\r\n}";
const GENERATED_SOURCE: &str =
    "public static class Generated { public static int GeneratedLeaf()=>5; }";

fn engine() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
}

fn query(root: &Path, operation: Value) -> Value {
    bound_query(root, operation, None)
}

fn bound_query(root: &Path, operation: Value, generation: Option<&str>) -> Value {
    let reply = query_reply(root, operation, generation);
    assert_eq!(reply.code, 0, "{}", reply.body);
    reply.body
}

struct QueryReply {
    code: i32,
    body: Value,
}

fn query_reply(root: &Path, operation: Value, generation: Option<&str>) -> QueryReply {
    let mut child = engine()
        .arg("query")
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            json!({"version":2,"project":root,"expected_generation":generation,"query":operation})
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    QueryReply {
        code: output.status.code().unwrap(),
        body: response,
    }
}

#[test]
fn csharp_is_advertised_as_a_flow_language() {
    let output = engine().arg("capabilities").output().unwrap();
    assert!(output.status.success());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        response["flow_languages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|language| language == "csharp")
    );
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn csharp_flow_queries_keep_exact_snapshot_and_overload_targets_without_dotnet() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::write(root.join("Fixture.csproj"), PROJECT).unwrap();
    fs::write(root.join("App.cs"), SOURCE).unwrap();
    let output = engine()
        .arg("analyze")
        .arg("--project")
        .arg(root)
        .arg("Fixture.csproj")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let snapshot = query(root, json!({"kind":"snapshot"}));
    let node = snapshot["result"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["label"] == "Run")
        .unwrap();
    let symbol = node["id"].as_str().unwrap();
    let flow = bound_query(
        root,
        json!({"kind":"flow","symbol":symbol}),
        snapshot["generation"].as_str(),
    );
    assert_eq!(flow["result"]["source"]["content"], SOURCE);
    assert_eq!(flow["generation"], snapshot["generation"]);
    assert_eq!(flow["result"]["function"]["symbol"], symbol);
    let serialized = flow["result"]["function"].to_string();
    assert!(serialized.contains("Leaf(System.Int32)"));
    assert!(serialized.contains("Leaf(System.String)"));
    assert!(serialized.contains("roslyn:call"));
    assert_persistence_and_composition(root);
    fs::write(root.join("App.cs"), "changed source").unwrap();
    assert_eq!(
        query(root, json!({"kind":"flow","symbol":symbol}))["result"]["source"]["content"],
        SOURCE
    );
    fs::remove_file(root.join("App.cs")).unwrap();
    assert_eq!(
        query(root, json!({"kind":"flow","symbol":symbol}))["result"]["source"]["content"],
        SOURCE
    );
    assert_cached_compiler_expansion(root, symbol);
}

fn assert_persistence_and_composition(root: &Path) {
    use codecallnavigator_core::{artifact, flow, method_policy, store::SqliteStore};
    use std::{collections::HashSet, sync::atomic::AtomicBool};
    let roots = vec![("repo".into(), root.canonicalize().unwrap())];
    let artifact = artifact::load(root).unwrap();
    let graph = artifact.hydrate(&roots).unwrap();
    let expected = serde_json::to_value(&graph.flow_inventory).unwrap();
    let cache = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(cache.path()).unwrap();
    store.publish(&graph, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        serde_json::to_value(store.load_active().unwrap().unwrap().flow_inventory).unwrap(),
        expected
    );
    let roundtrip = artifact::canonical(&graph, &roots).unwrap();
    assert_eq!(
        serde_json::to_value(roundtrip.hydrate(&roots).unwrap().flow_inventory).unwrap(),
        expected
    );
    let mut composed = None;
    flow::merge_inventory(&mut composed, graph.flow_inventory.clone()).unwrap();
    flow::merge_inventory(&mut composed, graph.flow_inventory.clone()).unwrap();
    assert_eq!(serde_json::to_value(&composed).unwrap(), expected);
    let target = graph
        .nodes
        .iter()
        .find(|node| node.signature.ends_with("Leaf(System.String)"))
        .unwrap()
        .id
        .clone();
    let mut pruned = graph.clone();
    method_policy::prune(&mut pruned, &HashSet::from([target.clone()])).unwrap();
    for function in &mut pruned.flow_inventory.unwrap().functions {
        assert_ne!(function.symbol, target);
        flow::visit_calls_mut(function, &mut |call| {
            assert!(
                call.targets
                    .iter()
                    .all(|candidate| candidate.symbol != target)
            )
        });
    }
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn source_accessors_local_functions_and_top_level_calls_keep_existing_identities() {
    use codecallnavigator_core::flow;
    use std::collections::HashSet;
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::write(
        root.join("Fixture.csproj"),
        PROJECT.replace(
            "</PropertyGroup>",
            "<OutputType>Exe</OutputType></PropertyGroup>",
        ),
    )
    .unwrap();
    let source = "System.Console.WriteLine(App.Start());\nclass App { public int Value { get { return Leaf(4); } } static int Leaf(int x)=>x; public static int Start() { int Local()=>Leaf(2); return Local(); } }";
    fs::write(root.join("App.cs"), source).unwrap();
    let output = engine()
        .args(["analyze", "--project"])
        .arg(root)
        .arg("Fixture.csproj")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let snapshot = query(root, json!({"kind":"snapshot"}));
    let nodes = snapshot["result"]["nodes"].as_array().unwrap();
    let identities: HashSet<_> = nodes
        .iter()
        .map(|node| node["id"].as_str().unwrap())
        .collect();
    for expected in ["get_Value", "Local", "Start"] {
        assert!(nodes.iter().any(|node| node["label"] == expected));
    }
    assert!(nodes.iter().any(|node| node["kind"] == "toplevel"));
    for node in nodes {
        let result = bound_query(
            root,
            json!({"kind":"flow","symbol":node["id"]}),
            snapshot["generation"].as_str(),
        );
        assert_eq!(result["result"]["source"]["content"], source);
        let mut function: flow::FunctionFlowIR =
            serde_json::from_value(result["result"]["function"].clone()).unwrap();
        assert_eq!(function.project, node["project"].as_str().unwrap());
        flow::visit_calls_mut(&mut function, &mut |call| {
            for target in &call.targets {
                assert!(identities.contains(target.symbol.as_str()));
                assert!(!target.provenance.is_empty());
            }
        });
    }
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn generated_compile_sources_and_bodyless_declarations_preserve_graph_policy() {
    let directory = generated_fixture();
    let root = directory.path();
    let output = engine()
        .args(["analyze", "--project"])
        .arg(root)
        .arg("Fixture.csproj")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact = codecallnavigator_core::artifact::working_manifest_path(root).unwrap();
    let saved = fs::read(&artifact).unwrap();
    let snapshot = query(root, json!({"kind":"snapshot"}));
    let graph = &snapshot["result"];
    let nodes = graph["nodes"].as_array().unwrap();
    let generated_node = nodes
        .iter()
        .find(|node| node["label"] == "GeneratedLeaf")
        .unwrap();
    assert_eq!(generated_node["is_generated"], true);
    assert!(
        generated_node["signature"]
            .as_str()
            .unwrap()
            .ends_with("[Fixture]::Generated::GeneratedLeaf()")
    );
    let flow = bound_query(
        root,
        json!({"kind":"flow","symbol":generated_node["id"]}),
        snapshot["generation"].as_str(),
    );
    assert_eq!(flow["result"]["source"]["content"], GENERATED_SOURCE);
    assert_eq!(flow["result"]["function"]["symbol"], generated_node["id"]);
    let caller = nodes
        .iter()
        .position(|node| node["label"] == "Run")
        .unwrap();
    let target = nodes
        .iter()
        .position(|node| node["label"] == "GeneratedLeaf")
        .unwrap();
    assert!(
        graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["from"] == caller && edge["to"] == target)
    );
    assert_bodyless_declarations(root, &snapshot);
    assert!(nodes.iter().all(|node| !matches!(
        node["kind"].as_str().unwrap(),
        "constructor" | "cctor" | "fieldinit" | "field" | "delegate"
    )));
    assert_eq!(query(root, json!({"kind":"snapshot"}))["result"], *graph);
    assert_eq!(fs::read(artifact).unwrap(), saved);
}

fn generated_fixture() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::write(
        root.join("Fixture.csproj"),
        PROJECT.replace(
            "</Project>",
            "<ItemGroup><Compile Update=\"Generated.cs\" AutoGen=\"true\" /></ItemGroup></Project>",
        ),
    )
    .unwrap();
    fs::write(root.join("Generated.cs"), GENERATED_SOURCE).unwrap();
    fs::write(root.join("App.cs"),"public interface IContract { int Missing(); } public abstract class AbstractBase { public abstract int MissingAbstract(); } public class App { public static int Run()=>Generated.GeneratedLeaf(); }").unwrap();
    directory
}

fn assert_bodyless_declarations(root: &Path, snapshot: &Value) {
    let nodes = snapshot["result"]["nodes"].as_array().unwrap();
    for name in ["Missing", "MissingAbstract"] {
        let declaration = nodes.iter().find(|node| node["label"] == name).unwrap();
        let reply = query_reply(
            root,
            json!({"kind":"flow","symbol":declaration["id"]}),
            snapshot["generation"].as_str(),
        );
        assert_eq!(reply.code, 2);
        assert_eq!(reply.body["error"]["code"], "flow_unavailable");
        let message = reply.body["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("No unambiguous source flow") || message.contains("No source body")
        );
    }
}

#[test]
#[ignore = "requires pinned CallAtlas and installed .NET 11 SDK"]
fn cross_file_partial_method_keeps_graph_when_flow_source_cannot_match_native_owner() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let project = PROJECT
        .replace("</PropertyGroup>", "<EnableDefaultCompileItems>false</EnableDefaultCompileItems></PropertyGroup>")
        .replace("</Project>", "<ItemGroup><Compile Include=\"ADefinition.cs\"/><Compile Include=\"ZImplementation.cs\"/></ItemGroup></Project>");
    fs::write(root.join("Fixture.csproj"), project).unwrap();
    fs::write(
        root.join("ADefinition.cs"),
        "public partial class App { public partial int F(); public int Run()=>F(); }",
    )
    .unwrap();
    fs::write(
        root.join("ZImplementation.cs"),
        "public partial class App { public partial int F()=>1; }",
    )
    .unwrap();
    let output = engine()
        .args(["analyze", "--project"])
        .arg(root)
        .arg("Fixture.csproj")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let snapshot = query(root, json!({"kind":"snapshot"}));
    let graph = &snapshot["result"];
    let nodes = graph["nodes"].as_array().unwrap();
    let target = nodes.iter().position(|node| node["label"] == "F").unwrap();
    let caller = nodes
        .iter()
        .position(|node| node["label"] == "Run")
        .unwrap();
    assert!(
        graph["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| edge["from"] == caller && edge["to"] == target)
    );
    assert!(
        nodes[target]["file"]
            .as_str()
            .unwrap()
            .ends_with("ADefinition.cs")
    );
    let reply = query_reply(
        root,
        json!({"kind":"flow","symbol":nodes[target]["id"]}),
        snapshot["generation"].as_str(),
    );
    assert_eq!(reply.code, 2);
    assert_eq!(reply.body["error"]["code"], "flow_unavailable");
    assert_eq!(
        query(root, json!({"kind":"snapshot"}))["generation"],
        snapshot["generation"]
    );
}

fn assert_cached_compiler_expansion(root: &Path, caller: &str) {
    use codecallnavigator_core::artifact;
    let path = artifact::working_manifest_path(root).unwrap();
    let bytes = fs::read(&path).unwrap();
    let saved = artifact::load(root).unwrap();
    compiler_expansion::assert_cached_expansion(
        &saved,
        compiler_expansion::Pair {
            caller,
            call_text: "Leaf(1)",
            label: "Leaf(1)",
            callee_label: "Leaf",
            callee_signature: "Leaf(System.Int32)",
            content: [SOURCE, SOURCE],
        },
    );
    assert_eq!(fs::read(path).unwrap(), bytes);
}
