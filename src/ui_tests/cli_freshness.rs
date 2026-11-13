//! Saved compiler graphs, CLI notifications and actual Rune adoption controls.
use super::{
    cli_backend::{analyze, analyze_existing, desktop, search_start, settle},
    click,
    project_explorer::capture,
};
use crate::{app::State, freshness_workspace::Status, jobs};
use codecallnavigator::artifact::verification::InputStatus;
use rune::Runtime;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

fn capture_state(runtime: &mut Runtime<State>, name: &str) {
    runtime.pointer_move(5.0, 5.0);
    if let Some(output) = std::env::var_os("CODECALLNAVIGATOR_FRESHNESS_UI_CAPTURES") {
        for _ in 0..20 {
            runtime.frame(1.0 / 60.0);
        }
        capture(
            runtime,
            (1420.0, 900.0),
            &PathBuf::from(output).join(format!("{name}.png")),
        );
    }
}

fn assert_workspace_identity(runtime: &Runtime<State>, expected: &Path) {
    let label = expected.file_name().unwrap().to_string_lossy();
    let shown = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("workspace-identity"))
        .and_then(|element| element.text);
    assert_eq!(
        shown.as_deref(),
        Some(label.as_ref()),
        "Workspace header must name the displayed graph's binding"
    );
}

fn source(project: &Path, value: u32, keep_start: bool) {
    let start = if keep_start {
        "export function start(flag: boolean): number { if(flag) return leaf(); return 0; }"
    } else {
        ""
    };
    fs::write(
        project.join("example.ts"),
        format!("export function leaf(): number {{ return {value}; }}\n{start}\n"),
    )
    .unwrap();
}

fn wait_observation(runtime: &mut Runtime<State>) {
    settle(runtime, |state| {
        matches!(state.freshness.status, Status::Observed(_))
    });
}

