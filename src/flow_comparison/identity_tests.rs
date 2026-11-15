//! Gate worker resolution so installation races are deterministic and do not need UI filesystem reads.
use super::tests::{project, settle, state};
use super::*;
use std::{path::Path, sync::mpsc, time::Duration};
fn gated_open(
    state: &mut crate::app::State,
    root: &Path,
) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
    let runtime = state.runtime.as_ref().unwrap().as_ref().clone();
    let project = Project {
        context: QueryContext::for_project(&runtime, root.into()),
        base: "refs/tags/base".into(),
        head: "refs/tags/head".into(),
        runtime,
    };
    let workspace = &mut state.flow_comparison;
    workspace.project = Some(project.clone());
    workspace.phase = Phase::Loading;
    let cancel = workspace.catalog_clock.restart();
    let serial = workspace.catalog_clock.serial;
    let operation = requests::CatalogOperation::Open {
        project: Box::new(project),
        identity: workspace
            .identity
            .reporter(serial, workspace.workers.clone()),
    };
    let work = workspace.workers.track(requests::CatalogWork {
        serial,
        cancel,
        operation,
    });
    let (started, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    workspace.catalog_job.request(work, move |work| {
        started.send(()).unwrap();
        wait.recv().unwrap();
        work.map(requests::CatalogWork::run)
    });
    (ready, release)
}
#[test]
fn graph_installation_waits_for_worker_identity_during_loading_then_keeps_only_the_same_project() {
    for same in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let canonical = root.path().canonicalize().unwrap();
        project(&canonical);
        let aliases = tempfile::tempdir().unwrap();
        let alias = aliases.path().join("alias");
        std::os::unix::fs::symlink(&canonical, &alias).unwrap();
        let other = tempfile::tempdir().unwrap();
        let installed = if same {
            canonical.clone()
        } else {
            other.path().canonicalize().unwrap()
        };
        let mut state = state(&alias);
        let (started, release) = gated_open(&mut state, &alias);
        started.recv_timeout(Duration::from_secs(2)).unwrap();
        state.flow_comparison.project_installed(Some(&installed));
        assert!(state.flow_comparison.identity.captured().is_none());
        assert!(matches!(state.flow_comparison.phase, Phase::Loading));
        release.send(()).unwrap();
        settle(&mut state);
        if same {
            assert!(state.flow_comparison.catalog.is_some());
            assert!(matches!(state.flow_comparison.phase, Phase::Bound));
        } else {
            assert!(state.flow_comparison.catalog.is_none());
            assert!(matches!(state.flow_comparison.phase, Phase::Closed));
        }
    }
}
