use super::*;
use crate::flow::{CallTarget, SourceSpan};
use serde_json::json;
use std::fs;

const CALLER: &str = "[CompilerAssembly]::App::Run()";
const TARGET: &str = "[CompilerAssembly]::App::Leaf()";
struct Fixture {
    root: tempfile::TempDir,
    graph: GraphData,
    indices: HashMap<String, u32>,
    owners: SourceOwners,
    snapshots: source::Snapshots,
    function: FunctionFlowIR,
}
impl Fixture {
    fn new(utf8: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("App.cs");
        let compiler = "class App { int Run()=>Leaf(); int Leaf()=>1; }";
        let content = format!("\u{feff}{compiler}");
        let bytes = if utf8 {
            content.as_bytes().to_vec()
        } else {
            [
                vec![0xff, 0xfe],
                compiler.encode_utf16().flat_map(u16::to_le_bytes).collect(),
            ]
            .concat()
        };
        fs::write(&file, bytes).unwrap();
        let file = file.canonicalize().unwrap();
        let build = root
            .path()
            .join("ActualProject.csproj")
            .to_string_lossy()
            .into_owned();
        let node = |raw: &str| Node {
            id: format!("{build}|{raw}"),
            build: build.clone(),
            project: "Evaluated owner".into(),
            file: file.to_string_lossy().into_owned(),
            language: "csharp".into(),
            kind: "method".into(),
            signature: raw.into(),
            ..Default::default()
        };
        let graph = GraphData {
            nodes: vec![node(CALLER), node(TARGET)],
            ..Default::default()
        };
        let owners = HashMap::from([(
            file.clone(),
            HashMap::from([(
                "CompilerAssembly".into(),
                super::super::SourceOwner {
                    project: "Evaluated owner".into(),
                    is_test: false,
                    is_generated: false,
                },
            )]),
        )]);
        let mut snapshots = source::Snapshots::default();
        snapshots
            .content(
                source::CompilerContext {
                    file: &file,
                    assembly: "CompilerAssembly",
                    code_page: None,
                },
                compiler,
            )
            .unwrap();
        let function=serde_json::from_value(json!({"symbol":CALLER,"file":file,"project":"CompilerAssembly","language":"csharp","span":{"start":3,"end":content.len()},"body":[{"kind":"action","text":"Leaf()","span":{"start":3,"end":8},"calls":[{"span":{"start":3,"end":8},"label":"Leaf()","targets":[{"symbol":TARGET,"certainty":"possible","provenance":"roslyn:call"}]}]}]})).unwrap();
        Self {
            root,
            graph,
            indices: HashMap::from([(CALLER.into(), 0), (TARGET.into(), 1)]),
            owners,
            snapshots,
            function,
        }
    }
    fn append(&mut self, functions: Vec<FunctionFlowIR>) -> Result<()> {
        append(
            &mut self.graph,
            FlowInventory {
                version: 1,
                functions,
            },
            BindingContext {
                root: self.root.path(),
                indices: &self.indices,
                snapshots: &self.snapshots,
                owners: &self.owners,
            },
        )
    }
}

#[test]
fn identity_and_ownership_are_copied_from_existing_nodes_and_survive_target_pruning() {
    let mut fixture = Fixture::new(true);
    fixture.graph.nodes[0].id = "authoritative-node-id".into();
    let nodes = fixture.graph.nodes.clone();
    fixture.append(vec![fixture.function.clone()]).unwrap();
    assert_eq!(
        serde_json::to_value(&fixture.graph.nodes).unwrap(),
        serde_json::to_value(&nodes).unwrap()
    );
    let inventory = fixture.graph.flow_inventory.as_mut().unwrap();
    let function = &inventory.functions[0];
    assert_eq!(function.symbol, nodes[0].id);
    assert_eq!(function.project, "Evaluated owner");
    assert_eq!(function.build, nodes[0].build);
    let mut calls = Vec::new();
    flow::visit_calls_mut(&mut inventory.functions[0], &mut |call| {
        calls.extend(call.targets.clone())
    });
    assert_eq!(
        calls,
        [CallTarget {
            symbol: nodes[1].id.clone(),
            certainty: "possible".into(),
            provenance: "roslyn:call".into(),
            ..Default::default()
        }]
    );
    inventory.retain_symbols(&HashSet::from([nodes[0].id.as_str()]));
    flow::visit_calls_mut(&mut inventory.functions[0], &mut |call| {
        assert!(call.targets.is_empty());
        assert!(call.diagnostic.is_some());
    });
}

#[test]
fn duplicate_missing_and_misattributed_facts_are_rejected() {
    let cases: [fn(&mut FunctionFlowIR); 6] = [
        |f| f.symbol = "missing".into(),
        |f| f.project = "WrongAssembly".into(),
        |f| f.file = "Wrong.cs".into(),
        |f| {
            flow::visit_calls_mut(f, &mut |call| {
                call.targets[0].symbol = "missing target".into()
            })
        },
        |f| flow::visit_calls_mut(f, &mut |call| call.targets[0].kind = "navigation".into()),
        |f| {
            f.body[0].span = SourceSpan {
                start: 4,
                end: 99999,
            }
        },
    ];
    for change in cases {
        let mut fixture = Fixture::new(true);
        let mut function = fixture.function.clone();
        change(&mut function);
        assert!(fixture.append(vec![function]).is_err());
    }
    let mut fixture = Fixture::new(true);
    assert!(
        fixture
            .append(vec![fixture.function.clone(), fixture.function.clone()])
            .is_err()
    );
}

#[test]
fn valid_non_utf8_and_unowned_source_keep_graphs_without_guessed_flows() {
    for utf8 in [false, true] {
        let mut fixture = Fixture::new(utf8);
        if utf8 {
            fixture.owners.clear();
        }
        fixture.append(vec![fixture.function.clone()]).unwrap();
        assert_eq!(fixture.graph.nodes.len(), 2);
        assert!(fixture.graph.flow_inventory.unwrap().functions.is_empty());
        assert!(!fixture.graph.diagnostics.is_empty());
    }
}