struct DelayedLayout(Arc<AtomicBool>);
impl Drop for DelayedLayout {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

fn delay_old_layout(runtime: &mut Runtime<State>) -> DelayedLayout {
    let release = Arc::new(AtomicBool::new(false));
    let worker = release.clone();
    let (epoch, revision) = (runtime.state.epoch, runtime.state.revision);
    runtime.state.jobs.layout.request((), move |_| {
        while !worker.load(Ordering::Acquire) {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        (
            epoch,
            revision,
            Ok(crate::projection::GraphView {
                message: "OBSOLETE layout".into(),
                ..Default::default()
            }),
        )
    });
    DelayedLayout(release)
}

fn reload(runtime: &mut Runtime<State>, previous: u64) {
    settle(runtime, |state| {
        state
            .freshness
            .new_generation(state.generation.as_deref())
            .is_some()
    });
    click(runtime, "freshness-reload");
    settle(runtime, |state| state.epoch != previous);
}

fn preserve_context_across_publication(runtime: &mut Runtime<State>, project: &Path, symbol: &str) {
    runtime.update(|state, _| {
        state.nav.camera.pan = [91.0, -47.0];
        state.nav.camera.zoom = 0.67;
    });
    let history = runtime.state.back.len();
    let epoch = runtime.state.epoch;
    source(project, 2, true);
    settle(runtime, |state| {
        matches!(state.freshness.status, Status::Observed(InputStatus::Stale))
    });
    capture_state(runtime, "stale");
    drop(analyze_existing(project));
    settle(runtime, |state| {
        state
            .freshness
            .new_generation(state.generation.as_deref())
            .is_some()
    });
    capture_state(runtime, "reload-offered");
    let delayed = delay_old_layout(runtime);
    reload(runtime, epoch);
    drop(delayed);
    settle(runtime, |state| state.view_is_current());
    assert_eq!(runtime.state.nav.selected.as_deref(), Some(symbol));
    assert_eq!(runtime.state.back.len(), history);
    assert_eq!(runtime.state.nav.camera.pan, [91.0, -47.0]);
    assert_eq!(runtime.state.nav.camera.zoom, 0.67);
    assert!(!runtime.state.graph_view.message.contains("OBSOLETE"));
    assert_workspace_identity(runtime, project);
    assert_eq!(
        runtime
            .state
            .query_context
            .as_ref()
            .unwrap()
            .descriptor
            .project,
        project
    );
    capture_state(runtime, "adopted");
}

fn legacy_saved_graph_is_unverifiable(runtime: &mut Runtime<State>, project: &Path) {
    let path = project.join(codecallnavigator::artifact::GRAPH_PATH);
    let mut artifact: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    artifact["graph"]["analysis"] = serde_json::Value::Null;
    fs::write(path, serde_json::to_vec(&artifact).unwrap()).unwrap();
    let epoch = runtime.state.epoch;
    reload(runtime, epoch);
    settle(runtime, |state| {
        state.view_is_current()
            && matches!(
                state.freshness.status,
                Status::Observed(InputStatus::Unverifiable)
            )
    });
    capture_state(runtime, "unverifiable");
    assert_eq!(runtime.state.nav.camera.pan, [91.0, -47.0]);
    assert!(runtime.elements().iter().any(|element| {
        element
            .text
            .as_deref()
            .is_some_and(|text| text.contains("unverifiable"))
    }));
}

fn compact_failure_and_manual_retry(runtime: &mut Runtime<State>, project: &Path) {
    let path = project.join(codecallnavigator::artifact::GRAPH_PATH);
    let bytes = fs::read(&path).unwrap();
    let generation = runtime.state.generation.clone();
    fs::remove_file(&path).unwrap();
    settle(runtime, |state| {
        matches!(state.freshness.status, Status::Failed(_))
    });
    for _ in 0..20 {
        runtime.frame(0.05);
    }
    assert!(
        matches!(runtime.state.freshness.status, Status::Failed(_)),
        "Terminal monitor failures require manual retry"
    );
    assert_eq!(runtime.state.generation, generation);
    runtime.set_system_dark(false);
    runtime.set_viewport((1050.0, 680.0), 1.0);
    runtime.update(|state, _| state.fit_graph());
    if let Some(output) = std::env::var_os("CODECALLNAVIGATOR_FRESHNESS_UI_CAPTURES") {
        for _ in 0..20 {
            runtime.frame(1.0 / 60.0);
        }
        capture(
            runtime,
            (1050.0, 680.0),
            &PathBuf::from(output).join("failed-light-compact.png"),
        );
    }
    assert!(runtime.find("freshness-retry").is_some());
    fs::write(path, bytes).unwrap();
    click(runtime, "freshness-retry");
    wait_observation(runtime);
    assert_eq!(runtime.state.generation, generation);
    runtime.set_viewport((1420.0, 900.0), 1.0);
    runtime.set_system_dark(true);
}

fn deleted_selection_and_project_switch(
    runtime: &mut Runtime<State>,
    project: &Path,
    other: &Path,
) {
    source(project, 3, false);
    drop(analyze_existing(project));
    let epoch = runtime.state.epoch;
    reload(runtime, epoch);
    settle(runtime, |state| state.view_is_current());
    assert!(runtime.state.nav.selected.is_none());
    assert!(runtime.state.nav.focus_target.is_none());
    assert_eq!(runtime.state.nav.camera.pan, [91.0, -47.0]);
    capture_state(runtime, "deleted-selection");
    runtime.update(|state, _| state.fit_graph());
    capture_state(runtime, "fitted-view");
    compact_failure_and_manual_retry(runtime, project);
    drop(analyze(other));
    runtime.update(|state, _| state.discover_root());
    settle(runtime, |state| !state.targets.is_empty());
    click(runtime, "analyze");
    settle(runtime, |state| {
        !state.analyzing
            && state
                .query_context
                .as_ref()
                .is_some_and(|context| context.descriptor.project == other)
    });
    assert!(runtime.state.back.is_empty());
    assert!(runtime.state.forward.is_empty());
    assert!(runtime.state.nav.selected.is_none());
    assert_workspace_identity(runtime, other);
}

#[test]
#[ignore = "requires built CLI, Node24/TypeScript6; optional captures require Metal"]
fn actual_cli_freshness_reload_preserves_loaded_binding_camera_and_discards_obsolete_jobs() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().canonicalize().unwrap();
    let other_directory = tempfile::tempdir().unwrap();
    let other = other_directory.path().canonicalize().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let mut runtime = desktop(analyze(&project), &project);
    assert_workspace_identity(&runtime, &project);
    runtime.update(|state, _| {
        state.root = rune::TextEdit::new(other.to_string_lossy());
        state.root_edited();
    });
    assert_workspace_identity(&runtime, &project);
    runtime.state.store = Some(Arc::new(
        codecallnavigator::store::SqliteStore::open(cache.path()).unwrap(),
    ));
    let symbol = search_start(&mut runtime);
    wait_observation(&mut runtime);
    preserve_context_across_publication(&mut runtime, &project, &symbol);
    legacy_saved_graph_is_unverifiable(&mut runtime, &project);
    deleted_selection_and_project_switch(&mut runtime, &project, &other);
    runtime.update(jobs::close);
    settle(&mut runtime, |state| {
        state.freshness.stopped()
            && state.cache_retention.stopped()
            && !state.jobs.persistence.is_pending()
    });
}

#[test]
fn automatic_refresh_is_an_explicit_loaded_project_action() {
    let mut state = State::fixture();
    state.freshness.accept(crate::freshness_workspace::fixture(
        InputStatus::Match,
        &"a".repeat(64),
    ));
    let mut runtime = Runtime::new(
        state,
        crate::view::view,
        (1420.0, 900.0),
        rune::Env::new(true, 1.0),
    );
    runtime.frame(1.0 / 60.0);
    let action = runtime
        .elements()
        .into_iter()
        .find(|element| element.key.as_deref() == Some("freshness-auto-toggle"));
    assert!(
        action.is_some(),
        "Saved-input status needs an explicit automatic-refresh opt-in action"
    );
}

#[test]
#[ignore = "requires built CLI with automatic refresh, Node24/TypeScript6; optional captures require Metal"]
fn actual_cli_automatic_refresh_requires_opt_in_and_preserves_loaded_context() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().canonicalize().unwrap();
    let pending = tempfile::tempdir().unwrap();
    let snapshot = analyze(&project);
    let original = snapshot.context.descriptor.generation.clone();
    let mut runtime = desktop(snapshot, pending.path());
    wait_observation(&mut runtime);
    source(&project, 2, true);
    settle(&mut runtime, |state| state.freshness.is_stale());
    assert_eq!(
        runtime.state.generation, original,
        "Passive observation must not analyze or adopt"
    );
    runtime.update(|state, _| {
        state.nav.camera.pan = [71.0, -29.0];
        state.nav.camera.zoom = 0.73;
    });
    click(&mut runtime, "freshness-auto-toggle");
    settle(&mut runtime, |state| {
        state.generation != original && state.view_is_current()
    });
    assert_eq!(
        runtime
            .state
            .query_context
            .as_ref()
            .unwrap()
            .descriptor
            .project,
        project
    );
    assert!(runtime.state.freshness.automatic.enabled(&project));
    assert_eq!(runtime.state.nav.camera.pan, [71.0, -29.0]);
    assert_eq!(runtime.state.nav.camera.zoom, 0.73);
    capture_state(&mut runtime, "automatic-enabled");
    invalid_automatic_code_recovers(&mut runtime, &project);
    manual_analysis_waits_for_automatic_cleanup(&mut runtime, &project);
    click(&mut runtime, "freshness-auto-toggle");
    settle(&mut runtime, |state| {
        !state.freshness.automatic.enabled(&project)
    });
    let disabled = runtime.state.generation.clone();
    source(&project, 4, true);
    settle(&mut runtime, |state| state.freshness.is_stale());
    assert_eq!(runtime.state.generation, disabled);
    capture_state(&mut runtime, "automatic-disabled");
    runtime.update(jobs::close);
    settle(&mut runtime, |state| state.freshness.stopped());
}

