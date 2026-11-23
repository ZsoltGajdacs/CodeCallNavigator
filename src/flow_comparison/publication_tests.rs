//! Force obsolete completions to land, rather than relying only on Latest dropping them.
use super::tests::{project, settle, state};
use super::{
    requests::{
        CatalogAnswer, CatalogOutput, DetailOutput, DetailSelection, PreparedDetail, StringError,
    },
    *,
};
use crate::engine_client::flow_comparison::{CatalogPage, Error};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, mpsc},
    time::{Duration, Instant},
};
fn landed<T: Send + 'static>(job: &Latest<T>) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while job.is_pending() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn superseded_success_and_error_catalog_outputs_cannot_change_the_current_workspace() {
    let root = tempfile::tempdir().unwrap();
    project(root.path());
    let mut state = state(root.path());
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    for failure in [false, true] {
        let workspace = &mut state.flow_comparison;
        let before = workspace.catalog.clone().unwrap();
        let page = workspace
            .binding
            .as_ref()
            .unwrap()
            .catalog(
                CatalogPage::default(),
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
        let result = if failure {
            Err(Error::Invalid(anyhow::anyhow!("obsolete failure")))
        } else {
            Ok(CatalogAnswer::Page(Box::new(page)))
        };
        let serial = workspace.catalog_clock.serial;
        let input = workspace.workers.track(CatalogOutput {
            serial,
            refresh: None,
            physical_project: Some(root.path().canonicalize().unwrap()),
            result,
        });
        let (release, wait) = mpsc::channel();
        workspace.catalog_job.request(input, move |input| {
            wait.recv().unwrap();
            input
        });
        workspace.catalog_clock.cancel();
        release.send(()).unwrap();
        landed(&workspace.catalog_job);
        assert!(!workspace.poll());
        assert!(Arc::ptr_eq(&before, workspace.catalog.as_ref().unwrap()));
        assert!(matches!(workspace.phase, Phase::Bound));
    }
}
#[test]
fn detail_outputs_need_both_the_current_serial_and_comparison_binding() {
    let root = tempfile::tempdir().unwrap();
    project(root.path());
    let mut state = state(root.path());
    state
        .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
        .unwrap();
    settle(&mut state);
    let symbol = super::tests::selected(&state);
    let dot = state.runtime.as_ref().unwrap().dot.clone();
    enum SupersededBy {
        Serial,
        Binding,
        ClassSelection,
    }
    for (obsolete, success) in [
        (SupersededBy::Serial, false),
        (SupersededBy::Binding, false),
        (SupersededBy::Serial, true),
        (SupersededBy::Binding, true),
        (SupersededBy::ClassSelection, true),
    ] {
        let workspace = &mut state.flow_comparison;
        let serial = workspace.detail_clock.serial;
        let binding = if matches!(obsolete, SupersededBy::Binding) {
            "obsolete".into()
        } else {
            workspace.header().unwrap().comparison_id.clone()
        };
        let selection = DetailSelection::Function(Selection {
            symbol: symbol.clone(),
            head_symbol: None,
        });
        workspace.detail_selection = Some(if matches!(obsolete, SupersededBy::ClassSelection) {
            DetailSelection::Class("newly selected class".into())
        } else {
            selection.clone()
        });
        let result = if success {
            Ok(prepare_function(workspace, &symbol, &dot))
        } else {
            Err(StringError::Preparation("obsolete error".into()))
        };
        let output = DetailOutput {
            serial,
            binding,
            selection,
            result,
        };
        let input = workspace.workers.track(output);
        let (release, wait) = mpsc::channel();
        workspace.detail_job.request(input, move |input| {
            wait.recv().unwrap();
            input
        });
        if matches!(obsolete, SupersededBy::Serial) {
            workspace.detail_clock.cancel();
        }
        release.send(()).unwrap();
        landed(&workspace.detail_job);
        assert!(!workspace.poll());
        assert!(workspace.current.is_none());
        if matches!(obsolete, SupersededBy::ClassSelection) {
            assert_eq!(workspace.class_selection(), Some("newly selected class"));
        }
        assert!(matches!(workspace.phase, Phase::Bound));
    }
}

fn prepare_function(workspace: &Workspace, symbol: &str, dot: &Path) -> PreparedDetail {
    let cancel = AtomicBool::new(false);
    let detail = workspace
        .binding
        .as_ref()
        .unwrap()
        .detail(symbol, None, &cancel)
        .unwrap();
    let prepared = super::pair::Prepared::load(detail, dot, &cancel).unwrap();
    PreparedDetail::Function(Box::new(prepared))
}
