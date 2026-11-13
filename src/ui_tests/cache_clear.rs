use super::*;
use std::sync::Arc;

fn cache_runtime(project: &std::path::Path) -> Runtime<State> {
    let mut runtime = runtime();
    runtime.state.root = rune::TextEdit::new(project.to_string_lossy());
    runtime.state.runtime = Some(Arc::new(crate::runtime::Runtime::discover().unwrap()));
    runtime.frame(1.0 / 60.0);
    runtime
}

#[test]
fn clear_cache_control_is_adjacent_and_disabled_during_analysis() {
    let project = tempfile::tempdir().unwrap();
    let mut runtime = cache_runtime(project.path());
    let clear = runtime.find("clear-cache").unwrap();
    let analyze = runtime.find("analyze").unwrap();
    assert!(clear.0 < analyze.0);
    assert!((clear.1 - analyze.1).abs() < 2.0);
    runtime.state.analyzing = true;
    runtime.frame(1.0 / 60.0);
    click(&mut runtime, "clear-cache");
    assert!(!runtime.state.clearing_cache);
    assert!(!runtime.state.jobs.cache_clear.is_pending());
}

#[test]
fn clearing_blocks_analysis_even_when_called_directly() {
    let project = tempfile::tempdir().unwrap();
    let mut runtime = cache_runtime(project.path());
    let state = &mut runtime.state;
    state.store = Some(Arc::new(
        codecallnavigator::store::SqliteStore::open(project.path()).unwrap(),
    ));
    state.targets.push(crate::app::TargetChoice {
        selected: true,
        target: crate::discovery::BuildTarget {
            path: project.path().join("tsconfig.json"),
            language: "typescript".into(),
            kind: "typescript".into(),
            label: "project".into(),
        },
    });
    state.clearing_cache = true;
    state.analyze_selected();
    assert!(!state.analyzing);
    assert!(state.warning.is_empty());
    assert!(!state.jobs.analysis.is_pending());
    assert!(!state.can_clear_cache());
    runtime.frame(1.0 / 60.0);
    click(&mut runtime, "analyze");
    click(&mut runtime, "clear-cache");
    assert!(!runtime.state.analyzing);
    assert!(!runtime.state.jobs.analysis.is_pending());
    assert!(!runtime.state.jobs.cache_clear.is_pending());
}

#[test]
#[ignore = "requires built standalone engine"]
fn clear_cache_button_uses_real_engine_preserves_graph_and_allows_retry() {
    let project = tempfile::tempdir().unwrap();
    let cache_dir = project.path().join(".codecallnavigator");
    std::fs::create_dir(&cache_dir).unwrap();
    let cache = cache_dir.join("inference.sqlite3");
    std::fs::write(&cache, b"disposable cache").unwrap();
    let mut runtime = cache_runtime(project.path());
    let graph = runtime.state.graph.clone().unwrap();
    let nav = runtime.state.session();
    runtime
        .state
        .freshness
        .accept(crate::freshness_workspace::fixture(
            codecallnavigator::artifact::verification::InputStatus::Stale,
            runtime.state.generation.as_deref().unwrap_or("fixture"),
        ));
    click(&mut runtime, "clear-cache");
    wait_for(&mut runtime, |state| !state.clearing_cache);
    assert!(!cache.exists());
    assert!(Arc::ptr_eq(runtime.state.graph.as_ref().unwrap(), &graph));
    assert_eq!(runtime.state.session().root, nav.root);
    assert_eq!(runtime.state.status, crate::cache_jobs::CLEARED);
    assert!(
        runtime
            .elements()
            .iter()
            .any(|element| { element.text.as_deref() == Some(crate::cache_jobs::CLEARED) })
    );
    assert!(runtime.state.can_clear_cache());

    let lock = codecallnavigator::artifact::AnalysisLock::acquire(project.path()).unwrap();
    click(&mut runtime, "clear-cache");
    wait_for(&mut runtime, |state| !state.clearing_cache);
    assert!(runtime.state.warning.contains("Another analysis"));
    assert!(runtime.state.can_clear_cache());
    drop(lock);
    click(&mut runtime, "clear-cache");
    wait_for(&mut runtime, |state| !state.clearing_cache);
    assert!(runtime.state.warning.is_empty());
    assert_eq!(runtime.state.status, crate::cache_jobs::CLEARED);
}
