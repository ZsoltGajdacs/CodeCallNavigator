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
        "comparison-close",
        "comparison-snapshot-base-find",
        "comparison-snapshot-base-prepare",
        "comparison-snapshot-after-find",
        "comparison-snapshot-after-prepare",
    ] {
        assert!(runtime.find(key).is_some(), "Missing control {key}");
    }
    for key in [
        "comparison-load",
        "comparison-reload",
        "comparison-snapshots",
        "comparison-graph-differences",
        "comparison-impact-preview",
        "comparison-catalog-open",
    ] {
        assert!(runtime.find(key).is_none(), "Retired control {key}");
    }
    assert_keyboard_picker(&mut runtime);
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
    for _ in 0..8 {
        runtime.key(rune::KeyEvent {
            key: rune::Key::Tab,
            mods: Default::default(),
        });
        runtime.frame(1.0 / 60.0);
        if runtime
            .elements()
            .iter()
            .any(|element| element.key.as_deref() == Some("comparison-base") && element.focused)
        {
            break;
        }
    }
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

#[test]
fn find_worktree_without_a_snapshot_rereads_git_changes() {
    use crate::engine_client::flow_comparison::ChangeKind;

    let (mut runtime, project) = project_runtime(true);
    std::fs::write(
        project.path().join(".git/info/exclude"),
        ".fixture-library/\nuse-isolated-cache\n",
    )
    .unwrap();
    std::fs::write(project.path().join("first.ts"), "let first = 1;\n").unwrap();
    click(&mut runtime, "compare-flows");
    settle(&mut runtime);
    assert_eq!(
        runtime.state.flow_comparison.references().unwrap().1,
        "WORKTREE"
    );
    assert!(runtime.state.flow_comparison.git_outline());

    std::fs::write(project.path().join("second.ts"), "let second = 2;\n").unwrap();
    click(&mut runtime, "comparison-snapshot-after-find");
    settle(&mut runtime);
    let changes = &runtime.state.flow_comparison.changes.rows;
    for file in ["first.ts", "second.ts"] {
        assert!(
            changes
                .iter()
                .any(|change| change.kind == ChangeKind::File && change.file == file)
        );
    }
    let change = changes
        .iter()
        .find(|change| change.file == "second.ts")
        .unwrap();
    let key = runtime
        .elements()
        .into_iter()
        .find_map(|element| {
            element
                .key
                .filter(|key| key.starts_with("comparison-change-") && key.ends_with(&change.id))
        })
        .unwrap();
    click(&mut runtime, &key);
    settle(&mut runtime);
    assert!(runtime.elements().iter().any(|el| {
        el.text
            .as_deref()
            .is_some_and(|text| text.contains("let second = 2;"))
    }));
    runtime.key(rune::KeyEvent {
        key: rune::Key::Escape,
        mods: Default::default(),
    });
    runtime.frame(1.0 / 60.0);
    click(&mut runtime, "comparison-changes-toggle");
    assert!(runtime.find("comparison-change-rows").is_none());
    click(&mut runtime, "comparison-changes-toggle");
    click(&mut runtime, "comparison-view-source-diff");
    settle(&mut runtime);
    assert!(runtime.elements().iter().any(|el| {
        el.text
            .as_deref()
            .is_some_and(|text| text.contains("let second = 2;"))
    }));
    assert!(runtime.state.graph.is_none());
}

#[test]
fn selecting_revisions_rebinds_and_opening_a_picker_does_not_retarget() {
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
    let pinned = runtime
        .state
        .flow_comparison
        .header()
        .unwrap()
        .comparison_id
        .clone();
    click(&mut runtime, "comparison-head");
    click(&mut runtime, "comparison-revisions-done");
    assert_eq!(
        runtime
            .state
            .flow_comparison
            .header()
            .unwrap()
            .comparison_id,
        pinned
    );
    choose_revision(&mut runtime, SnapshotSide::Head, "worktree");
    assert_eq!(
        runtime.state.flow_comparison.references(),
        Some((commit.as_str(), "WORKTREE"))
    );
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
