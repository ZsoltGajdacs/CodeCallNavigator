//! Saved Git fixtures exercise the public CLI without compiling current sources.
use crate::{app::State, jobs, view};
use codecallnavigator::{artifact, model::GraphData};
use rune::{Env, Runtime};
use std::{fs, path::Path};
pub(super) fn commit(root: &Path, graph: &GraphData, name: &str) {
    let saved = artifact::canonical(graph, &[("repo".into(), "/snapshot".into())]).unwrap();
    fs::write(root.join(artifact::GRAPH_PATH), saved.bytes().unwrap()).unwrap();
    crate::flow_comparison::tests::git(root, &["add", artifact::GRAPH_PATH]);
    crate::flow_comparison::tests::git(root, &["commit", "--allow-empty", "-qm", name]);
    crate::flow_comparison::tests::git(root, &["tag", name]);
}
pub(super) fn runtime(
    before: GraphData,
    after: GraphData,
    dark: bool,
) -> (Runtime<State>, tempfile::TempDir) {
    let project = tempfile::tempdir().unwrap();
    let root = project.path();
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "Comparison UI"],
        &["config", "user.email", "test@example.invalid"],
    ] {
        crate::flow_comparison::tests::git(root, args);
    }
    fs::create_dir(root.join(".codecallnavigator")).unwrap();
    commit(root, &before, "base");
    commit(root, &after, "head");
    fs::remove_file(root.join(artifact::GRAPH_PATH)).unwrap();
    let mut state = State::new();
    state.root = rune::TextEdit::new(root.to_string_lossy());
    state.runtime = Some(std::sync::Arc::new(
        crate::runtime::Runtime::discover().unwrap(),
    ));
    let mut runtime = Runtime::new(state, view::view, (1050.0, 680.0), Env::new(dark, 1.0));
    runtime.poll = Some(jobs::poll);
    runtime.on_key = Some(view::key);
    runtime.frame(0.0);
    (runtime, project)
}
fn shift(value: &mut serde_json::Value, delta: usize) {
    match value {
        serde_json::Value::Array(items) => {
            for item in items {
                shift(item, delta);
            }
        }
        serde_json::Value::Object(fields) => {
            shift_span(fields, delta);
            for item in fields.values_mut() {
                shift(item, delta);
            }
        }
        _ => {}
    }
}
fn shift_span(fields: &mut serde_json::Map<String, serde_json::Value>, delta: usize) {
    let (Some(start), Some(end)) = (
        fields.get("start").and_then(|v| v.as_u64()),
        fields.get("end").and_then(|v| v.as_u64()),
    ) else {
        return;
    };
    if start == 0 && end == 0 {
        return;
    }
    fields.insert("start".into(), (start + delta as u64).into());
    fields.insert("end".into(), (end + delta as u64).into());
}

pub(super) fn prefixed(name: &str, lines: usize) -> GraphData {
    let mut graph = crate::flow_scenes::data();
    let prefix = format!("// {name} café 🧭\r\n").repeat(lines);
    graph.architecture.as_mut().unwrap().files[0]
        .content
        .insert_str(0, &prefix);
    let mut inventory = serde_json::to_value(graph.flow_inventory.take().unwrap()).unwrap();
    shift(&mut inventory, prefix.len());
    graph.flow_inventory = Some(serde_json::from_value(inventory).unwrap());
    for node in &mut graph.nodes {
        node.line += lines as u32;
    }
    graph
}
pub(super) fn choose(runtime: &mut Runtime<State>, label: &str) -> String {
    let symbol=runtime.state.flow_comparison.catalog.as_ref().unwrap().records.iter().find(|record| {
        [&record.before,&record.after].into_iter().any(|side|matches!(side,crate::engine_client::flow_comparison::CatalogSide::Present(side) if side.node.as_ref().is_some_and(|node|node.label.starts_with(label))))
    }).unwrap_or_else(||panic!("No record for {label}: {:?}",runtime.state.flow_comparison.catalog)).symbol.clone();
    super::click(runtime, &format!("comparison-record-{symbol}"));
    super::flow_comparisons::settle(runtime);
    symbol
}
