//! Explicit two-function snapshot fixture through the production builder and Graphviz.
use crate::app::State;
use codegraph::{
    architecture::{ArchitectureFacts, FileFacts},
    flow::*,
    graph::GraphIndex,
    model::{Edge, GraphData, Node},
};
use std::sync::Arc;
const SOURCE: &str = "function load(x: boolean) {\n  if (x) return save();\n  return 0;\n}\nfunction save() { return 1; }\n";
const FILE: &str = "/snapshot/flow.ts";
const BUILD: &str = "/snapshot/tsconfig.json";
fn span(text: &str) -> SourceSpan {
    let start = if text == "x" {
        SOURCE.find("if (x)").unwrap() + 4
    } else {
        SOURCE.find(text).unwrap()
    };
    SourceSpan {
        start,
        end: start + text.len(),
    }
}
fn value(text: &str, calls: Vec<CallSite>) -> Expression {
    Expression::Value {
        text: text.into(),
        span: span(text),
        calls,
        regions: Vec::new(),
    }
}
fn statement(kind: StatementKind, text: &str) -> Statement {
    Statement {
        kind,
        span: span(text),
    }
}
fn function(symbol: &str, body: Vec<Statement>) -> FunctionFlowIR {
    FunctionFlowIR {
        symbol: symbol.into(),
        file: FILE.into(),
        build: BUILD.into(),
        project: "Snapshot".into(),
        language: "typescript".into(),
        span: if symbol == "flow/save" {
            SourceSpan {
                start: SOURCE.find("function save").unwrap(),
                end: SOURCE.len(),
            }
        } else {
            SourceSpan {
                start: 0,
                end: SOURCE.find("function save").unwrap(),
            }
        },
        body,
        diagnostics: Vec::new(),
    }
}
pub fn data() -> GraphData {
    GraphData {
        flow_inventory: Some(inventory()),
        ..graph_data()
    }
}
fn inventory() -> FlowInventory {
    let load = function(
        "flow/load",
        vec![
            statement(
                StatementKind::If {
                    condition: value("x", Vec::new()),
                    then_branch: vec![statement(
                        StatementKind::Return {
                            value: Some(value(
                                "save()",
                                vec![CallSite {
                                    span: span("save()"),
                                    label: "save".into(),
                                    targets: vec![CallTarget {
                                        symbol: "flow/save".into(),
                                        certainty: "direct".into(),
                                        provenance: "explicit fixture".into(),
                                    }],
                                    diagnostic: None,
                                }],
                            )),
                        },
                        "return save();",
                    )],
                    else_branch: Vec::new(),
                },
                "if (x) return save();",
            ),
            statement(
                StatementKind::Return {
                    value: Some(value("0", Vec::new())),
                },
                "return 0;",
            ),
        ],
    );
    let save = function(
        "flow/save",
        vec![statement(
            StatementKind::Return {
                value: Some(value("1", Vec::new())),
            },
            "return 1;",
        )],
    );
    FlowInventory {
        version: INVENTORY_VERSION,
        functions: vec![load, save],
    }
}
fn graph_data() -> GraphData {
    GraphData {
        nodes: [
            ("flow/load", "load(x: boolean)", 1),
            ("flow/save", "save()", 5),
        ]
        .into_iter()
        .map(|(id, label, line)| Node {
            id: id.into(),
            label: label.into(),
            signature: label.into(),
            language: "typescript".into(),
            kind: "function".into(),
            build: BUILD.into(),
            project: "Snapshot".into(),
            container: "flow.ts".into(),
            file: FILE.into(),
            line,
            ..Default::default()
        })
        .collect(),
        edges: vec![Edge {
            from: 0,
            to: 1,
            kind: "call".into(),
            certainty: "direct".into(),
            count: 1,
            provenance: "explicit fixture".into(),
        }],
        architecture: Some(ArchitectureFacts {
            version: 1,
            files: vec![FileFacts {
                build: BUILD.into(),
                project: "Snapshot".into(),
                file: FILE.into(),
                language: "typescript".into(),
                content: SOURCE.into(),
                ..Default::default()
            }],
        }),
        ..Default::default()
    }
}
pub fn install(state: &mut State) {
    crate::scenes::install(state);
    state.graph = Some(Arc::new(GraphIndex::new(Arc::new(data())).unwrap()));
    state.nav = crate::app::Navigation::default();
    state.nav.selected = Some("flow/load".into());
    state.replace_graph_view(crate::projection::project(
        state.graph.as_ref().unwrap(),
        &state.request().unwrap(),
    ));
    crate::scenes::position(&mut state.graph_view);
    state.runtime = codegraph::analyzers::AnalyzerRuntime::discover()
        .ok()
        .map(Arc::new);
    state.needs_layout = false;
    state.open_selected_flow();
}
