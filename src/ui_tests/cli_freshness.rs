//! Saved compiler graphs, CLI notifications and actual Rune adoption controls.
use super::{
    cli_backend::{
        analyze, analyze_existing, analyze_with_runtime, configure_typescript_project, desktop,
        search_start, settle,
    },
    click,
    project_explorer::capture,
};
use crate::{
    app::State,
    flow_comparison::tests::{git, isolate_cli_cache},
    freshness_workspace::Status,
    jobs,
};
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
        .find(|element| element.key.as_deref() == Some("project-picker-trigger"))
        .and_then(|element| element.label);
    assert_eq!(
        shown.as_deref(),
        Some(label.as_ref()),
        "Workspace header must name the selected analysis folder"
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
    let mut artifact = codecallnavigator::artifact::load(project).unwrap();
    artifact.graph["analysis"] = serde_json::Value::Null;
    codecallnavigator::artifact::publish(
        project,
        &artifact,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
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
    if runtime.state.ui.overlay != Some(crate::presentation::Overlay::Activity) {
        click(runtime, "activity-details");
        super::settle_revealed_control(runtime);
    }
    assert!(runtime.find("freshness-retry").is_some());
    fs::write(path, bytes).unwrap();
    click(runtime, "freshness-retry");
    wait_observation(runtime);
    assert_eq!(runtime.state.generation, generation);
    runtime.set_viewport((1420.0, 900.0), 1.0);
    runtime.set_system_dark(true);
}

fn deleted_selection_and_manual_retry(runtime: &mut Runtime<State>, project: &Path) {
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
}

fn assert_graphless(state: &State) {
    assert!(
        state.graph.is_none() && state.query_context.is_none() && state.generation.is_none(),
        "The previous graph and its binding must be released"
    );
    assert!(state.graph_view.nodes.is_empty());
    assert!(
        state.back.is_empty() && state.forward.is_empty() && state.nav.selected.is_none(),
        "The previous graph's navigation must be cleared"
    );
}

fn select_folder(runtime: &mut Runtime<State>, project: &Path) {
    runtime.update(|state, _| state.select_recent_project(project.to_string_lossy().into_owned()));
    runtime.frame(1.0 / 60.0);
    assert_workspace_identity(runtime, project);
}

fn switch_to_graphless(runtime: &mut Runtime<State>, graphless: &Path) {
    runtime.update(|state, _| {
        state.select_recent_project(graphless.to_string_lossy().into_owned());
        assert_graphless(state);
    });
    runtime.frame(1.0 / 60.0);
    assert_workspace_identity(runtime, graphless);
    assert!(runtime.find("graph-node-0").is_none());
    settle(runtime, |state| {
        !state.targets.is_empty() && !state.project_open.loading()
    });
    assert_graphless(&runtime.state);
    assert!(
        runtime.state.warning.is_empty(),
        "{}",
        runtime.state.warning
    );
    assert!(
        !graphless
            .join(codecallnavigator::artifact::GRAPH_PATH)
            .exists()
    );
    capture_state(runtime, "graphless-project");
    click(runtime, "project-picker-trigger");
    super::settle_revealed_control(runtime);
    assert!(runtime.elements().iter().any(|element| {
        element.key.as_deref() == Some("analyze")
            && element.label.as_deref() == Some("Generate graph")
            && element.focusable
    }));
    super::dismiss_control_surface(runtime);
}

fn reopen_saved_graph(runtime: &mut Runtime<State>, project: &Path, graphless: &Path) {
    let manifest = project.join(codecallnavigator::artifact::GRAPH_PATH);
    let saved = fs::read(&manifest).unwrap();
    runtime.update(|state, _| {
        for folder in [project, graphless, project] {
            state.select_recent_project(folder.to_string_lossy().into_owned());
            assert_graphless(state);
        }
    });
    runtime.frame(1.0 / 60.0);
    assert_workspace_identity(runtime, project);
    settle(runtime, |state| {
        state.view_is_current()
            && !state.analyzing
            && !state.targets.is_empty()
            && state
                .query_context
                .as_ref()
                .is_some_and(|context| context.descriptor.project == project)
    });
    assert_eq!(fs::read(manifest).unwrap(), saved);
}

fn same_folder_analysis_preserves_graph(runtime: &mut Runtime<State>, project: &Path) {
    let graph = runtime.state.graph.clone().unwrap();
    let generation = runtime.state.generation.clone();
    let assert_previous_graph = |state: &State| {
        assert!(Arc::ptr_eq(state.graph.as_ref().unwrap(), &graph));
        assert_eq!(state.generation, generation);
        assert!(state.view_is_current() && !state.graph_view.nodes.is_empty());
    };
    select_folder(runtime, project);
    assert_previous_graph(&runtime.state);
    settle(runtime, |state| {
        !state.targets.is_empty() && !state.project_job_busy()
    });

    fs::write(project.join("example.ts"), "export function broken( {\n").unwrap();
    runtime.update(|state, _| {
        state.analyze_selected();
        assert!(state.analyzing);
        assert_previous_graph(state);
    });
    settle(runtime, |state| !state.analyzing);
    assert!(runtime.state.feedback().is_some());
    assert_previous_graph(&runtime.state);

    source(project, 4, true);
    runtime.update(|state, _| {
        state.analyze_selected();
        assert!(state.analyzing);
        state.cancel_analysis();
        assert_previous_graph(state);
    });
    settle(runtime, |state| !state.analyzing);
    assert!(runtime.state.warning.to_lowercase().contains("cancel"));
    assert_previous_graph(&runtime.state);
    assert_workspace_identity(runtime, project);
}

fn close_project(runtime: &mut Runtime<State>) {
    runtime.update(jobs::close);
    settle(runtime, |state| {
        state.freshness.stopped()
            && state.cache_retention.stopped()
            && !state.jobs.persistence.is_pending()
    });
}

fn restart_graphless(cache: PathBuf, project: &Path) {
    let state = State::new();
    state.jobs.boot.request(cache, |cache| {
        jobs::load_boot(cache).map_err(|error| error.to_string())
    });
    let mut runtime = Runtime::new(
        state,
        crate::view::view,
        (1420.0, 900.0),
        rune::Env::new(true, 1.0),
    );
    runtime.poll = Some(jobs::poll);
    settle(&mut runtime, |state| {
        state.store.is_some() && !state.targets.is_empty() && !state.project_open.loading()
    });
    assert_workspace_identity(&runtime, project);
    assert_graphless(&runtime.state);
    assert!(
        runtime.state.warning.is_empty(),
        "{}",
        runtime.state.warning
    );
    close_project(&mut runtime);
}

#[test]
#[ignore = "requires built CLI, Node24/TypeScript5.9-or-6.0; optional captures require Metal"]
fn actual_cli_freshness_reload_preserves_loaded_binding_camera_and_discards_obsolete_jobs() {
    let directory = tempfile::tempdir().unwrap();
    let project = directory.path().canonicalize().unwrap();
    let other_directory = tempfile::tempdir().unwrap();
    let other = other_directory.path().canonicalize().unwrap();
    fs::write(other.join("tsconfig.json"), r#"{"files":["example.ts"]}"#).unwrap();
    source(&other, 1, false);
    let cache = tempfile::tempdir().unwrap();
    let mut runtime = desktop(analyze(&project), &project);
    runtime.state.project_open.folder = Some(project.to_string_lossy().into_owned());
    assert_workspace_identity(&runtime, &project);
    runtime.update(|state, _| {
        state.root = rune::TextEdit::new(other.to_string_lossy());
        state.root_edited();
    });
    runtime.frame(1.0 / 60.0);
    assert_workspace_identity(&runtime, &project);
    runtime.state.store = Some(Arc::new(
        codecallnavigator::store::SqliteStore::open(cache.path()).unwrap(),
    ));
    let symbol = search_start(&mut runtime);
    wait_observation(&mut runtime);
    preserve_context_across_publication(&mut runtime, &project, &symbol);
    legacy_saved_graph_is_unverifiable(&mut runtime, &project);
    deleted_selection_and_manual_retry(&mut runtime, &project);
    switch_to_graphless(&mut runtime, &other);
    reopen_saved_graph(&mut runtime, &project, &other);
    same_folder_analysis_preserves_graph(&mut runtime, &project);
    switch_to_graphless(&mut runtime, &other);
    close_project(&mut runtime);
    restart_graphless(cache.path().into(), &other);
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
    super::click(&mut runtime, "activity-details");
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
#[ignore = "requires built CLI with automatic refresh, Node24/TypeScript5.9-or-6.0; optional captures require Metal"]
fn actual_cli_automatic_refresh_requires_opt_in_and_preserves_loaded_context() {
    let directory = tempfile::tempdir().unwrap();
    let fixture = directory.path().canonicalize().unwrap();
    let project = fixture.join("checkout");
    fs::create_dir(&project).unwrap();
    configure_typescript_project(&project);
    git(&project, &["init", "-q"]);
    git(&project, &["config", "user.name", "Automatic refresh test"]);
    git(
        &project,
        &["config", "user.email", "refresh@example.invalid"],
    );
    git(&project, &["add", "tsconfig.json", "example.ts"]);
    git(&project, &["commit", "-qm", "Baseline"]);
    let mut engine = crate::runtime::Runtime::discover().unwrap();
    isolate_cli_cache(&mut engine, &fixture.join("snapshot-cache"));
    let original = analyze_with_runtime(&project, &engine)
        .context
        .descriptor
        .generation;
    source(&project, 2, true);
    git(&project, &["add", "example.ts"]);
    git(&project, &["commit", "-qm", "Update source"]);

    let mut runtime = open_project(&project, engine);
    settle(&mut runtime, |state| {
        state.freshness.is_stale() && state.view_is_current() && !state.jobs.discovery.is_pending()
    });
    assert_eq!(
        runtime.state.generation, original,
        "Passive observation must not analyze or adopt"
    );
    runtime.update(|state, _| {
        state.nav.camera.pan = [71.0, -29.0];
        state.nav.camera.zoom = 0.73;
    });
    click(&mut runtime, "freshness-auto-toggle");
    assert_typescript_progress(&mut runtime);
    assert_eq!(runtime.state.generation, original);
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

/// Risk: graphless opt-in, mixed source roots and first adoption cross the real
/// desktop, observer, retained Node compiler and saved-generation boundaries.
#[test]
#[ignore = "requires built CLI with automatic refresh, Node24/TypeScript5.9-or-6.0"]
fn actual_cli_automatic_refresh_adopts_first_graph_and_keeps_inferred_roots() {
    use codecallnavigator::refresh::Phase;

    let fixture = tempfile::tempdir().unwrap();
    let (project, shared) = fresh_project_with_linked_source(fixture.path());
    let mut ui = desktop_without_graph(&project);
    settle(&mut ui, |state| {
        !state.project_open.loading() && state.targets.iter().any(|choice| choice.selected)
    });
    click(&mut ui, "freshness-auto-toggle");
    assert_typescript_progress(&mut ui);
    assert!(ui.state.graph.is_none());
    settle(&mut ui, |state| {
        state.freshness.automatic.activity.as_ref().is_some_and(|activity| {
            matches!(&activity.phase, Phase::Failed { error, .. } if error.code == "failed")
        })
    });
    assert!(ui.state.graph.is_none());
    assert!(ui.state.freshness.last.is_none());
    assert!(
        !project
            .join(codecallnavigator::artifact::GRAPH_PATH)
            .exists()
    );

    fs::write(
        project.join("main.ts"),
        "import { leaf } from '../shared/leaf'; export function main() { return leaf(); }",
    )
    .unwrap();
    settle(&mut ui, |state| {
        state.graph.is_some() && state.view_is_current()
    });
    let context = ui.state.query_context.as_ref().unwrap();
    assert_eq!(context.descriptor.project, project);
    assert!(
        context
            .descriptor
            .roots
            .iter()
            .any(|(_, root)| shared.starts_with(root))
    );
    let graph = ui.state.graph.as_ref().unwrap();
    assert!(graph.data.nodes.iter().any(|node| node.label == "leaf"));
    assert!(ui.state.automatic_refresh_enabled());
    let first = ui.state.generation.clone();
    fs::write(
        shared.join("leaf.ts"),
        "export function leaf() { return 'fresh'; }",
    )
    .unwrap();
    settle(&mut ui, |state| {
        state.generation != first && state.view_is_current()
    });
    assert!(
        ui.state
            .freshness
            .automatic
            .activity
            .as_ref()
            .unwrap()
            .runs_started
            >= 3
    );
    assert_opt_out_stops_analysis(&mut ui, &shared);
    ui.update(jobs::close);
    settle(&mut ui, |state| state.freshness.stopped());
    assert!(ui.state.freshness.cleanup_warning.is_none());
}

/// Only the real compiler-to-desktop journey proves target attribution and
/// that the recovery control can publish a graph the CLI search index serves.
#[test]
#[ignore = "requires built CLI, Maven/Java and Node/TypeScript"]
fn actual_cli_mixed_analysis_failure_can_retry_java_only() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().canonicalize().unwrap();
    let project = mixed_failure_project(&root);
    let mut ui = desktop_without_graph(&project);
    settle(&mut ui, |state| {
        !state.project_open.loading()
            && state
                .targets
                .iter()
                .filter(|choice| choice.selected)
                .count()
                == 2
    });
    ui.update(|state, _| state.analyze_selected());
    settle(&mut ui, |state| state.analysis_failure.is_some());
    let manual = assert_typescript_failure(&ui, &project);
    assert_visible_failure(&mut ui);
    ui.update(|state, _| assert_retry_scope(state));

    // The recovery button is the interaction under test. Start the real
    // automatic session through the same action used by its toolbar control.
    ui.update(|state, _| state.toggle_automatic_refresh());
    assert!(
        ui.state.automatic_refresh_enabled(),
        "automatic session did not start"
    );
    settle(&mut ui, |state| {
        state.analysis_failure.is_none() && state.feedback().is_some()
    });
    assert_eq!(assert_typescript_failure(&ui, &project), manual);
    click(&mut ui, "retry-remaining-builds");
    assert!(!ui.state.automatic_refresh_enabled());
    assert!(
        ui.state
            .selected_analysis()
            .unwrap()
            .targets
            .iter()
            .all(|target| target.language == "java")
    );
    settle(&mut ui, |state| {
        (state.graph.is_some() && state.view_is_current()) || state.analysis_failure.is_some()
    });
    assert!(
        ui.state.graph.is_some(),
        "Java-only retry failed: {:?}",
        ui.state.feedback().map(|failure| failure.diagnostic)
    );
    assert!(ui.state.feedback().is_none());

    click(&mut ui, "open-search");
    ui.update(|state, _| {
        state.search.input = rune::TextEdit::new("Standalone");
        state.request_search();
    });
    settle(&mut ui, |state| !state.search.hits().is_empty());
    assert!(
        ui.state
            .search
            .hits()
            .iter()
            .any(|hit| hit.name.contains("Standalone"))
    );
    ui.update(jobs::close);
    settle(&mut ui, |state| state.freshness.stopped());
    assert!(ui.state.freshness.cleanup_warning.is_none());
}

fn mixed_failure_project(root: &Path) -> PathBuf {
    let project = root.join("standalone");
    let java_source = project.join("src/main/java/fixture");
    let ui = project.join("ui");
    for dir in [&java_source, &ui, &root.join("parent")] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(
        root.join("parent/pom.xml"),
        include_str!("../../fixtures/maven/parent/pom.xml"),
    )
    .unwrap();
    fs::write(
        project.join("pom.xml"),
        include_str!("../../fixtures/maven/standalone/pom.xml"),
    )
    .unwrap();
    fs::write(
        java_source.join("Standalone.java"),
        include_str!("../../fixtures/maven/standalone/src/main/java/fixture/Standalone.java"),
    )
    .unwrap();
    fs::write(
        ui.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"types":[]},"files":["broken.ts"]}"#,
    )
    .unwrap();
    let broken: String = (0..100)
        .map(|index| format!("export const broken{index}: number = 'text';\n"))
        .collect();
    fs::write(ui.join("broken.ts"), broken).unwrap();
    let modules =
        std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES").expect("Installed TypeScript");
    std::os::unix::fs::symlink(modules, ui.join("node_modules")).unwrap();
    project
}

fn assert_typescript_failure(ui: &Runtime<State>, project: &Path) -> String {
    let feedback = ui.state.feedback().expect("visible analysis failure");
    let target = feedback.target.expect("identified failing target");
    assert_eq!(target.target.path, project.join("ui/tsconfig.json"));
    assert!(
        feedback.diagnostic.contains("TS2322"),
        "{}",
        feedback.diagnostic
    );
    assert!(feedback.diagnostic.ends_with("[truncated]"));
    assert!(feedback.summary().contains("No graph was created."));
    assert!(ui.find("retry-remaining-builds").is_some());
    assert!(ui.state.graph.is_none());
    assert!(
        !project
            .join(codecallnavigator::artifact::GRAPH_PATH)
            .exists()
    );
    feedback.summary()
}

fn assert_visible_failure(ui: &mut Runtime<State>) {
    ui.update(|state, _| state.ui.sidebar.set_open(true));
    for viewport in [(720.0, 680.0), (1960.0, 900.0)] {
        ui.set_viewport(viewport, 1.0);
        for _ in 0..60 {
            ui.frame(1.0 / 60.0);
        }
        let elements = ui.elements();
        for key in ["retry-remaining-builds", "failed-analysis-details"] {
            assert_recovery_control_visible(&elements, key, viewport);
        }
    }
    click(ui, "failed-analysis-details");
    assert!(ui.find("analysis-failure-diagnostic").is_some());
    ui.update(|state, _| state.close_overlay());
    ui.set_viewport((1420.0, 900.0), 1.0);
    for _ in 0..60 {
        ui.frame(1.0 / 60.0);
    }
}

fn assert_recovery_control_visible(
    elements: &[rune::ElementInfo],
    key: &str,
    viewport: (f32, f32),
) {
    let action = elements
        .iter()
        .find(|element| element.key.as_deref() == Some(key) && !element.inert && !element.exiting)
        .unwrap_or_else(|| panic!("Missing {key}"));
    assert!(
        action.visible[2] >= action.rect[2] - 1.0 && action.visible[3] >= action.rect[3] - 1.0,
        "{key} clipped at {viewport:?}"
    );
}

fn assert_retry_scope(state: &mut State) {
    let retry = state.retry_analysis().expect("mixed selection can recover");
    let before = state.selected_analysis().unwrap().targets;
    let folder = state.project_open.folder.replace("/other-project".into());
    state.retry_without_build(&retry.target);
    assert_eq!(state.selected_analysis().unwrap().targets, before);
    state.project_open.folder = folder;

    let maven = state
        .targets
        .iter()
        .position(|choice| choice.target.kind == "maven")
        .unwrap();
    state.targets[maven].selected = false;
    state.retry_without_build(&retry.target);
    assert_eq!(state.selected_analysis().unwrap().targets.len(), 1);
    assert!(state.retry_analysis().is_none());
    state.targets[maven].selected = true;
}

fn assert_opt_out_stops_analysis(ui: &mut Runtime<State>, shared: &Path) {
    click(ui, "freshness-auto-toggle");
    assert!(!ui.state.automatic_refresh_enabled());
    wait_observation(ui);
    let last = ui.state.generation.clone();
    fs::write(
        shared.join("leaf.ts"),
        "export function leaf() { return 17; }",
    )
    .unwrap();
    settle(ui, |state| state.freshness.is_stale());
    assert_eq!(ui.state.generation, last);
}

fn fresh_project_with_linked_source(base: &Path) -> (PathBuf, PathBuf) {
    let root = base.canonicalize().unwrap();
    let project = root.join("checkout");
    let shared = root.join("shared");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&shared).unwrap();
    fs::write(
        project.join("tsconfig.json"),
        r#"{"compilerOptions":{"strict":true,"target":"ES2022","rootDir":"..","types":[]},"files":["main.ts","../shared/leaf.ts"]}"#,
    )
    .unwrap();
    fs::write(project.join("main.ts"), "export function broken( {").unwrap();
    fs::write(
        shared.join("leaf.ts"),
        "export function leaf() { return 1; }",
    )
    .unwrap();
    let modules =
        std::env::var_os("CODECALLNAVIGATOR_TS_NODE_MODULES").expect("Installed TypeScript");
    std::os::unix::fs::symlink(modules, project.join("node_modules")).unwrap();
    (project, shared)
}

