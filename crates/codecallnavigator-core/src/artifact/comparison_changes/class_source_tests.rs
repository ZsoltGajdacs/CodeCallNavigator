use super::*;
use crate::{
    architecture::{ArchitectureFacts, FACTS_VERSION, FileFacts},
    flow::{FlowInventory, FunctionFlowIR, SourceSpan},
    model::{Edge, GraphData, Node},
};

fn graph(content: &str, classes: &[&str]) -> GraphData {
    GraphData {
        nodes: classes
            .iter()
            .map(|class| Node {
                id: format!("method-{class}"),
                container: (*class).into(),
                file: "@repo/classes.cs".into(),
                language: "csharp".into(),
                ..Node::default()
            })
            .collect(),
        architecture: Some(ArchitectureFacts {
            version: FACTS_VERSION,
            files: vec![FileFacts {
                file: "@repo/classes.cs".into(),
                language: "csharp".into(),
                names: classes.iter().map(|class| (*class).into()).collect(),
                kinds: vec!["class".into()],
                content: content.into(),
                ..FileFacts::default()
            }],
        }),
        ..GraphData::default()
    }
}

#[test]
fn changed_class_preview_respects_saved_file_ownership() {
    let old = graph("class A { int N = 1; }\n", &["A"]);
    let new = graph("class A { int N = 2; }\n", &["A"]);
    let empty = BTreeSet::new();
    let id = Class::from_node(&old.nodes[0]).unwrap().id();
    let target = ClassSourceTarget {
        root: &id,
        node: &id,
        side: ComparisonSide::After,
        depth: 1,
    };
    let before = Snapshot::new(&old, &empty).unwrap();
    let after = Snapshot::new(&new, &empty).unwrap();
    assert!(matches!(preview(&before, &after, target),
        Some(ClassSource::Diff { hunks, .. }) if hunks[0].lines.iter().any(|line| line.text.contains("N = 2"))));
    assert!(
        matches!(preview(&before, &before, ClassSourceTarget { side: ComparisonSide::Before, ..target }),
        Some(ClassSource::Excerpt { start: 1, text, .. }) if text.contains("N = 1"))
    );
    let shared = graph("class A {}\nclass B {}\n", &["A", "B"]);
    let shared = Snapshot::new(&shared, &empty).unwrap();
    assert!(matches!(
        preview(&before, &shared, target),
        Some(ClassSource::Unavailable { .. })
    ));
}

#[test]
fn shared_file_diff_contains_only_the_selected_members_saved_source() {
    let old = shared_graph(
        "class A { int N() { return 1; } }\nclass B { int N() { return 11; } }\n",
        "return 1;",
    );
    let new = shared_graph(
        "class A { int N() { return 2; } }\nclass B { int N() { return 77; } }\n",
        "return 2;",
    );
    let empty = BTreeSet::new();
    let id = Class::from_node(&old.nodes[0]).unwrap().id();
    let before = Snapshot::new(&old, &empty).unwrap();
    let after = Snapshot::new(&new, &empty).unwrap();
    let Some(ClassSource::Diff { hunks, .. }) = preview(
        &before,
        &after,
        ClassSourceTarget {
            root: &id,
            node: &id,
            side: ComparisonSide::After,
            depth: 1,
        },
    ) else {
        panic!("An exact member span should support a saved diff");
    };
    let lines: Vec<_> = hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .map(|line| line.text.as_str())
        .collect();
    assert_eq!(lines, ["return 1;", "return 2;"]);
}

#[test]
fn expanded_class_source_requires_reachability_in_the_hovered_revision() {
    let mut before = graph("class A {}\nclass B {}\nclass C {}\n", &["A", "B", "C"]);
    before.nodes[2].file = "@repo/third.cs".into();
    before.architecture.as_mut().unwrap().files.push(FileFacts {
        file: "@repo/third.cs".into(),
        language: "csharp".into(),
        names: vec!["C".into()],
        kinds: vec!["class".into()],
        content: "class C {}\n".into(),
        ..FileFacts::default()
    });
    let call = |from, to| Edge {
        from,
        to,
        kind: "call".into(),
        certainty: "direct".into(),
        count: 1,
        provenance: String::new(),
        evidence: Vec::new(),
    };
    before.edges = vec![call(0, 1), call(1, 2)];
    let mut after = before.clone();
    after.edges.pop();
    let empty = BTreeSet::new();
    let [root, node] = [0, 2].map(|index| Class::from_node(&before.nodes[index]).unwrap().id());
    let before = Snapshot::new(&before, &empty).unwrap();
    let after = Snapshot::new(&after, &empty).unwrap();
    let target = ClassSourceTarget {
        root: &root,
        node: &node,
        side: ComparisonSide::Before,
        depth: 1,
    };
    assert!(preview(&before, &after, target).is_none());
    let target = ClassSourceTarget { depth: 2, ..target };
    assert!(matches!(
        preview(&before, &after, target),
        Some(ClassSource::Excerpt { file, text, .. }) if file == "third.cs" && text == "class C {}"
    ));
    assert!(
        preview(
            &before,
            &after,
            ClassSourceTarget {
                side: ComparisonSide::After,
                ..target
            }
        )
        .is_none()
    );
}

fn shared_graph(content: &str, owned: &str) -> GraphData {
    let mut graph = graph(content, &["A", "B"]);
    let node = &graph.nodes[0];
    let start = content.find(owned).unwrap();
    graph.flow_inventory = Some(FlowInventory {
        version: 1,
        functions: vec![FunctionFlowIR {
            symbol: node.id.clone(),
            file: node.file.clone(),
            language: node.language.clone(),
            span: SourceSpan {
                start,
                end: start + owned.len(),
            },
            ..FunctionFlowIR::default()
        }],
    });
    graph
}
