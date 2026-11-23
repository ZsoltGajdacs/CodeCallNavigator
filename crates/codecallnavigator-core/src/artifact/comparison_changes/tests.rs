use super::*;
use crate::{
    architecture::{ArchitectureFacts, FACTS_VERSION, FileFacts},
    flow::{FlowInventory, FunctionFlowIR, SourceSpan},
    model::{Edge, EntryPoints},
};

fn snapshot(source: &str, certainty: &str) -> GraphData {
    let node = |id: &str, line| Node {
        id: id.into(),
        label: id.into(),
        signature: format!("{id}()"),
        language: "csharp".into(),
        kind: "method".into(),
        build: "@repo/App.csproj".into(),
        project: "App".into(),
        container: "Example.OrderService".into(),
        file: "@repo/OrderService.cs".into(),
        line,
        ..Node::default()
    };
    let file = FileFacts {
        build: "@repo/App.csproj".into(),
        project: "App".into(),
        language: "csharp".into(),
        file: "@repo/OrderService.cs".into(),
        names: vec!["Example.OrderService".into(), "OrderService".into()],
        kinds: vec!["class".into()],
        content: source.into(),
        ..FileFacts::default()
    };
    let flow = |symbol: &str, text: &str| {
        let start = source.find(text).unwrap();
        FunctionFlowIR {
            symbol: symbol.into(),
            build: file.build.clone(),
            project: file.project.clone(),
            language: file.language.clone(),
            file: file.file.clone(),
            span: SourceSpan {
                start,
                end: start + text.len(),
            },
            ..FunctionFlowIR::default()
        }
    };
    let functions = vec![flow("entry", "Run();"), flow("run", "return")];
    GraphData {
        nodes: vec![node("entry", 3), node("run", 4)],
        edges: vec![Edge {
            from: 0,
            to: 1,
            kind: "call".into(),
            certainty: certainty.into(),
            count: 1,
            provenance: "fixture".into(),
            evidence: vec![],
        }],
        entry_points: Some(EntryPoints {
            framework_entries: vec!["entry".into()],
            ..EntryPoints::default()
        }),
        architecture: Some(ArchitectureFacts {
            version: FACTS_VERSION,
            files: vec![file],
        }),
        flow_inventory: Some(FlowInventory {
            version: 1,
            functions,
        }),
        ..GraphData::default()
    }
}

#[test]
fn offset_only_flow_is_unchanged_but_class_declaration_is_listed() {
    let before = "class OrderService {\nint limit = 10;\nvoid Entry() { Run(); }\nvoid Run() { return; }\n}\n";
    let after = "class OrderService {\nint limit = 200;\nvoid Entry() { Run(); }\nvoid Run() { return; }\n}\n";
    let diff =
        Comparison::from_graphs(snapshot(before, "direct"), snapshot(after, "direct")).unwrap();
    assert_eq!(diff.changes.len(), 1);
    assert_eq!(diff.changes[0].kind, ChangeKind::Class);
    assert_eq!(diff.changes[0].changes_in, [Facet::Declaration]);
    assert_eq!(
        diff.changes[0].entry_points[0].before,
        Some(Certainty::Direct)
    );
    let source = diff.source_diff(&diff.changes[0].id).unwrap();
    assert_eq!(source.hunks.len(), 1);
    assert!(
        source.hunks[0]
            .lines
            .iter()
            .any(|line| line.kind == LineKind::Insert && line.text.contains("200"))
    );
}

#[test]
fn ambiguous_declarations_produce_a_file_preview() {
    let before = "class OrderService {\nint limit = 10;\nvoid Entry() { Run(); }\nvoid Run() { return; }\n}\nclass Other {}\n";
    let after = before.replace("limit = 10", "limit = 20");
    let multi = |source: &str| {
        let mut graph = snapshot(source, "direct");
        graph.architecture.as_mut().unwrap().files[0]
            .names
            .push("Example.Other".into());
        graph
    };
    let diff = Comparison::from_graphs(multi(before), multi(&after)).unwrap();
    assert_eq!(diff.changes.len(), 1);
    assert_eq!(diff.changes[0].kind, ChangeKind::File);
    assert!(diff.changes[0].entry_points.is_empty());
    assert_eq!(
        diff.source_diff(&diff.changes[0].id).unwrap().hunks[0]
            .before
            .start,
        1
    );
}

