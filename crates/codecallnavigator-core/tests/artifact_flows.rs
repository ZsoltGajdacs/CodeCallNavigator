use codecallnavigator_core::{
    architecture::{ArchitectureFacts, FileFacts},
    artifact::{
        self, Artifact,
        flow::{Reason, Selection, Unavailable, View},
    },
    flow::*,
    model::{GraphData, Node},
    query,
};
use serde_json::json;

const SOURCE: &str = "class F { int f(){ return target(); } }";
const BUILD: &str = "/gone/App.build";
const FILE: &str = "/gone/F.java";

fn graph() -> GraphData {
    let span = SourceSpan {
        start: 0,
        end: SOURCE.len(),
    };
    GraphData {
        nodes: vec![node("f", "F"), node("g", "Endpoint target")],
        flow_inventory: Some(FlowInventory {
            version: 1,
            functions: vec![FunctionFlowIR {
                execution: Default::default(),
                symbol: "f".into(),
                file: FILE.into(),
                build: BUILD.into(),
                project: "App".into(),
                language: "java".into(),
                span,
                body: vec![Statement {
                    span,
                    kind: StatementKind::Return {
                        value: Some(Expression::Value {
                            text: "target()".into(),
                            span,
                            regions: vec![],
                            calls: vec![CallSite {
                                invocation: Default::default(),
                                span,
                                label: "target()".into(),
                                targets: vec![CallTarget {
                                    symbol: "g".into(),
                                    kind: "call".into(),
                                    certainty: "possible".into(),
                                    provenance: "compiler candidate".into(),
                                }],
                                diagnostic: None,
                            }],
                        }),
                    },
                }],
                diagnostics: vec![],
            }],
        }),
        architecture: Some(ArchitectureFacts {
            version: 1,
            files: vec![FileFacts {
                file: FILE.into(),
                build: BUILD.into(),
                project: "App".into(),
                language: "java".into(),
                content: SOURCE.into(),
                ..Default::default()
            }],
        }),
        ..Default::default()
    }
}
fn node(id: &str, label: &str) -> Node {
    Node {
        id: id.into(),
        label: label.into(),
        signature: format!("F::{id}()"),
        language: "java".into(),
        file: FILE.into(),
        build: BUILD.into(),
        project: "App".into(),
        kind: "method".into(),
        line: 1,
        ..Default::default()
    }
}
fn fixture() -> Artifact {
    artifact::canonical(&graph(), &[("repo".into(), "/gone".into())]).unwrap()
}
fn symbol(artifact: &Artifact) -> &str {
    artifact.graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["label"] == "F")
        .unwrap()["id"]
        .as_str()
        .unwrap()
}
fn ready<'a>(view: &'a View<'_>) -> Box<artifact::flow::Ready<'a>> {
    let symbol = view
        .symbols()
        .find(|symbol| view.describe(symbol).unwrap().unwrap().label == "F")
        .unwrap();
    match view.select(symbol) {
        Selection::Ready(value) => value,
        other => panic!("Expected selected saved flow, got {other:?}"),
    }
}
fn unavailable(artifact: &Artifact, expected: Reason) {
    match View::new(artifact) {
        Err(value) => assert_eq!(value.reason, expected, "{}", value.message),
        Ok(view) => match view.select(symbol(artifact)) {
            Selection::Unavailable(value) => {
                assert_eq!(value.reason, expected, "{}", value.message)
            }
            other => panic!("Expected {expected:?}, got {other:?}"),
        },
    }
}

