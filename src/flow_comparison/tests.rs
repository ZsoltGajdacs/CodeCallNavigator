use super::*;
use crate::{app::State, runtime::Runtime};
use codecallnavigator::artifact;
use std::{
    fs,
    path::Path,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
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
pub(crate) fn project(root: &Path) {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.name", "Workspace tests"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    fs::create_dir(root.join(".codecallnavigator")).unwrap();
    let graph = crate::flow_scenes::data();
    let saved = artifact::canonical(&graph, &[("repo".into(), "/snapshot".into())]).unwrap();
    fs::write(root.join(artifact::GRAPH_PATH), saved.bytes().unwrap()).unwrap();
    git(root, &["add", artifact::GRAPH_PATH]);
    git(root, &["commit", "-qm", "base"]);
    git(root, &["tag", "base"]);
    git(root, &["tag", "head"]);
    fs::remove_file(root.join(artifact::GRAPH_PATH)).unwrap();
}
pub(crate) fn state(root: &Path) -> State {
    let mut state = State::new();
    state.root = rune::TextEdit::new(root.to_string_lossy());
    state.runtime = Some(Arc::new(Runtime::discover().unwrap()));
    state
}
pub(crate) fn settle(state: &mut State) {
    let start = Instant::now();
    while state.flow_comparison.pending() {
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "Comparison job timed out"
        );
        state.flow_comparison.poll();
        std::thread::sleep(Duration::from_millis(10));
    }
    state.flow_comparison.poll();
}
pub(crate) fn selected(state: &State) -> String {
    state.flow_comparison.catalog.as_ref().unwrap().records.iter().find(|record| {
        matches!(&record.before,crate::engine_client::flow_comparison::CatalogSide::Present(side) if side.node.as_ref().is_some_and(|node|node.label.starts_with("load(")))
    }).unwrap().symbol.clone()
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
    state.flow_comparison.page(Default::default()).unwrap();
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
                physical_project: None,
                result: Err(crate::engine_client::flow_comparison::Error::Invalid(
                    anyhow::anyhow!("late failure"),
                )),
            }
        })
    });
}