#[test]
fn call_certainty_is_bound_to_each_snapshot() {
    let source = "class OrderService {\nint limit = 10;\nvoid Entry() { Run(); }\nvoid Run() { return; }\n}\n";
    let before = snapshot(source, "possible");
    let after = snapshot(source, "direct");
    let diff = Comparison::from_graphs(before, after).unwrap();
    let changed = diff
        .changes
        .iter()
        .find(|row| row.kind == ChangeKind::Method)
        .unwrap();
    assert_eq!(changed.name, "entry()");
    assert_eq!(changed.changes_in, [Facet::Flow]);
    let class = diff
        .changes
        .iter()
        .find(|row| row.kind == ChangeKind::Class)
        .unwrap();
    assert_eq!(class.id, changed.class_id.clone().unwrap());
    assert_eq!(class.entry_points[0].before, Some(Certainty::Direct));
    assert_eq!(class.entry_points[0].after, Some(Certainty::Direct));

    let before = snapshot(source, "possible");
    let after = snapshot(&source.replace("return;", "return 42;"), "direct");
    let mut before = before;
    let mut after = after;
    for graph in [&mut before, &mut after] {
        let flow = &mut graph.flow_inventory.as_mut().unwrap().functions[1];
        flow.span.end = graph.architecture.as_ref().unwrap().files[0]
            .content
            .rfind(';')
            .unwrap()
            + 1;
    }
    let diff = Comparison::from_graphs(before, after).unwrap();
    let changed = diff.changes.iter().find(|row| row.name == "run()").unwrap();
    assert_eq!(changed.entry_points[0].before, Some(Certainty::Possible));
    assert_eq!(changed.entry_points[0].after, Some(Certainty::Direct));
}

#[test]
fn unified_hunks_preserve_crlf_and_missing_final_newline() {
    let before = "alpha\r\nbeta\r\nend";
    let after = "alpha\r\nupdated\r\nend";
    let patch = source::patch(before, after);
    let hunk = &patch.hunks()[0];
    assert_eq!(hunk.old_range().start(), 1);
    assert_eq!(hunk.new_range().start(), 1);
    assert_eq!(source::changed_lines(&patch, true).collect::<Vec<_>>(), [2]);
    assert_eq!(
        source::changed_lines(&patch, false).collect::<Vec<_>>(),
        [2]
    );
    assert!(patch.to_string().contains("\\ No newline at end of file"));
    assert_eq!(hunk.lines()[1], diffy::Line::Delete("beta\r\n"));
    assert_eq!(hunk.lines()[2], diffy::Line::Insert("updated\r\n"));
}

#[test]
fn typescript_methods_use_saved_semantic_class_owners() {
    use serde_json::json;

    let node = |class: &str| {
        let base = json!(["<module>", "ClassDeclaration", class, ""]).to_string();
        let owner = json!([base, 0]).to_string();
        let base = json!([owner, "MethodDeclaration", "run", "()"]).to_string();
        let method = json!([base, 0]).to_string();
        Node {
            id: json!(["typescript", "@repo/tsconfig.json", "@repo/main.ts", method]).to_string(),
            language: "typescript".into(),
            project: "App".into(),
            build: "@repo/tsconfig.json".into(),
            file: "@repo/main.ts".into(),
            container: "main.ts".into(),
            label: "run".into(),
            signature: "()".into(),
            kind: "function".into(),
            ..Node::default()
        }
    };
    let a = node("First");
    let b = node("Second");
    let first = Class::from_node(&a).unwrap();
    let second = Class::from_node(&b).unwrap();
    assert_ne!(first.id(), second.id());
    assert_eq!(first.name(), "First");
    assert_eq!(second.name(), "Second");
    let historical = Node {
        id: "historical".into(),
        ..a.clone()
    };
    assert!(Class::from_node(&historical).is_none());

    let mut modified = a.clone();
    modified.body_unavailable = true;
    let before = GraphData {
        nodes: vec![a, b.clone()],
        ..GraphData::default()
    };
    let after = GraphData {
        nodes: vec![modified, b],
        ..GraphData::default()
    };
    let diff = Comparison::from_graphs(before, after).unwrap();
    assert_eq!(diff.changes.len(), 2);
    assert_eq!(diff.changes[0].name, "First");
    assert_eq!(
        diff.changes[1].class_id.as_deref(),
        Some(diff.changes[0].id.as_str())
    );
}