#[test]
fn borrows_exact_portable_source_and_resolves_endpoint_target_labels() {
    let artifact = fixture();
    let view = View::new(&artifact).unwrap();
    let selected = ready(&view);
    let stored = artifact.graph["architecture"]["files"][0]["content"]
        .as_str()
        .unwrap();
    assert_eq!(selected.content, stored);
    assert_eq!(selected.content.as_ptr(), ready(&view).content.as_ptr());
    assert_eq!(selected.function.file, "@repo/F.java");
    assert_eq!(selected.node.owner.file, "@repo/F.java");
    assert_eq!(selected.targets.len(), 1);
    assert_eq!(selected.targets[0].label, "Endpoint target");
    assert_eq!(
        selected.function.body,
        graph().flow_inventory.unwrap().functions[0]
            .body
            .iter()
            .cloned()
            .map(|mut statement| {
                if let StatementKind::Return {
                    value: Some(Expression::Value { calls, .. }),
                } = &mut statement.kind
                {
                    calls[0].targets[0].symbol = selected.targets[0].symbol.into();
                }
                statement
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn missing_membership_is_distinct_from_unavailable_saved_facts() {
    let mut artifact = fixture();
    assert!(matches!(
        View::new(&artifact).unwrap().select("not-in-this-endpoint"),
        Selection::Missing
    ));
    artifact
        .graph
        .as_object_mut()
        .unwrap()
        .remove("flow_inventory");
    unavailable(&artifact, Reason::MissingInventory);
    let mut artifact = fixture();
    artifact.graph["flow_inventory"]["functions"] = json!([]);
    unavailable(&artifact, Reason::MissingFunction);
    artifact.graph["flow_inventory"]["version"] = json!(2);
    unavailable(&artifact, Reason::InventoryVersion);
    let mut artifact = fixture();
    artifact
        .graph
        .as_object_mut()
        .unwrap()
        .remove("architecture");
    unavailable(&artifact, Reason::MissingSource);
}

#[test]
fn consumed_duplicate_nodes_functions_and_source_owners_are_not_first_matches() {
    let mut artifact = fixture();
    // Duplicate precisely the requested node, irrespective of canonical node ordering.
    let requested = artifact.graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["label"] == "F")
        .unwrap()
        .clone();
    artifact.graph["nodes"]
        .as_array_mut()
        .unwrap()
        .push(requested);
    unavailable(&artifact, Reason::DuplicateNode);
    let mut artifact = fixture();
    let duplicate = artifact.graph["flow_inventory"]["functions"][0].clone();
    artifact.graph["flow_inventory"]["functions"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    unavailable(&artifact, Reason::DuplicateFunction);
    let mut artifact = fixture();
    let duplicate = artifact.graph["architecture"]["files"][0].clone();
    artifact.graph["architecture"]["files"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    unavailable(&artifact, Reason::DuplicateSource);
}

#[test]
fn selected_function_must_match_all_node_ownership_fields_exactly() {
    for (field, value) in [
        ("build", "@other/App.build"),
        ("project", "Other"),
        ("file", "@repo/Other.java"),
        ("language", "Java"),
    ] {
        let mut artifact = fixture();
        artifact.graph["flow_inventory"]["functions"][0][field] = json!(value);
        unavailable(&artifact, Reason::FunctionOwnership);
    }
}

#[test]
fn same_filename_in_another_project_never_supplies_selected_source() {
    let mut artifact = fixture();
    let mut other = artifact.graph["architecture"]["files"][0].clone();
    other["project"] = json!("Other");
    other["content"] = json!("wrong endpoint source");
    artifact.graph["architecture"]["files"]
        .as_array_mut()
        .unwrap()
        .insert(0, other);
    assert_eq!(ready(&View::new(&artifact).unwrap()).content, SOURCE);
    artifact.graph["architecture"]["files"]
        .as_array_mut()
        .unwrap()
        .pop();
    unavailable(&artifact, Reason::MissingSource);
}

#[test]
fn shared_range_and_source_size_limits_reject_selected_facts() {
    let mut artifact = fixture();
    artifact.graph["flow_inventory"]["functions"][0]["span"]["end"] = json!(SOURCE.len() + 1);
    unavailable(&artifact, Reason::InvalidFlow);
    let mut artifact = fixture();
    artifact.graph["architecture"]["files"][0]["content"] =
        json!("x".repeat(query::MAX_SOURCE_BYTES as usize + 1));
    unavailable(&artifact, Reason::InvalidFlow);
    let mut artifact = fixture();
    artifact.graph["architecture"]["files"][0]["content"] = json!("é".repeat(SOURCE.len()));
    artifact.graph["flow_inventory"]["functions"][0]["span"]["start"] = json!(1);
    unavailable(&artifact, Reason::InvalidFlow);
}

#[test]
fn malformed_inventory_is_rejected_before_any_flow_is_exposed() {
    let mut artifact = fixture();
    artifact.graph["flow_inventory"]["functions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"symbol":"unselected-old-facts","body":{"incompatible":"shape"}}));
    assert!(matches!(
        View::new(&artifact),
        Err(Unavailable {
            reason: Reason::InvalidFlow,
            ..
        })
    ));
}

#[test]
fn ordinary_flow_response_and_error_are_preserved() {
    let data = graph();
    let function = &data.flow_inventory.as_ref().unwrap().functions[0];
    let expected = json!({"function":function,"chart":codecallnavigator_core::flow::build(function).unwrap(),"source":{"origin":"analyzed","file":FILE,"symbol":"f","line":1,"content":SOURCE}});
    let view = View::from_parts(&data, data.flow_inventory.as_ref()).unwrap();
    let actual = query::function_flow(&view, "f").unwrap();
    assert_eq!(
        serde_json::to_vec(&actual).unwrap(),
        serde_json::to_vec(&expected).unwrap()
    );
    let mut invalid = graph();
    invalid.flow_inventory.as_mut().unwrap().functions[0]
        .span
        .end += 1;
    let view = View::from_parts(&invalid, invalid.flow_inventory.as_ref()).unwrap();
    let error = query::function_flow(&view, "f").unwrap_err();
    assert_eq!(error.code, "flow_unavailable");
    assert_eq!(
        error.message,
        "Function range does not match its analyzed UTF-8 source"
    );
}

fn selected_node_mut(artifact: &mut Artifact) -> &mut serde_json::Value {
    artifact.graph["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["label"] == "F")
        .unwrap()
}

#[test]
fn unavailable_bodies_and_invalid_endpoint_targets_are_explicit() {
    let mut artifact = fixture();
    selected_node_mut(&mut artifact)["body_unavailable"] = json!(true);
    unavailable(&artifact, Reason::BodyUnavailable);
    let mut artifact = fixture();
    selected_node_mut(&mut artifact)["language"] = json!("rust");
    unavailable(&artifact, Reason::UnsupportedLanguage);
    let mut artifact = fixture();
    artifact.graph["nodes"]
        .as_array_mut()
        .unwrap()
        .retain(|node| node["label"] == "F");
    unavailable(&artifact, Reason::InvalidTarget);
    let mut artifact = fixture();
    let target = artifact.graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["label"] != "F")
        .unwrap()
        .clone();
    artifact.graph["nodes"].as_array_mut().unwrap().push(target);
    unavailable(&artifact, Reason::InvalidTarget);
}

#[test]
fn unknown_source_owner_and_malformed_selected_facts_do_not_become_ready() {
    let mut artifact = fixture();
    selected_node_mut(&mut artifact)["file"] = json!("");
    artifact.graph["flow_inventory"]["functions"][0]["file"] = json!("");
    artifact.graph["architecture"]["files"][0]["file"] = json!("");
    unavailable(&artifact, Reason::MissingSource);
    let mut artifact = fixture();
    artifact.graph["flow_inventory"]["functions"][0]["body"] = json!("not statements");
    unavailable(&artifact, Reason::InvalidFlow);
    let mut artifact = fixture();
    artifact.graph["architecture"]["files"][0]
        .as_object_mut()
        .unwrap()
        .remove("content");
    unavailable(&artifact, Reason::MissingSource);
    let mut artifact = fixture();
    artifact.graph["architecture"]["files"][0]["content"] = json!(42);
    unavailable(&artifact, Reason::InvalidSource);
}

#[test]
fn target_summaries_stay_with_each_endpoint_metadata() {
    let before = fixture();
    let mut after = fixture();
    let target = after.graph["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["label"] != "F")
        .unwrap();
    target["label"] = json!("After endpoint target");
    let before_view = View::new(&before).unwrap();
    let after_view = View::new(&after).unwrap();
    let old = ready(&before_view);
    let new = ready(&after_view);
    assert_eq!(old.targets[0].label, "Endpoint target");
    assert_eq!(new.targets[0].label, "After endpoint target");
    assert_eq!(old.targets[0].symbol, new.targets[0].symbol);
}

#[test]
fn ordinary_flow_source_limit_build_and_availability_errors_are_preserved() {
    let mut data = graph();
    data.architecture.as_mut().unwrap().files[0].content =
        "x".repeat(query::MAX_SOURCE_BYTES as usize + 1);
    assert_ordinary_error(
        data,
        "flow_unavailable",
        "The analyzed source exceeds the 16 MiB flow source limit",
    );
    let mut data = graph();
    let function = &mut data.flow_inventory.as_mut().unwrap().functions[0];
    function.body = vec![
        Statement {
            span: function.span,
            kind: StatementKind::Action {
                text: "step".into(),
                calls: vec![]
            }
        };
        MAX_STEPS
    ];
    let expected = format!(
        "{:#}",
        codecallnavigator_core::flow::build(function).unwrap_err()
    );
    assert_ordinary_error(data, "flow_unavailable", &expected);
    let mut data = graph();
    data.flow_inventory = None;
    assert_ordinary_error(
        data,
        "flow_unavailable",
        "Refresh this workspace to collect function flowcharts.",
    );
    let mut data = graph();
    data.architecture = None;
    assert_ordinary_error(
        data,
        "source_unavailable",
        "Refresh to collect analyzed source for this declaration",
    );
}

fn assert_ordinary_error(data: GraphData, code: &str, message: &str) {
    let view = View::from_parts(&data, data.flow_inventory.as_ref()).unwrap();
    let error = query::function_flow(&view, "f").unwrap_err();
    assert_eq!(error.code, code);
    assert_eq!(error.message, message);
}

struct SourceCase<'a> {
    language: &'a str,
    file: &'a str,
    body: &'a str,
}

fn encoded_language_graph(case: SourceCase<'_>) -> GraphData {
    let SourceCase {
        language,
        file,
        body,
    } = case;
    let content = format!("\u{feff}// café 🧭\r\n{body}\r\n");
    let mut data = graph();
    for node in &mut data.nodes {
        node.language = language.into();
        node.file = file.into();
    }
    let function = &mut data.flow_inventory.as_mut().unwrap().functions[0];
    function.language = language.into();
    function.file = file.into();
    function.span = SourceSpan {
        start: 0,
        end: content.len(),
    };
    let start = content.find("target()").unwrap();
    let call_span = SourceSpan {
        start,
        end: start + "target()".len(),
    };
    function.body[0].span = function.span;
    if let StatementKind::Return {
        value: Some(Expression::Value { span, calls, .. }),
    } = &mut function.body[0].kind
    {
        *span = call_span;
        calls[0].span = call_span;
    }
    let source = &mut data.architecture.as_mut().unwrap().files[0];
    source.language = language.into();
    source.file = file.into();
    source.content = content;
    data
}

#[test]
fn unicode_bom_crlf_flow_matches_current_query_after_explicit_owner_rebasing() {
    for (language, file, body) in [
        (
            "java",
            "/gone/F.java",
            "class F { int f(){ return target(); } int target(){return 1;} }",
        ),
        (
            "csharp",
            "/gone/F.cs",
            "class F { int f(){ return target(); } int target(){return 1;} }",
        ),
        (
            "typescript",
            "/gone/F.ts",
            "function f(){ return target(); } function target(){return 1;}",
        ),
    ] {
        let data = encoded_language_graph(SourceCase {
            language,
            file,
            body,
        });
        let artifact = artifact::canonical(&data, &[("repo".into(), "/gone".into())]).unwrap();
        let view = View::new(&artifact).unwrap();
        let selected = ready(&view);
        let stored = artifact.graph["architecture"]["files"][0]["content"]
            .as_str()
            .unwrap();
        assert_eq!(
            (selected.content, selected.content.as_ptr()),
            (stored, ready(&view).content.as_ptr())
        );
        assert_eq!(
            &selected.content
                [selected.function.body[0].span.start..selected.function.body[0].span.end],
            stored
        );
        let hydrated = artifact
            .hydrate(&[("repo".into(), "/gone".into())])
            .unwrap();
        let current_view = View::from_parts(&hydrated, hydrated.flow_inventory.as_ref()).unwrap();
        let current = query::function_flow(&current_view, selected.node.symbol).unwrap();
        assert_eq!(current["source"]["content"], stored);
        let mut saved_chart = serde_json::to_value(&selected.chart).unwrap();
        assert_eq!(saved_chart["file"], file.replacen("/gone/", "@repo/", 1));
        assert_eq!(current["chart"]["file"], file);
        // Source ownership differs by documented portable/local path representation only.
        saved_chart["file"] = json!(file);
        assert_eq!(
            serde_json::to_vec(&saved_chart).unwrap(),
            serde_json::to_vec(&current["chart"]).unwrap()
        );
    }
}

#[test]
fn ordinary_lookup_availability_messages_keep_the_legacy_error_contract() {
    let mut data = graph();
    data.nodes[0].language = "rust".into();
    data.flow_inventory = None;
    assert_ordinary_error(
        data,
        "flow_unavailable",
        "Function flowcharts currently support Java, TypeScript and C#.",
    );
    let mut data = graph();
    data.nodes[0].body_unavailable = true;
    assert_ordinary_error(
        data,
        "flow_unavailable",
        "No source body is available for this declaration.",
    );
    let mut data = graph();
    data.flow_inventory.as_mut().unwrap().functions.clear();
    assert_ordinary_error(
        data,
        "flow_unavailable",
        "No unambiguous source flow was exported for this callable. Refresh an older graph; unsupported or ambiguous bodies remain unavailable.",
    );
}

#[test]
fn target_body_availability_is_an_endpoint_fact_not_inferred_from_inventory() {
    let mut artifact = fixture();
    artifact.graph["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["label"] != "F")
        .unwrap()["body_unavailable"] = json!(true);
    let view = View::new(&artifact).unwrap();
    let selected = ready(&view);
    assert!(!selected.node.body_unavailable);
    assert!(selected.targets[0].body_unavailable);
    artifact.graph["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["label"] != "F")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("body_unavailable");
    assert!(!ready(&View::new(&artifact).unwrap()).targets[0].body_unavailable);
}