fn invalid_automatic_code_recovers(runtime: &mut Runtime<State>, project: &Path) {
    use codecallnavigator::refresh::Phase;
    let successful = runtime.state.generation.clone();
    fs::write(project.join("example.ts"), "export function broken( {\n").unwrap();
    settle(runtime, |state| {
        state
            .freshness
            .automatic
            .activity
            .as_ref()
            .is_some_and(|activity| matches!(activity.phase, Phase::Failed { .. }))
    });
    assert_eq!(
        runtime.state.generation, successful,
        "Invalid code must preserve the successful graph"
    );
    assert!(
        runtime.state.freshness.is_stale(),
        "Activity must not replace input validity"
    );
    capture_state(runtime, "automatic-invalid-code");
    capture_automatic_compact_failure(runtime);
    source(project, 3, true);
    settle(runtime, |state| {
        state.generation != successful && state.view_is_current()
    });
}

fn manual_analysis_waits_for_automatic_cleanup(runtime: &mut Runtime<State>, project: &Path) {
    use codecallnavigator::refresh::Phase;
    let engine = crate::runtime::Runtime::discover().unwrap();
    let targets = crate::engine_client::discover(&engine, project).unwrap();
    runtime.update(|state, _| {
        state.root = rune::TextEdit::new(project.to_string_lossy());
        state.targets = targets
            .into_iter()
            .map(|target| crate::app::TargetChoice {
                target,
                selected: true,
            })
            .collect();
    });
    source(project, 5, true);
    settle(runtime, |state| {
        state
            .freshness
            .automatic
            .activity
            .as_ref()
            .is_some_and(|activity| matches!(activity.phase, Phase::Running { .. }))
    });
    runtime.update(|state, _| state.analyze_selected());
    assert!(runtime.state.freshness.manual_pending());
    settle(runtime, |state| state.analyzing);
    settle(runtime, |state| !state.analyzing && state.view_is_current());
    assert!(
        runtime.state.warning.is_empty(),
        "{}",
        runtime.state.warning
    );
    assert!(runtime.state.freshness.automatic.enabled(project));
}

fn capture_automatic_compact_failure(runtime: &mut Runtime<State>) {
    let Some(output) = std::env::var_os("CODECALLNAVIGATOR_FRESHNESS_UI_CAPTURES") else {
        return;
    };
    runtime.set_system_dark(false);
    runtime.set_viewport((1050.0, 680.0), 1.0);
    for _ in 0..20 {
        runtime.frame(1.0 / 60.0);
    }
    capture(
        runtime,
        (1050.0, 680.0),
        &PathBuf::from(output).join("automatic-invalid-light-compact.png"),
    );
    runtime.set_viewport((1420.0, 900.0), 1.0);
    runtime.set_system_dark(true);
}