fn desktop_without_graph(project: &Path) -> Runtime<State> {
    open_project(project, crate::runtime::Runtime::discover().unwrap())
}

fn open_project(project: &Path, engine: crate::runtime::Runtime) -> Runtime<State> {
    let mut state = State::new();
    state.runtime = Some(Arc::new(engine));
    state.open_project(project.to_string_lossy().into_owned());
    let mut runtime = Runtime::new(
        state,
        crate::view::view,
        (1420.0, 900.0),
        rune::Env::new(true, 1.0),
    );
    runtime.poll = Some(jobs::poll);
    runtime
}

fn assert_typescript_progress(runtime: &mut Runtime<State>) {
    use codecallnavigator::refresh::Phase;
    let stage = |state: &State| {
        state
            .freshness
            .automatic
            .activity
            .as_ref()
            .is_some_and(|activity| {
                matches!(&activity.phase, Phase::Running { progress_message: Some(message), .. }
                if message.starts_with("Preparing TypeScript project"))
            })
    };
    settle(runtime, stage);
    let elements = runtime.elements();
    for key in ["status-message", "active-analysis"] {
        assert!(
            elements.iter().any(|element| {
                element.key.as_deref() == Some(key)
                    && !element.exiting
                    && element
                        .text
                        .as_deref()
                        .is_some_and(|message| message.starts_with("Preparing TypeScript project"))
            }),
            "{key} did not render the live TypeScript preparation stage"
        );
    }
    assert!(elements.iter().any(|element| {
        element.key.as_deref() == Some("analysis-stage-elapsed")
            && !element.exiting
            && element
                .text
                .as_deref()
                .is_some_and(|value| value.starts_with("Stage shown for "))
    }));
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
