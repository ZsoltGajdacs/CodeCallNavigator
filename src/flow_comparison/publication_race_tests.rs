//! Reproduce an identity message and settled catalog arriving after the first poll drain.
use crate::flow_comparison::{
    Phase,
    identity::Installation,
    requests::{CatalogAnswer, CatalogOutput},
    tests::{project, settle, state},
};
use std::sync::atomic::AtomicBool;
#[test]
fn a_completed_catalog_reply_reconciles_identity_before_publication() {
    for success in [true, false] {
        let root = tempfile::tempdir().unwrap();
        project(root.path());
        let mut state = state(root.path());
        state
            .open_flow_comparison("refs/tags/base".into(), "refs/tags/head".into())
            .unwrap();
        settle(&mut state);
        let workspace = &mut state.flow_comparison;
        let context = workspace.project.as_ref().unwrap().context.clone();
        let answer = context
            .flow_comparison()
            .open("refs/tags/base", "refs/tags/head", &AtomicBool::new(false))
            .unwrap();
        workspace.binding = None;
        workspace.catalog = None;
        workspace.phase = Phase::Loading;
        workspace.identity.reset();
        let other = tempfile::tempdir().unwrap();
        let canonical_other = other.path().canonicalize().unwrap();
        assert!(matches!(
            workspace.identity.installed(&canonical_other),
            Installation::Pending
        ));
        let serial = workspace.catalog_clock.serial;
        assert!(
            workspace.identity.poll(serial).is_none(),
            "First poll drain sees no identity yet"
        );
        workspace
            .identity
            .reporter(serial, workspace.workers.clone())
            .resolve(context)
            .unwrap();
        let result = if success {
            Ok(CatalogAnswer::Opened(Box::new(
                crate::flow_comparison::requests::Opened {
                    bound: answer.0,
                    page: answer.1,
                },
            )))
        } else {
            drop(answer);
            Err(crate::engine_client::flow_comparison::Error::Invalid(
                anyhow::anyhow!("late catalog failure"),
            ))
        };
        workspace.accept_catalog(CatalogOutput {
            serial,
            physical_project: Some(root.path().canonicalize().unwrap()),
            result,
        });
        assert!(
            workspace.catalog.is_none(),
            "A late Opened result must not bypass the pending project comparison"
        );
        assert!(
            matches!(workspace.phase, Phase::Closed),
            "A completed catalog outcome must reconcile the pending project; success={success}, phase={:?}",
            workspace.phase
        );
    }
}
