//! Exercise typed preparation diagnostics through the Compare view and clipboard.
use super::*;
use codecallnavigator::{
    discovery::BuildIdentity,
    protocol::{AnalysisPhase, TargetFailure},
};
use rune::{Env, Runtime};

#[test]
fn preparation_failure_is_visible_copyable_and_cleared_on_project_change() {
    let mut state = State::new();
    state.flow_comparison.controls.visible = true;
    let snapshots = &mut state.flow_comparison.snapshots;
    snapshots.side = Some(Side::Base);
    snapshots.project = Some("/fixture-project".into());
    let diagnostic = "maven-compile failed\n[ERROR] Cannot find symbol getValues()";
    let logs = "/temporary/failed-analysis/0-1";
    snapshots.accept(preparation_error(
        engine_client::AnalysisFailure {
            code: Some(AnalysisFailureCode::Failed),
            message: diagnostic.into(),
            target: Some(TargetFailure {
                target: BuildIdentity {
                    path: "/temporary/snapshot/checkout".into(),
                    kind: "maven".into(),
                },
                phase: AnalysisPhase::Compilation,
                logs: Some(logs.into()),
            }),
        }
        .into(),
    ));
    let mut runtime = Runtime::new(
        state,
        crate::view::view,
        (800.0, 600.0),
        Env::new(true, 1.0),
    );
    let clipboard = rune::clipboard::memory();
    runtime.use_clipboard(&clipboard);
    runtime.frame(1.0 / 60.0);
    let elements = runtime.elements();
    let detail = elements
        .iter()
        .find(|element| element.key.as_deref() == Some("comparison-snapshot-diagnostic-text"))
        .unwrap();
    let text = detail.text.as_deref().unwrap();
    assert!(text.contains("Compilation failed"));
    assert!(text.contains(diagnostic));
    assert!(text.contains(logs));
    assert!(detail.visible[3] > 0.0);

    let (x, y) = runtime.find("comparison-snapshot-copy-error").unwrap();
    runtime.pointer_down(x, y);
    runtime.frame(1.0 / 60.0);
    runtime.pointer_up(x, y);
    runtime.frame(1.0 / 60.0);
    assert_eq!(clipboard.text().as_deref().map(String::as_str), Some(text));

    runtime.update(|state, _| {
        state
            .flow_comparison
            .snapshots
            .project_installed(Some(Path::new("/other-project")));
    });
    runtime.frame(1.0 / 60.0);
    assert!(runtime.find("comparison-snapshot-copy-error").is_none());
    assert!(runtime.state.flow_comparison.snapshots.message.is_empty());
}
