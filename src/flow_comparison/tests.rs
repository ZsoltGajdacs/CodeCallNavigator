use super::*;
use crate::{app::State, runtime::Runtime};
use codecallnavigator::artifact::{
    self, Artifact,
    local_snapshots::{Retention, SnapshotIdentity, SnapshotRecord, Store},
    revision::Repository,
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};

pub(crate) fn isolate_cli_cache(runtime: &mut Runtime, cache: &Path) {
    let wrapper = cache.with_file_name("use-isolated-cache");
    let script = format!(
        "#!/bin/sh\nexport CODECALLNAVIGATOR_GRAPH_LIBRARY={}\nexec {} \"$@\"\n",
        shell_path(cache),
        shell_path(&runtime.engine),
    );
    fs::write(&wrapper, script).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    runtime.engine = wrapper;
}

fn shell_path(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

pub(crate) fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
fn init_git(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "Workspace tests"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
}

pub(crate) fn project(root: &Path) {
    init_git(root);
    git(root, &["commit", "--allow-empty", "-qm", "base"]);
    git(root, &["tag", "base"]);
    git(root, &["tag", "head"]);
    retain_scene(root, &crate::flow_scenes::data(), "HEAD");
}

pub(crate) fn fixture_store(root: &Path) -> Store {
    // Fixture engine and library paths model fixed user-level configuration.
    // A mutable project alias must not redirect either path.
    Store::at(root.canonicalize().unwrap().join(".fixture-library"))
}

pub(crate) fn fixture_runtime(root: &Path) -> Runtime {
    let mut runtime = Runtime::discover().unwrap();
    let store = fixture_store(root);
    if store.root().exists() {
        isolate_cli_cache(&mut runtime, store.root());
    }
    runtime
}

pub(crate) fn retain_scene(
    root: &Path,
    graph: &codecallnavigator::model::GraphData,
    revision: &str,
) -> SnapshotRecord {
    let saved = artifact::canonical(graph, &[("repo".into(), "/snapshot".into())]).unwrap();
    retain_fixture(root, &saved, revision)
}

/// Seed already-saved graph fixtures. Compiler-backed tests exercise real retention.
pub(crate) fn retain_fixture(root: &Path, saved: &Artifact, revision: &str) -> SnapshotRecord {
    let repository = Repository::open(root).unwrap();
    let commit = repository.resolve(revision).unwrap();
    fixture_store(root)
        .retain(
            &repository.common_dir,
            Retention {
                identity: SnapshotIdentity {
                    commit,
                    project: repository.project,
                    analyzer: "desktop-flow-fixture".into(),
                    selection: "{}".into(),
                },
                encoded: &saved.encode().unwrap(),
            },
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap()
}
pub(crate) fn state(root: &Path) -> State {
    let mut state = State::new();
    state.root = rune::TextEdit::new(root.to_string_lossy());
    state.project_open.folder = Some(state.root.text.clone());
    state.runtime = Some(Arc::new(fixture_runtime(root)));
    state
}
pub(crate) fn settle(state: &mut State) {
    let start = Instant::now();
    loop {
        state.flow_comparison.poll();
        // A completion can be ready to publish before it starts the next worker.
        if !state.flow_comparison.pending() && state.flow_comparison.stopped() {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "Comparison job timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
pub(crate) fn selected(state: &State) -> String {
    state.flow_comparison.catalog.as_ref().unwrap().records.iter().find(|record| {
        matches!(&record.before,crate::engine_client::flow_comparison::CatalogSide::Present(side) if side.node.as_ref().is_some_and(|node|node.label.starts_with("load(")))
    }).unwrap().symbol.clone()
}

#[test]
fn git_outline_remains_usable_without_graph_and_reloads_after_worktree_edit() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    init_git(root);
    let source = root.join("code.ts");
    fs::write(&source, "const value = 1;\n").unwrap();
    git(root, &["add", "code.ts"]);
    git(root, &["commit", "-qm", "base"]);
    fs::write(
        root.join(".git/info/exclude"),
        "/.fixture-library/\n/use-isolated-cache\n",
    )
    .unwrap();
    fs::create_dir(fixture_store(root).root()).unwrap();
    fs::write(&source, "const value = 2;\n").unwrap();
    let mut state = state(root);
    state
        .open_flow_comparison("HEAD".into(), "WORKTREE".into())
        .unwrap();
    settle(&mut state);

    let comparison = &mut state.flow_comparison;
    assert!(comparison.git_outline());
    assert!(!comparison.outline_graph_ready());
    assert_eq!(comparison.changes.rows.len(), 1);
    let id = comparison.changes.rows[0].id.clone();
    comparison.choose_change_source(id.clone(), "code row".into());
    settle(&mut state);
    assert_preview(&state.flow_comparison, &id, "const value = 2;\n");

    fs::write(&source, "const value = 3;\n").unwrap();
    state
        .flow_comparison
        .choose_change_source(id.clone(), "code row".into());
    settle(&mut state);
    assert!(state.flow_comparison.changes.reload_required);
    assert!(state.flow_comparison.previews.active.is_none());

    state.flow_comparison.reload().unwrap();
    settle(&mut state);
    let id = state.flow_comparison.changes.rows[0].id.clone();
    state
        .flow_comparison
        .choose_change_source(id.clone(), "code row".into());
    settle(&mut state);
    assert_preview(&state.flow_comparison, &id, "const value = 3;\n");
    assert!(state.graph.is_none());
}

#[test]
fn stale_worktree_class_rows_open_the_bound_saved_diagram() {
    use crate::engine_client::flow_comparison::ChangeKind;
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    init_git(&root);
    save_class_fixture_and_edit_worktree(&root);

    let mut state = state(&root);
    state
        .open_flow_comparison("HEAD".into(), "WORKTREE".into())
        .unwrap();
    let comparison = &mut state.flow_comparison;
    let start = Instant::now();
    while !comparison.changes.loaded {
        comparison.publish_changes();
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "Git outline job timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(comparison.class_pair().is_none());
    let class = comparison
        .changes
        .rows
        .iter()
        .find(|row| row.kind == ChangeKind::Class)
        .unwrap();
    assert!(
        !class.flow_available,
        "The Worktree source must differ from the saved graph"
    );
    let class_id = class.id.clone();
    let file_id = comparison
        .changes
        .rows
        .iter()
        .find(|row| row.kind == ChangeKind::File && row.file == class.file)
        .unwrap()
        .id
        .clone();
    let mut ui = click_sidebar_row(state, &file_id);
    assert!(ui.state.flow_comparison.previews.active.is_none());
    settle(&mut ui.state);

    let comparison = &ui.state.flow_comparison;
    let pair = comparison.class_pair().unwrap_or_else(|| {
        panic!(
            "Class activation must show the saved diagram. Outline: {}. Diagram: {}",
            comparison.changes.message, comparison.detail_message,
        )
    });
    assert_eq!(pair.graph.root, class_id);
    assert!(pair.has_root(SideId::Before));
    assert!(pair.has_root(SideId::After));
    assert!(comparison.previews.active.is_none());
    assert!(comparison.changes.message.is_empty());
    assert!(matches!(
        comparison.worktree_snapshot(),
        Some(super::WorktreeSnapshot::Saved(Some(
            codecallnavigator::artifact::verification::InputStatus::Stale
        )))
    ));
}

fn click_sidebar_row(state: State, id: &str) -> rune::Runtime<State> {
    let mut ui = rune::Runtime::new(
        state,
        crate::view::view,
        (1050.0, 680.0),
        rune::Env::new(true, 1.0),
    );
    ui.frame(1.0 / 60.0);
    let (x, y) = ui.find(&format!("comparison-change-row-{id}")).unwrap();
    ui.pointer_down(x, y);
    ui.pointer_up(x, y);
    ui.frame(1.0 / 60.0);
    ui
}

// Reuse the retained flow fixture with class owner keys so this tests the
// desktop and real query boundary independently of compiler setup.
fn save_class_fixture_and_edit_worktree(root: &Path) {
    use codecallnavigator::{artifact::provenance, fingerprint};
    use std::sync::atomic::AtomicBool;

    let source = root.join("flow.ts");
    let graph = crate::flow_scenes::data();
    let facts = &graph.architecture.as_ref().unwrap().files[0];
    fs::write(&source, &facts.content).unwrap();
    git(root, &["add", "flow.ts"]);
    git(root, &["commit", "-qm", "base"]);

    let owner = serde_json::json!([
        serde_json::json!(["", "ClassDeclaration", "SavedFixture", ""]).to_string(),
        0
    ])
    .to_string();
    let keys = graph
        .nodes
        .iter()
        .map(|node| {
            let key = serde_json::json!([
                serde_json::json!([owner, "MethodDeclaration", node.label, node.signature])
                    .to_string(),
                0
            ]);
            (node.id.clone(), key.to_string())
        })
        .collect();
    let mappings = [("repo".into(), Path::new("/snapshot").to_owned())];
    let mut saved = artifact::canonical_with_identities(&graph, &mappings, &keys).unwrap();
    let inputs = fingerprint::capture([source.clone()]).unwrap();
    let live_roots = [("repo".into(), root.to_owned())];
    saved.graph["analysis"] =
        serde_json::to_value(provenance::build(&inputs, &[], "{}", &live_roots).unwrap()).unwrap();
    retain_fixture(root, &saved, "HEAD");
    artifact::AnalysisLock::acquire_in(root, fixture_store(root))
        .unwrap()
        .publish(&saved, &AtomicBool::new(false))
        .unwrap();
    fs::write(
        root.join(".git/info/exclude"),
        "/.fixture-library/\n/use-isolated-cache\n",
    )
    .unwrap();
    fs::write(&source, facts.content.replace("return 0;", "return 42;")).unwrap();
}

fn assert_preview(comparison: &Workspace, id: &str, expected: &str) {
    let preview = comparison.previews.diff(id).unwrap().unwrap();
    assert_eq!(preview.file, "code.ts");
    assert!(
        preview
            .hunks
            .iter()
            .flat_map(|hunk| &hunk.lines)
            .any(|line| line.text == expected)
    );
}

#[test]
fn historical_workspace_loads_without_graph_and_keeps_captured_project_and_side_state() {
    let root = tempfile::tempdir().unwrap();
    project(root.path());
    let mut state = state(root.path());
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    let symbol = selected(&state);
    assert!(state.graph.is_none());
    state.root = rune::TextEdit::new("/edited-other-project");
    state.root_edited();
    state.epoch += 10;
    state
        .flow_comparison
        .select(Selection {
            symbol: symbol.clone(),
            head_symbol: None,
        })
        .unwrap();
    settle(&mut state);
    let pair = state.flow_comparison.current.as_mut().unwrap();
    let Side::Ready(before) = &mut pair.before else {
        panic!()
    };
    let Side::Ready(after) = &mut pair.after else {
        panic!()
    };
    assert_ne!(before.session.scroll_key, after.session.scroll_key);
    before.session.scroll.set([3.0, 90.0]);
    before.session.camera.pan = [11.0, 22.0];
    assert_ne!(before.session.scroll.get(), after.session.scroll.get());
    let target = before
        .targets
        .iter()
        .find(|node| !node.body_unavailable)
        .unwrap()
        .symbol
        .clone();
    state
        .flow_comparison
        .follow(SideId::Before, &target)
        .unwrap();
    settle(&mut state);
    assert!(state.flow_comparison.back());
    let Side::Ready(before) = &state.flow_comparison.current.as_ref().unwrap().before else {
        panic!()
    };
    assert_eq!(before.session.scroll.get(), [3.0, 90.0]);
    assert_eq!(before.session.camera.pan, [11.0, 22.0]);
    assert_eq!(
        state.flow_comparison.descriptor().unwrap().project,
        root.path()
    );
}
#[test]
fn moved_reference_requires_reload_and_clears_every_actionable_result() {
    let root = tempfile::tempdir().unwrap();
    project(root.path());
    let mut state = state(root.path());
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    let symbol = selected(&state);
    state
        .flow_comparison
        .select(Selection {
            symbol,
            head_symbol: None,
        })
        .unwrap();
    settle(&mut state);
    git(root.path(), &["commit", "--allow-empty", "-qm", "moved"]);
    git(root.path(), &["tag", "-f", "head"]);
    retain_scene(root.path(), &crate::flow_scenes::data(), "HEAD");
    let selection = state
        .flow_comparison
        .current
        .as_ref()
        .unwrap()
        .selection
        .clone();
    state.flow_comparison.select(selection).unwrap();
    settle(&mut state);
    assert!(matches!(
        state.flow_comparison.phase,
        Phase::ReloadRequired(_)
    ));
    assert!(state.flow_comparison.current.is_none());
    assert!(state.flow_comparison.catalog.is_none());
    assert!(state.flow_comparison.history.is_empty());
    state.flow_comparison.reload().unwrap();
    settle(&mut state);
    assert!(state.flow_comparison.catalog.is_some());
}

pub(crate) fn hold_request(
    workspace: &mut Workspace,
    started: std::sync::mpsc::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
) {
    let input = workspace.workers.track(());
    let serial = workspace.catalog_clock.serial;
    workspace.catalog_job.request(input, move |input| {
        input.map(|()| {
            started.send(()).unwrap();
            release.recv().unwrap();
            requests::CatalogOutput {
                serial,
                refresh: None,
                physical_project: None,
                result: Err(crate::engine_client::flow_comparison::Error::Invalid(
                    anyhow::anyhow!("late failure"),
                )),
            }
        })
    });
}
