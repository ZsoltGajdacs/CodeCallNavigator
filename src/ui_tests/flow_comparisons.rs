//! Comparison controls exercise the production Rune view without requiring a current graph.
use super::click;
use crate::{app::State, flow_comparison::SnapshotSide, jobs, view};
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
        "comparison-snapshots",
        "comparison-graph-differences",
        "comparison-impact-preview",
    ] {
        assert!(
            runtime.find(key).is_some(),
            "Missing comparison control {key}"
        );
    }
    assert_keyboard_picker(&mut runtime);
    click(&mut runtime, "comparison-snapshots");
    for key in [
        "comparison-snapshot-base-lookup",
        "comparison-snapshot-base-prepare",
        "comparison-snapshot-after-lookup",
        "comparison-snapshot-after-prepare",
        "comparison-snapshot-select-builds",
    ] {
        assert!(
            runtime.find(key).is_some(),
            "Missing snapshot control {key}"
        );
    }
    assert!(runtime.find("comparison-snapshot-target").is_none());
    let message = runtime.state.flow_comparison.snapshots.message.clone();
    click(&mut runtime, "comparison-snapshot-base-prepare");
    assert_eq!(runtime.state.flow_comparison.snapshots.message, message);
    click(&mut runtime, "comparison-close");
    click(&mut runtime, "project-picker-trigger");
    assert_eq!(
        runtime
            .elements()
            .iter()
            .filter(|element| element.key.as_deref() == Some("compare-flows"))
            .count(),
        1
    );
    assert!(runtime.find("choose-folder").is_some());
}

fn assert_keyboard_picker(runtime: &mut Runtime<State>) {
    runtime.set_full_keyboard_access(false);
    runtime.key(rune::KeyEvent {
        key: rune::Key::Tab,
        mods: Default::default(),
    });
    runtime.frame(1.0 / 60.0);
    assert!(
        runtime.elements().iter().any(|element| {
            element.key.as_deref() == Some("comparison-base") && element.focused
        })
    );
    runtime.key(rune::KeyEvent {
        key: rune::Key::Down,
        mods: Default::default(),
    });
    runtime.frame(1.0 / 60.0);
    assert!(runtime.find("comparison-revisions").is_some());
    runtime.key(rune::KeyEvent {
        key: rune::Key::Escape,
        mods: Default::default(),
    });
    runtime.frame(1.0 / 60.0);
}

pub(super) fn project_runtime(dark: bool) -> (Runtime<State>, tempfile::TempDir) {
    let project = tempfile::tempdir().unwrap();
    crate::flow_comparison::tests::project(project.path());
    let mut state = State::new();
    state.root = rune::TextEdit::new(project.path().to_string_lossy());
    state.project_open.folder = Some(state.root.text.clone());
    state.runtime = Some(std::sync::Arc::new(
        crate::flow_comparison::tests::fixture_runtime(project.path()),
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
        "comparison-search" => &runtime.state.flow_comparison.controls.search.text,
        _ => return,
    };
    assert_eq!(actual, value, "Text input {key} did not accept editing");
}
pub(super) fn settle(runtime: &mut Runtime<State>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        runtime.frame(1.0 / 60.0);
        if !runtime.state.flow_comparison.pending() && runtime.state.flow_comparison.stopped() {
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
    let base = fixture_commit(runtime, SnapshotSide::Base);
    let head = fixture_commit(runtime, SnapshotSide::Head);
    choose_revision(runtime, SnapshotSide::Base, &base);
    choose_revision(runtime, SnapshotSide::Head, &head);
    click(runtime, "comparison-load");
    settle(runtime);
    if runtime.state.flow_comparison.catalog.is_some() {
        open_catalog(runtime);
    }
}
fn fixture_commit(runtime: &Runtime<State>, side: SnapshotSide) -> String {
    let reference = match side {
        SnapshotSide::Base => "refs/tags/base",
        SnapshotSide::Head => "refs/tags/head",
    };
    codecallnavigator::artifact::revision::Repository::open(std::path::Path::new(
        runtime.state.project_open.folder.as_deref().unwrap(),
    ))
    .unwrap()
    .resolve(reference)
    .unwrap()
}

pub(super) fn choose_revision(runtime: &mut Runtime<State>, side: SnapshotSide, commit: &str) {
    if runtime.state.flow_comparison.controls.catalog_open {
        click(runtime, "comparison-catalog-close");
    }
    let key = match side {
        SnapshotSide::Base => "comparison-base",
        SnapshotSide::Head => "comparison-head",
    };
    click(runtime, key);
    settle(runtime);
    click(runtime, &format!("comparison-revision-{commit}"));
    settle(runtime);
    if commit != "worktree" {
        let message = runtime
            .state
            .flow_comparison
            .revisions
            .message
            .as_deref()
            .unwrap();
        assert!(!message.trim().is_empty());
    }
    click(runtime, "comparison-revisions-done");
}

pub(super) fn open_catalog(runtime: &mut Runtime<State>) {
    if !runtime.state.flow_comparison.controls.catalog_open {
        click(runtime, "comparison-catalog-open");
    }
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
    let commit = fixture_commit(&runtime, SnapshotSide::Base);
    assert_eq!(
        runtime.state.flow_comparison.references(),
        Some((commit.as_str(), commit.as_str()))
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
    let commit = fixture_commit(&runtime, SnapshotSide::Base);
    assert_eq!(
        runtime.state.flow_comparison.references(),
        Some((commit.as_str(), commit.as_str()))
    );
    assert_eq!(
        runtime.state.flow_comparison.descriptor().unwrap().project,
        project.path()
    );
    assert!(runtime.state.flow_comparison.catalog.is_some());
    choose_revision(&mut runtime, SnapshotSide::Head, "worktree");
    assert_eq!(
        runtime.state.flow_comparison.references(),
        Some((commit.as_str(), commit.as_str()))
    );
    click(&mut runtime, "comparison-reload");
    settle(&mut runtime);
    assert_eq!(
        runtime.state.flow_comparison.references(),
        Some((commit.as_str(), commit.as_str()))
    );
    assert!(runtime.state.flow_comparison.catalog.is_some());
    assert!(runtime.state.graph.is_none());
    click(&mut runtime, "comparison-close");
    runtime.update(|state, _| {
        state.show_flow_comparison();
        crate::graph_install::clear(state);
    });
    settle(&mut runtime);
    assert!(runtime.state.flow_comparison.revisions.rows.is_empty());
    assert!(!runtime.state.flow_comparison.controls.visible);
}
