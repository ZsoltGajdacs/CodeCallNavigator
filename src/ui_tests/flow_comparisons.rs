//! Comparison controls exercise the production Rune view without requiring a current graph.
use super::click;
use crate::{app::State, jobs, view};
use rune::{Env, Runtime};
#[test]
fn global_comparison_entry_opens_revision_controls_without_a_loaded_graph() {
    let mut runtime = Runtime::new(
        State::new(),
        view::view,
        (1050.0, 680.0),
        Env::new(true, 1.0),
    );
    runtime.poll = Some(jobs::poll);
    runtime.on_key = Some(view::key);
    runtime.frame(0.0);
    assert!(runtime.state.graph.is_none());
    click(&mut runtime, "compare-flows");
    for key in [
        "comparison-base",
        "comparison-head",
        "comparison-load",
        "comparison-close",
    ] {
        assert!(
            runtime.find(key).is_some(),
            "Missing comparison control {key}"
        );
    }
    click(&mut runtime, "comparison-close");
    click(&mut runtime, "project-picker-trigger");
    assert!(runtime.find("compare-flows").is_some());
}

pub(super) fn project_runtime(dark: bool) -> (Runtime<State>, tempfile::TempDir) {
    let project = tempfile::tempdir().unwrap();
    crate::flow_comparison::tests::project(project.path());
    let mut state = State::new();
    state.root = rune::TextEdit::new(project.path().to_string_lossy());
    state.project_open.folder = Some(state.root.text.clone());
    state.runtime = Some(std::sync::Arc::new(
        crate::runtime::Runtime::discover().unwrap(),
    ));
    let mut runtime = Runtime::new(state, view::view, (1050.0, 680.0), Env::new(dark, 1.0));
    runtime.poll = Some(jobs::poll);
    runtime.on_key = Some(view::key);
    runtime.frame(0.0);
    (runtime, project)
}
pub(super) fn edit(runtime: &mut Runtime<State>, key: &str, value: &str) {
    if runtime.state.flow_comparison.controls.catalog_open && key != "comparison-search" {
        click(runtime, "comparison-catalog-close");
    }
    click(runtime, key);
    runtime.key(rune::KeyEvent {
        key: rune::Key::Command('a'),
        mods: rune::Mods {
            cmd: true,
            logo: true,
            ..Default::default()
        },
    });
    runtime.key(rune::KeyEvent {
        key: rune::Key::Text(value.into()),
        mods: Default::default(),
    });
    runtime.frame(1.0 / 60.0);
    let actual = match key {
        "comparison-base" => &runtime.state.flow_comparison.controls.base.text,
        "comparison-head" => &runtime.state.flow_comparison.controls.head.text,
        "comparison-search" => &runtime.state.flow_comparison.controls.search.text,
        _ => return,
    };
    assert_eq!(actual, value, "Text input {key} did not accept editing");
}
pub(super) fn settle(runtime: &mut Runtime<State>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        runtime.frame(1.0 / 60.0);
        if !runtime.state.flow_comparison.pending() {
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    for _ in 0..5 {
        runtime.frame(1.0 / 60.0);
    }
}
pub(super) fn load(runtime: &mut Runtime<State>) {
    click(runtime, "compare-flows");
    edit(runtime, "comparison-base", "refs/tags/base");
    edit(runtime, "comparison-head", "refs/tags/head");
    click(runtime, "comparison-load");
    settle(runtime);
}
fn symbol(runtime: &Runtime<State>, label: &str) -> String {
    runtime.state.flow_comparison.catalog.as_ref().unwrap().records.iter().find(|record| {
        matches!(&record.before,crate::engine_client::flow_comparison::CatalogSide::Present(present) if present.node.as_ref().is_some_and(|node|node.label.starts_with(label)))
    }).unwrap().symbol.clone()
}
#[test]
fn real_revision_controls_load_catalog_select_a_function_and_show_both_charts() {
    let (mut runtime, _project) = project_runtime(true);
    load(&mut runtime);
    assert_eq!(
        runtime.state.flow_comparison.references(),
        Some(("refs/tags/base", "refs/tags/head"))
    );
    let symbol = symbol(&runtime, "load(");
    click(&mut runtime, &format!("comparison-record-{symbol}"));
    settle(&mut runtime);
    for key in [
        "comparison-before-canvas",
        "comparison-after-canvas",
        "comparison-source-before",
        "comparison-source-after",
    ] {
        assert!(runtime.find(key).is_some(), "Missing comparison pane {key}");
    }
    assert!(runtime.state.graph.is_none());
}

#[test]
fn revision_controls_submit_refs_and_editing_does_not_retarget_the_loaded_pair() {
    let (mut runtime, project) = project_runtime(true);
    load(&mut runtime);
    assert_eq!(
        runtime.state.flow_comparison.references(),
        Some(("refs/tags/base", "refs/tags/head"))
    );
    assert_eq!(
        runtime.state.flow_comparison.descriptor().unwrap().project,
        project.path()
    );
    assert!(runtime.state.flow_comparison.catalog.is_some());
    edit(&mut runtime, "comparison-base", "different-base");
    edit(&mut runtime, "comparison-head", "different-head");
    assert_eq!(
        runtime.state.flow_comparison.references(),
        Some(("refs/tags/base", "refs/tags/head"))
    );
    click(&mut runtime, "comparison-reload");
    settle(&mut runtime);
    assert_eq!(
        runtime.state.flow_comparison.references(),
        Some(("refs/tags/base", "refs/tags/head"))
    );
    assert!(runtime.state.flow_comparison.catalog.is_some());
    assert!(runtime.state.graph.is_none());
}
